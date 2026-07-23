use crate::api::auth::with_auto_reauth;
use crate::error::AppError;
use rspotify::{AuthCodePkceSpotify, clients::OAuthClient};

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
}
