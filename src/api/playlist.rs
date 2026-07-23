use crate::api::auth::with_auto_reauth;
use crate::error::AppError;
use rspotify::prelude::Id;
use rspotify::{AuthCodePkceSpotify, clients::BaseClient, clients::OAuthClient};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaylistSummary {
    pub id: String,
    pub name: String,
    pub owner_name: String,
    pub image_url: Option<String>,
    pub total_tracks: u32,
}

/// Fetches all playlists owned or followed by the authenticated user (`/me/playlists`), with pagination support.
#[allow(clippy::missing_errors_doc)]
pub async fn fetch_user_playlists(
    spotify: &AuthCodePkceSpotify,
) -> Result<Vec<PlaylistSummary>, AppError> {
    with_auto_reauth(spotify, || async {
        let mut playlists = Vec::new();
        let limit = 50;
        let mut offset = 0;

        loop {
            let page = spotify
                .current_user_playlists_manual(Some(limit), Some(offset))
                .await
                .map_err(|e| AppError::Network(format!("Failed to fetch playlists page: {e}")))?;

            let page_count = page.items.len();
            let has_next = page.next.is_some();

            for item in page.items {
                let owner_name = item
                    .owner
                    .display_name
                    .unwrap_or_else(|| item.owner.id.to_string());
                let image_url = item.images.first().map(|img| img.url.clone());

                #[allow(deprecated)]
                let total_tracks = item.tracks.total;

                playlists.push(PlaylistSummary {
                    id: item.id.to_string(),
                    name: item.name,
                    owner_name,
                    image_url,
                    total_tracks,
                });
            }

            if !has_next || page_count < limit as usize {
                break;
            }

            offset += limit;
        }

        Ok(playlists)
    })
    .await
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaylistTrack {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_ms: u32,
    pub uri: String,
    pub image_url: Option<String>,
}

/// Fetches track listings on demand for a specific playlist ID.
#[allow(clippy::missing_errors_doc)]
pub async fn fetch_playlist_tracks(
    spotify: &AuthCodePkceSpotify,
    playlist_id: &str,
) -> Result<Vec<PlaylistTrack>, AppError> {
    let pid = rspotify::model::PlaylistId::from_id(playlist_id)
        .map_err(|e| AppError::Network(format!("Invalid playlist ID: {e}")))?;

    with_auto_reauth(spotify, || async {
        let mut tracks = Vec::new();
        let limit = 50;
        let mut offset = 0;

        loop {
            let page = spotify
                .playlist_items_manual(pid.clone(), None, None, Some(limit), Some(offset))
                .await
                .map_err(|e| {
                    AppError::Network(format!("Failed to fetch playlist tracks page: {e}"))
                })?;

            let page_count = page.items.len();
            let has_next = page.next.is_some();

            for item in page.items {
                #[allow(deprecated)]
                let maybe_track = item.track;
                if let Some(rspotify::model::PlayableItem::Track(full_track)) = maybe_track {
                    let artist = full_track
                        .artists
                        .iter()
                        .map(|a| a.name.as_str())
                        .collect::<Vec<_>>()
                        .join(", ");

                    let image_url = full_track.album.images.first().map(|img| img.url.clone());
                    let track_id = full_track
                        .id
                        .as_ref()
                        .map_or_else(String::new, ToString::to_string);
                    let uri = full_track.id.as_ref().map_or_else(String::new, Id::uri);

                    tracks.push(PlaylistTrack {
                        id: track_id,
                        title: full_track.name,
                        artist,
                        album: full_track.album.name,
                        duration_ms: u32::try_from(full_track.duration.num_milliseconds())
                            .unwrap_or(0),
                        uri,
                        image_url,
                    });
                }
            }

            if !has_next || page_count < limit as usize {
                break;
            }

            offset += limit;
        }

        Ok(tracks)
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_playlist_summary_struct() {
        let p = PlaylistSummary {
            id: "pl_123".to_string(),
            name: "Synthwave Vibes".to_string(),
            owner_name: "Gena".to_string(),
            image_url: Some("https://example.com/cover.jpg".to_string()),
            total_tracks: 42,
        };
        assert_eq!(p.name, "Synthwave Vibes");
        assert_eq!(p.total_tracks, 42);
    }

    #[test]
    fn test_playlist_track_struct() {
        let t = PlaylistTrack {
            id: "tr_1".to_string(),
            title: "Resonance".to_string(),
            artist: "HOME".to_string(),
            album: "Odyssey".to_string(),
            duration_ms: 212_000,
            uri: "spotify:track:tr_1".to_string(),
            image_url: None,
        };
        assert_eq!(t.title, "Resonance");
        assert_eq!(t.duration_ms, 212_000);
    }
}
