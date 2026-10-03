use crate::api::auth::{map_rspotify_error, with_auto_reauth};
use crate::error::AppError;
use rspotify::model::{ArtistId, Market, SearchResult, SearchType};
use rspotify::prelude::Id;
use rspotify::{AuthCodePkceSpotify, clients::BaseClient};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtistTopTrack {
    pub id: String,
    pub title: String,
    pub album: String,
    pub duration_ms: u32,
    pub uri: String,
    pub image_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtistAlbum {
    pub id: String,
    pub name: String,
    pub image_url: Option<String>,
    pub release_date: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtistDetail {
    pub id: String,
    pub name: String,
    pub image_url: Option<String>,
    pub genres: Vec<String>,
    pub followers: u32,
    pub top_tracks: Vec<ArtistTopTrack>,
    pub albums: Vec<ArtistAlbum>,
}

fn to_top_track(t: rspotify::model::FullTrack) -> ArtistTopTrack {
    ArtistTopTrack {
        id: t.id.as_ref().map_or_else(String::new, ToString::to_string),
        uri: t.id.as_ref().map_or_else(String::new, Id::uri),
        title: t.name,
        image_url: crate::api::best_image_url(&t.album.images),
        album: t.album.name,
        duration_ms: u32::try_from(t.duration.num_milliseconds()).unwrap_or(0),
    }
}

/// `/artists/{id}/top-tracks` is forbidden for development-mode apps since Spotify's
/// 2026 Web API changes. Search results are popularity-ordered, so the artist's own
/// tracks from an `artist:` search are a close substitute.
async fn top_tracks_via_search(
    spotify: &AuthCodePkceSpotify,
    artist_id: &str,
    artist_name: &str,
) -> Result<Vec<ArtistTopTrack>, AppError> {
    let query = format!("artist:\"{}\"", artist_name.replace('"', ""));
    let result = spotify
        .search(&query, SearchType::Track, None, None, Some(20), Some(0))
        .await
        .map_err(map_rspotify_error)?;
    let SearchResult::Tracks(page) = result else {
        return Ok(Vec::new());
    };
    Ok(page
        .items
        .into_iter()
        .filter(|t| {
            t.artists
                .iter()
                .any(|a| a.id.as_ref().is_some_and(|id| id.id() == artist_id))
        })
        .take(10)
        .map(to_top_track)
        .collect())
}

/// Fetches detailed artist profile, top tracks, and discography (`/artists/{id}`).
///
/// Only the profile itself is required; top tracks and albums degrade to empty
/// sections instead of failing the whole page.
#[allow(clippy::missing_errors_doc, deprecated)]
pub async fn fetch_artist_details(
    spotify: &AuthCodePkceSpotify,
    artist_id_str: &str,
) -> Result<ArtistDetail, AppError> {
    let aid = ArtistId::from_id(artist_id_str)
        .map_err(|e| AppError::Network(format!("Invalid artist ID '{artist_id_str}': {e}")))?;

    with_auto_reauth(spotify, || async {
        let full_artist = spotify
            .artist(aid.clone())
            .await
            .map_err(map_rspotify_error)?;

        let top_tracks = match spotify
            .artist_top_tracks(aid.clone(), Some(Market::FromToken))
            .await
        {
            Ok(tracks) if !tracks.is_empty() => tracks.into_iter().map(to_top_track).collect(),
            _ => top_tracks_via_search(spotify, artist_id_str, &full_artist.name)
                .await
                .unwrap_or_default(),
        };

        let albums = spotify
            .artist_albums_manual(aid.clone(), None, None, Some(20), Some(0))
            .await
            .map(|page| {
                page.items
                    .into_iter()
                    .map(|a| ArtistAlbum {
                        id: a.id.as_ref().map_or_else(String::new, ToString::to_string),
                        image_url: crate::api::best_image_url(&a.images),
                        release_date: a.release_date.unwrap_or_default(),
                        name: a.name,
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(ArtistDetail {
            id: artist_id_str.to_string(),
            image_url: crate::api::best_image_url(&full_artist.images),
            name: full_artist.name,
            genres: full_artist.genres,
            followers: full_artist.followers.total,
            top_tracks,
            albums,
        })
    })
    .await
}

/// Follows an artist for the authenticated user.
#[allow(clippy::missing_errors_doc)]
#[allow(deprecated)]
pub async fn follow_artist(spotify: &AuthCodePkceSpotify, artist_id: &str) -> Result<(), AppError> {
    use rspotify::clients::OAuthClient;

    let aid = ArtistId::from_id(artist_id)
        .map_err(|e| AppError::Network(format!("Invalid artist ID '{artist_id}': {e}")))?;

    with_auto_reauth(spotify, || async {
        spotify
            .user_follow_artists([aid.clone()])
            .await
            .map_err(map_rspotify_error)?;
        Ok(())
    })
    .await
}

/// Unfollows an artist for the authenticated user.
#[allow(clippy::missing_errors_doc)]
#[allow(deprecated)]
pub async fn unfollow_artist(
    spotify: &AuthCodePkceSpotify,
    artist_id: &str,
) -> Result<(), AppError> {
    use rspotify::clients::OAuthClient;

    let aid = ArtistId::from_id(artist_id)
        .map_err(|e| AppError::Network(format!("Invalid artist ID '{artist_id}': {e}")))?;

    with_auto_reauth(spotify, || async {
        spotify
            .user_unfollow_artists([aid.clone()])
            .await
            .map_err(map_rspotify_error)?;
        Ok(())
    })
    .await
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ArtistBio {
    pub artist_name: String,
    pub title: String,
    pub description: Option<String>,
    pub extract: String,
}

#[derive(serde::Deserialize)]
struct WikiSummaryResponse {
    #[serde(default)]
    r#type: Option<String>,
    #[serde(default)]
    title: Option<String>,
    description: Option<String>,
    extract: Option<String>,
}

const WIKI_USER_AGENT: &str = "Spotifust/0.1.0 (https://github.com/gefydev/spotifust)";

const MUSIC_KEYWORDS: [&str; 30] = [
    "singer",
    "rapper",
    "band",
    "musician",
    "music",
    "songwriter",
    "dj",
    "producer",
    "duo",
    "group",
    "composer",
    "guitarist",
    "drummer",
    "bassist",
    "vocalist",
    "pianist",
    "rock",
    "pop",
    "hip hop",
    "trap",
    "reggaeton",
    "orchestra",
    "artist",
    "cantante",
    "banda",
    "grupo",
    "músico",
    "cantautor",
    "rapero",
    "compositor",
];

/// True when a Wikipedia summary is plausibly about a music act, so "Muse" does
/// not resolve to the Greek goddesses.
fn looks_like_music_act(description: Option<&str>, extract: &str) -> bool {
    let haystack = format!(
        "{} {}",
        description.unwrap_or_default(),
        extract.chars().take(240).collect::<String>()
    )
    .to_lowercase();
    let words: Vec<&str> = haystack.split(|c: char| !c.is_alphanumeric()).collect();
    MUSIC_KEYWORDS.iter().any(|k| {
        if k.contains(' ') {
            haystack.contains(k)
        } else {
            // Whole words (plus plurals) so "pop" doesn't match "population".
            words.iter().any(|w| {
                w == k || w.strip_suffix('s') == Some(k) || w.strip_suffix("es") == Some(k)
            })
        }
    })
}

async fn wiki_summary(
    client: &reqwest::Client,
    lang: &str,
    title: &str,
) -> Option<WikiSummaryResponse> {
    let mut url = reqwest::Url::parse(&format!(
        "https://{lang}.wikipedia.org/api/rest_v1/page/summary/"
    ))
    .ok()?;
    // `pop_if_empty` drops the trailing-slash segment; without it the URL became
    // `.../summary//Title`, which Wikipedia never resolves.
    url.path_segments_mut()
        .ok()?
        .pop_if_empty()
        .push(&title.replace(' ', "_"));
    let resp = client.get(url).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let body: WikiSummaryResponse = resp.json().await.ok()?;
    if body.r#type.as_deref() == Some("disambiguation") {
        return None;
    }
    Some(body)
}

#[derive(serde::Deserialize)]
struct WikiSearchResponse {
    pages: Vec<WikiSearchPage>,
}

#[derive(serde::Deserialize)]
struct WikiSearchPage {
    key: String,
}

/// Last resort: full-text search, keeping only pages whose title starts with the name.
async fn wiki_search_title(client: &reqwest::Client, lang: &str, name: &str) -> Option<String> {
    let url = reqwest::Url::parse_with_params(
        &format!("https://{lang}.wikipedia.org/w/rest.php/v1/search/page"),
        &[("q", format!("{name} music")), ("limit", "5".to_string())],
    )
    .ok()?;
    let resp = client.get(url).send().await.ok()?;
    let body: WikiSearchResponse = resp.json().await.ok()?;
    let wanted = name.replace(' ', "_").to_lowercase();
    body.pages
        .into_iter()
        .map(|p| p.key)
        .find(|key| key.to_lowercase().starts_with(&wanted))
}

#[allow(clippy::missing_errors_doc)]
pub async fn fetch_artist_bio(artist_name: &str) -> Result<ArtistBio, AppError> {
    let trimmed = crate::api::lyrics::primary_artist(artist_name.trim());
    if trimmed.is_empty() {
        return Err(AppError::Network("Empty artist name".to_string()));
    }

    let client = reqwest::Client::builder()
        .user_agent(WIKI_USER_AGENT)
        .timeout(std::time::Duration::from_secs(8))
        .build()
        .map_err(|e| AppError::Network(format!("Failed to build HTTP client: {e}")))?;

    for lang in ["en", "es"] {
        let mut candidates: Vec<String> = vec![trimmed.to_string()];
        for suffix in ["band", "musician", "rapper", "singer", "group"] {
            candidates.push(format!("{trimmed} ({suffix})"));
        }

        let mut found = None;
        for candidate in &candidates {
            if let Some(body) = wiki_summary(&client, lang, candidate).await {
                let extract = body.extract.as_deref().unwrap_or_default().trim();
                if !extract.is_empty() && looks_like_music_act(body.description.as_deref(), extract)
                {
                    found = Some((candidate.clone(), body));
                    break;
                }
            }
        }
        if found.is_none() {
            if let Some(key) = wiki_search_title(&client, lang, trimmed).await {
                if let Some(body) = wiki_summary(&client, lang, &key).await {
                    let extract = body.extract.as_deref().unwrap_or_default().trim();
                    if !extract.is_empty()
                        && looks_like_music_act(body.description.as_deref(), extract)
                    {
                        found = Some((key, body));
                    }
                }
            }
        }

        if let Some((candidate, body)) = found {
            let extract = body.extract.unwrap_or_default().trim().to_string();
            return Ok(ArtistBio {
                artist_name: trimmed.to_string(),
                title: body.title.unwrap_or(candidate),
                description: body.description,
                extract,
            });
        }
    }

    Err(AppError::Network(format!(
        "No Wikipedia bio found for '{trimmed}'"
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_artist_bio_json_deserialization() {
        let json_data = r#"{
            "type": "standard",
            "title": "Daft Punk",
            "description": "French electronic music duo",
            "extract": "Daft Punk were a French electronic music duo formed in 1993 in Paris."
        }"#;
        let res: Result<WikiSummaryResponse, _> = serde_json::from_str(json_data);
        assert!(res.is_ok());
        let body = res.unwrap();
        assert_eq!(body.r#type.as_deref(), Some("standard"));
        assert_eq!(body.title.as_deref(), Some("Daft Punk"));
        assert_eq!(
            body.description.as_deref(),
            Some("French electronic music duo")
        );
        assert!(body.extract.is_some());
    }

    #[test]
    fn test_artist_bio_disambiguation_detection() {
        let json_data = r#"{
            "type": "disambiguation",
            "title": "Queen",
            "description": "Topics referred to by the same term"
        }"#;
        let res: Result<WikiSummaryResponse, _> = serde_json::from_str(json_data);
        assert!(res.is_ok());
        let body = res.unwrap();
        assert_eq!(body.r#type.as_deref(), Some("disambiguation"));
    }

    #[test]
    fn test_music_act_filter() {
        assert!(looks_like_music_act(Some("British rock band"), ""));
        assert!(looks_like_music_act(Some("Argentine rapper"), ""));
        assert!(!looks_like_music_act(
            Some("Inspirational goddesses of literature, science, and the arts"),
            "In ancient Greek religion and mythology, the Muses are the inspirational goddesses."
        ));
    }

    #[test]
    fn test_artist_detail_struct() {
        let ad = ArtistDetail {
            id: "art_1".to_string(),
            name: "GUNSHIP".to_string(),
            image_url: None,
            genres: vec!["synthwave".to_string(), "retrowave".to_string()],
            followers: 250_000,
            top_tracks: vec![ArtistTopTrack {
                id: "t_10".to_string(),
                title: "Tech Noir".to_string(),
                album: "GUNSHIP".to_string(),
                duration_ms: 297_000,
                uri: "spotify:track:t_10".to_string(),
                image_url: None,
            }],
            albums: vec![ArtistAlbum {
                id: "alb_10".to_string(),
                name: "GUNSHIP".to_string(),
                image_url: None,
                release_date: "2015-07-24".to_string(),
            }],
        };
        assert_eq!(ad.name, "GUNSHIP");
        assert_eq!(ad.genres.len(), 2);
        assert_eq!(ad.top_tracks[0].title, "Tech Noir");
    }
}

#[cfg(test)]
mod live_tests {
    #[tokio::test]
    #[ignore = "hits Wikipedia"]
    async fn live_bio_lookup() {
        for (artist, expected) in [
            ("Oasis", "Oasis (band)"),
            ("Muse", "Muse (band)"),
            ("Soda Stereo", "Soda Stereo"),
            ("Bad Bunny, Jhay Cortez", "Bad Bunny"),
        ] {
            let bio = super::fetch_artist_bio(artist).await;
            assert_eq!(
                bio.map(|b| b.title).ok().as_deref(),
                Some(expected),
                "{artist}"
            );
        }
    }
}
