use crate::api::auth::{map_rspotify_error, with_auto_reauth};
use crate::error::AppError;
use rspotify::model::SearchType;
use rspotify::prelude::Id;
use rspotify::{AuthCodePkceSpotify, clients::BaseClient};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchResultTrack {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_ms: u32,
    pub uri: String,
    pub image_url: Option<String>,
    pub explicit: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchResultAlbum {
    pub id: String,
    pub name: String,
    pub artist_name: String,
    pub image_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchResultArtist {
    pub id: String,
    pub name: String,
    pub image_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SearchResults {
    pub tracks: Vec<SearchResultTrack>,
    pub albums: Vec<SearchResultAlbum>,
    pub artists: Vec<SearchResultArtist>,
}

/// Results requested per category; one request covers tracks, albums and artists.
const SEARCH_LIMIT: u32 = 10;

/// Executes a search query across tracks, albums, and artists (`/search`).
///
/// Uses a single multi-type request: the three sequential per-type requests this
/// replaced cost three round trips (~1.8 s on a 240 ms RTT link).
#[allow(clippy::missing_errors_doc)]
pub async fn execute_search(
    spotify: &AuthCodePkceSpotify,
    query: &str,
) -> Result<SearchResults, AppError> {
    if query.trim().is_empty() {
        return Ok(SearchResults::default());
    }

    with_auto_reauth(spotify, || async {
        let res = spotify
            .search_multiple(
                query,
                [SearchType::Track, SearchType::Album, SearchType::Artist],
                None,
                None,
                Some(SEARCH_LIMIT),
                Some(0),
            )
            .await
            .map_err(map_rspotify_error)?;

        let tracks = res
            .tracks
            .map(|page| {
                page.items
                    .into_iter()
                    .map(|track| SearchResultTrack {
                        id: track
                            .id
                            .as_ref()
                            .map_or_else(String::new, ToString::to_string),
                        uri: track.id.as_ref().map_or_else(String::new, Id::uri),
                        artist: track
                            .artists
                            .iter()
                            .map(|a| a.name.as_str())
                            .collect::<Vec<_>>()
                            .join(", "),
                        image_url: crate::api::best_image_url(&track.album.images),
                        album: track.album.name,
                        duration_ms: u32::try_from(track.duration.num_milliseconds()).unwrap_or(0),
                        explicit: track.explicit,
                        title: track.name,
                    })
                    .collect()
            })
            .unwrap_or_default();

        let albums = res
            .albums
            .map(|page| {
                page.items
                    .into_iter()
                    .map(|album| SearchResultAlbum {
                        id: album.id.map_or_else(String::new, |id| id.to_string()),
                        artist_name: album
                            .artists
                            .iter()
                            .map(|a| a.name.as_str())
                            .collect::<Vec<_>>()
                            .join(", "),
                        image_url: crate::api::best_image_url(&album.images),
                        name: album.name,
                    })
                    .collect()
            })
            .unwrap_or_default();

        let artists = res
            .artists
            .map(|page| {
                page.items
                    .into_iter()
                    .map(|artist| SearchResultArtist {
                        id: artist.id.to_string(),
                        image_url: crate::api::best_image_url(&artist.images),
                        name: artist.name,
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(SearchResults {
            tracks,
            albums,
            artists,
        })
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_results_default() {
        let res = SearchResults::default();
        assert_eq!(res.tracks, Vec::new());
        assert_eq!(res.albums, Vec::new());
        assert_eq!(res.artists, Vec::new());
    }

    #[test]
    fn test_search_result_track_explicit() {
        let t = SearchResultTrack {
            id: "track_1".to_string(),
            title: "Test Track".to_string(),
            artist: "Test Artist".to_string(),
            album: "Test Album".to_string(),
            duration_ms: 200_000,
            uri: "spotify:track:track_1".to_string(),
            image_url: None,
            explicit: true,
        };
        assert!(t.explicit);
    }
}

#[cfg(test)]
mod live_tests {
    #[tokio::test]
    #[ignore = "hits the Spotify Web API; run with SPOTIFUST_LIVE_KEYRING=1"]
    async fn live_search_latency() {
        let t0 = std::time::Instant::now();
        let spotify = crate::api::auth::check_existing_login()
            .await
            .expect("stored login");
        println!("login/refresh: {:?}", t0.elapsed());
        for q in ["oasis", "oasis wonderwall", "soda stereo"] {
            let t = std::time::Instant::now();
            let res = super::execute_search(&spotify, q).await.expect("search");
            println!(
                "search '{q}': {:?} ({} tracks, {} albums, {} artists)",
                t.elapsed(),
                res.tracks.len(),
                res.albums.len(),
                res.artists.len()
            );
        }
    }
}
