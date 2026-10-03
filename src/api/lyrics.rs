use crate::error::AppError;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SyncedLyricLine {
    pub timestamp_ms: u32,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LyricsData {
    pub track_name: String,
    pub artist_name: String,
    pub lines: Vec<SyncedLyricLine>,
    /// `false` when only plain (untimed) lyrics exist; timestamps are then meaningless.
    #[serde(default)]
    pub synced: bool,
}

/// LRCLIB answers in camelCase.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct LrcLibRecord {
    #[serde(default)]
    duration: f64,
    #[serde(default)]
    instrumental: bool,
    synced_lyrics: Option<String>,
    plain_lyrics: Option<String>,
}

impl LrcLibRecord {
    fn has_synced(&self) -> bool {
        self.synced_lyrics
            .as_deref()
            .is_some_and(|l| !l.trim().is_empty())
    }

    fn has_plain(&self) -> bool {
        self.plain_lyrics
            .as_deref()
            .is_some_and(|l| !l.trim().is_empty())
    }
}

const LRCLIB_USER_AGENT: &str = "Spotifust/0.1.0 (https://github.com/gefydev/spotifust)";

/// Returns the first credited artist ("A, B" / "A & B" / "A feat. B" -> "A").
#[must_use]
pub fn primary_artist(artist: &str) -> &str {
    let mut end = artist.len();
    for sep in [", ", " & ", " feat. ", " ft. ", " x ", "; "] {
        if let Some(idx) = artist.find(sep) {
            end = end.min(idx);
        }
    }
    artist[..end].trim()
}

/// Drops decorations Spotify adds to titles that LRCLIB rarely has
/// ("Song - Remastered 2011", "Song (feat. X)").
fn clean_title(title: &str) -> &str {
    let mut end = title.len();
    for marker in [" - ", " (feat", " (with", " [feat"] {
        if let Some(idx) = title.find(marker) {
            if idx > 0 {
                end = end.min(idx);
            }
        }
    }
    title[..end].trim()
}

fn lrclib_client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent(LRCLIB_USER_AGENT)
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap_or_default()
}

async fn lrclib_get(
    client: &reqwest::Client,
    track: &str,
    artist: &str,
    album: &str,
    duration_secs: u32,
) -> Option<LrcLibRecord> {
    let duration = duration_secs.to_string();
    let mut params = vec![("track_name", track), ("artist_name", artist)];
    if !album.is_empty() {
        params.push(("album_name", album));
    }
    if duration_secs > 0 {
        params.push(("duration", duration.as_str()));
    }
    let url = reqwest::Url::parse_with_params("https://lrclib.net/api/get", &params).ok()?;
    let res = client.get(url).send().await.ok()?;
    if !res.status().is_success() {
        return None;
    }
    res.json().await.ok()
}

async fn lrclib_search(
    client: &reqwest::Client,
    track: &str,
    artist: &str,
) -> Result<Vec<LrcLibRecord>, AppError> {
    let url = reqwest::Url::parse_with_params(
        "https://lrclib.net/api/search",
        &[("track_name", track), ("artist_name", artist)],
    )
    .map_err(|e| AppError::Network(format!("Failed to build lyrics URL: {e}")))?;
    let res = client
        .get(url)
        .send()
        .await
        .map_err(|e| AppError::Network(format!("Failed to query lyrics: {e}")))?;
    if !res.status().is_success() {
        return Err(AppError::Network(format!(
            "Lyrics search returned status {}",
            res.status()
        )));
    }
    res.json()
        .await
        .map_err(|e| AppError::Network(format!("Failed to parse lyrics response: {e}")))
}

/// Picks the best search hit: synced over plain, then closest duration.
fn best_match(records: Vec<LrcLibRecord>, duration_secs: u32) -> Option<LrcLibRecord> {
    let target = f64::from(duration_secs);
    let score = |r: &LrcLibRecord| {
        let distance = if duration_secs > 0 {
            (r.duration - target).abs()
        } else {
            0.0
        };
        // Wildly different durations are usually a different version/edit.
        let distance_penalty = if distance > 8.0 { 1000.0 } else { distance };
        let kind_penalty = if r.has_synced() { 0.0 } else { 100.0 };
        distance_penalty + kind_penalty
    };
    records
        .into_iter()
        .filter(|r| r.has_synced() || r.has_plain())
        .min_by(|a, b| score(a).total_cmp(&score(b)))
}

fn parse_record(record: &LrcLibRecord) -> (Vec<SyncedLyricLine>, bool) {
    if let Some(synced) = record
        .synced_lyrics
        .as_deref()
        .filter(|l| !l.trim().is_empty())
    {
        let lines = parse_lrc(synced);
        if !lines.is_empty() {
            return (lines, true);
        }
    }
    let lines = record
        .plain_lyrics
        .as_deref()
        .unwrap_or_default()
        .lines()
        .map(|line| SyncedLyricLine {
            timestamp_ms: 0,
            text: line.trim().to_string(),
        })
        .collect();
    (lines, false)
}

/// Parses LRC text, keeping blank timed lines (instrumental breaks) and
/// expanding lines that carry several timestamps (`[00:12.00][01:30.00]chorus`).
fn parse_lrc(lrc: &str) -> Vec<SyncedLyricLine> {
    let mut lines = Vec::new();
    for raw in lrc.lines() {
        let mut rest = raw.trim();
        let mut stamps = Vec::new();
        while let Some(stripped) = rest.strip_prefix('[') {
            let Some(close) = stripped.find(']') else {
                break;
            };
            let tag = &stripped[..close];
            match parse_lrc_timestamp(tag) {
                Some(ms) => stamps.push(ms),
                None => break, // metadata tag like [ar:...]
            }
            rest = stripped[close + 1..].trim_start();
        }
        for ms in stamps {
            lines.push(SyncedLyricLine {
                timestamp_ms: ms,
                text: rest.trim().to_string(),
            });
        }
    }
    lines.sort_by_key(|l| l.timestamp_ms);
    lines
}

/// Fetches lyrics for a track from LRCLIB, preferring time-synced lyrics.
#[allow(clippy::missing_errors_doc)]
pub async fn fetch_lyrics(
    track_name: &str,
    artist_name: &str,
    album_name: &str,
    duration_ms: u32,
) -> Result<LyricsData, AppError> {
    let client = lrclib_client();
    let artist = primary_artist(artist_name);
    let duration_secs = (duration_ms + 500) / 1000;

    let mut record = lrclib_get(&client, track_name, artist, album_name, duration_secs).await;
    if !record
        .as_ref()
        .is_some_and(|r| r.has_synced() || r.instrumental)
    {
        let cleaned = clean_title(track_name);
        let hits = match lrclib_search(&client, cleaned, artist).await {
            Ok(hits) => hits,
            // Keep the plain lyrics we already have if the fallback search fails.
            Err(_) if record.is_some() => Vec::new(),
            Err(e) => return Err(e),
        };
        if let Some(found) = best_match(hits, duration_secs) {
            if found.has_synced() || record.as_ref().is_none_or(|r| !r.has_plain()) {
                record = Some(found);
            }
        }
    }

    let (lines, synced) = record.as_ref().map(parse_record).unwrap_or_default();
    Ok(LyricsData {
        track_name: track_name.to_string(),
        artist_name: artist_name.to_string(),
        lines,
        synced,
    })
}

fn parse_lrc_timestamp(ts: &str) -> Option<u32> {
    let (mins, rest) = ts.split_once(':')?;
    let mins: u32 = mins.trim().parse().ok()?;
    let (secs, frac) = rest.split_once('.').unwrap_or((rest, ""));
    let secs: u32 = secs.trim().parse().ok()?;
    // Fractions may be centiseconds ("45") or milliseconds ("450").
    let frac_digits: String = frac.chars().take(3).collect();
    let millis = match frac_digits.len() {
        0 => 0,
        1 => frac_digits.parse::<u32>().ok()? * 100,
        2 => frac_digits.parse::<u32>().ok()? * 10,
        _ => frac_digits.parse::<u32>().ok()?,
    };
    Some(mins * 60_000 + secs * 1_000 + millis)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_lrc_timestamp() {
        assert_eq!(parse_lrc_timestamp("01:23.45"), Some(83_450));
        assert_eq!(parse_lrc_timestamp("00:00.00"), Some(0));
        assert_eq!(parse_lrc_timestamp("00:01.250"), Some(1_250));
        assert_eq!(parse_lrc_timestamp("ar:Someone"), None);
    }

    #[test]
    fn test_lrclib_camel_case_response() {
        let json = r#"{"duration":244.0,"instrumental":false,
            "plainLyrics":"Waiting in a car","syncedLyrics":"[00:10.00] Waiting in a car"}"#;
        let record: LrcLibRecord = serde_json::from_str(json).unwrap();
        assert!(record.has_synced());
        let (lines, synced) = parse_record(&record);
        assert!(synced);
        assert_eq!(lines[0].timestamp_ms, 10_000);
        assert_eq!(lines[0].text, "Waiting in a car");
    }

    #[test]
    fn test_parse_lrc_multi_timestamp_and_metadata() {
        let lines = parse_lrc("[ar:M83]\n[00:20.00][00:05.00]Chorus\n[00:10.00]\n");
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].timestamp_ms, 5_000);
        assert_eq!(lines[1].text, "");
        assert_eq!(lines[2].text, "Chorus");
    }

    #[test]
    fn test_primary_artist_and_clean_title() {
        assert_eq!(primary_artist("Daft Punk, Pharrell Williams"), "Daft Punk");
        assert_eq!(
            primary_artist("Calvin Harris feat. Rihanna"),
            "Calvin Harris"
        );
        assert_eq!(clean_title("Hey Jude - Remastered 2015"), "Hey Jude");
        assert_eq!(clean_title("Song (feat. X)"), "Song");
    }

    #[test]
    fn test_lyrics_data_active_line_detection() {
        let lyrics = LyricsData {
            synced: true,
            track_name: "Midnight City".to_string(),
            artist_name: "M83".to_string(),
            lines: vec![
                SyncedLyricLine {
                    timestamp_ms: 0,
                    text: "Intro".to_string(),
                },
                SyncedLyricLine {
                    timestamp_ms: 10_000,
                    text: "Waiting in a car".to_string(),
                },
                SyncedLyricLine {
                    timestamp_ms: 20_000,
                    text: "Waiting for a ride in the dark".to_string(),
                },
            ],
        };

        let current_pos = 15_000;
        let active_idx = lyrics
            .lines
            .iter()
            .rposition(|l| l.timestamp_ms <= current_pos);
        assert_eq!(active_idx, Some(1));

        let current_pos_start = 500;
        let active_idx_start = lyrics
            .lines
            .iter()
            .rposition(|l| l.timestamp_ms <= current_pos_start);
        assert_eq!(active_idx_start, Some(0));

        let current_pos_later = 25_000;
        let active_idx_later = lyrics
            .lines
            .iter()
            .rposition(|l| l.timestamp_ms <= current_pos_later);
        assert_eq!(active_idx_later, Some(2));
    }
}
