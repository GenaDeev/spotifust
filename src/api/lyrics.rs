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
}

/// Fetches synchronized lyrics for a track from LRCLIB API.
#[allow(clippy::missing_errors_doc)]
pub async fn fetch_lyrics(track_name: &str, artist_name: &str) -> Result<LyricsData, AppError> {
    let client = reqwest::Client::new();
    let url = reqwest::Url::parse_with_params(
        "https://lrclib.net/api/get",
        &[("track_name", track_name), ("artist_name", artist_name)],
    )
    .map_err(|e| AppError::Network(format!("Failed to build lyrics URL: {e}")))?;

    let res = client
        .get(url)
        .send()
        .await
        .map_err(|e| AppError::Network(format!("Failed to query lyrics: {e}")))?;

    if !res.status().is_success() {
        return Err(AppError::Network("Lyrics not found".to_string()));
    }

    #[derive(serde::Deserialize)]
    struct LrcLibResponse {
        synced_lyrics: Option<String>,
        plain_lyrics: Option<String>,
    }

    let body: LrcLibResponse = res
        .json()
        .await
        .map_err(|e| AppError::Network(format!("Failed to parse lyrics response: {e}")))?;

    let mut lines = Vec::new();

    if let Some(synced) = body.synced_lyrics {
        for line in synced.lines() {
            if line.starts_with('[') && line.contains(']') {
                let parts: Vec<&str> = line.splitn(2, ']').collect();
                if parts.len() == 2 {
                    let timestamp_str = parts[0].trim_start_matches('[');
                    let text = parts[1].trim().to_string();
                    let ms = parse_lrc_timestamp(timestamp_str);
                    lines.push(SyncedLyricLine {
                        timestamp_ms: ms,
                        text,
                    });
                }
            }
        }
    } else if let Some(plain) = body.plain_lyrics {
        for (idx, line) in plain.lines().enumerate() {
            lines.push(SyncedLyricLine {
                timestamp_ms: (idx * 3000) as u32,
                text: line.to_string(),
            });
        }
    }

    Ok(LyricsData {
        track_name: track_name.to_string(),
        artist_name: artist_name.to_string(),
        lines,
    })
}

fn parse_lrc_timestamp(ts: &str) -> u32 {
    let parts: Vec<&str> = ts.split(':').collect();
    if parts.len() == 2 {
        let mins: u32 = parts[0].parse().unwrap_or(0);
        let sec_parts: Vec<&str> = parts[1].split('.').collect();
        let secs: u32 = sec_parts[0].parse().unwrap_or(0);
        let millis: u32 = if sec_parts.len() > 1 {
            sec_parts[1].parse::<u32>().unwrap_or(0) * 10
        } else {
            0
        };
        return mins * 60_000 + secs * 1_000 + millis;
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_lrc_timestamp() {
        assert_eq!(parse_lrc_timestamp("01:23.45"), 83_450);
        assert_eq!(parse_lrc_timestamp("00:00.00"), 0);
    }
}
