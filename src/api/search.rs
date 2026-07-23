use crate::api::auth::with_auto_reauth;
use crate::error::AppError;
use rspotify::model::{SearchResult, SearchType};
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

/// Executes a search query across tracks, albums, and artists (`/search`).
#[allow(clippy::missing_errors_doc)]
pub async fn execute_search(
    spotify: &AuthCodePkceSpotify,
    query: &str,
) -> Result<SearchResults, AppError> {
    if query.trim().is_empty() {
        return Ok(SearchResults::default());
    }

    with_auto_reauth(spotify, || async {
        let result = spotify
            .search(query, SearchType::Track, None, None, Some(10), Some(0))
            .await
            .map_err(|e| AppError::Network(format!("Search request failed: {e}")))?;

        let mut search_results = SearchResults::default();

        if let SearchResult::Tracks(tracks_page) = result {
            for track in tracks_page.items {
                let artist = track
                    .artists
                    .iter()
                    .map(|a| a.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");

                let image_url = track.album.images.first().map(|img| img.url.clone());
                let track_id = track
                    .id
                    .as_ref()
                    .map_or_else(String::new, ToString::to_string);
                let uri = track.id.as_ref().map_or_else(String::new, Id::uri);

                search_results.tracks.push(SearchResultTrack {
                    id: track_id,
                    title: track.name,
                    artist,
                    album: track.album.name,
                    duration_ms: u32::try_from(track.duration.num_milliseconds()).unwrap_or(0),
                    uri,
                    image_url,
                });
            }
        }

        Ok(search_results)
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_results_default() {
        let res = SearchResults::default();
        assert!(res.tracks.is_empty());
        assert!(res.albums.is_empty());
        assert!(res.artists.is_empty());
    }
}
