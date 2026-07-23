use crate::api::auth::with_auto_reauth;
use crate::error::AppError;
use rspotify::{AuthCodePkceSpotify, clients::OAuthClient};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlbumSummary {
    pub id: String,
    pub name: String,
    pub artist_name: String,
    pub image_url: Option<String>,
    pub total_tracks: u32,
    pub release_date: String,
}

/// Fetches all albums saved by the authenticated user (`/me/albums`), with pagination support.
#[allow(clippy::missing_errors_doc)]
pub async fn fetch_saved_albums(
    spotify: &AuthCodePkceSpotify,
) -> Result<Vec<AlbumSummary>, AppError> {
    with_auto_reauth(spotify, || async {
        let mut albums = Vec::new();
        let limit = 50;
        let mut offset = 0;

        loop {
            let page = spotify
                .current_user_saved_albums_manual(None, Some(limit), Some(offset))
                .await
                .map_err(|e| AppError::Network(format!("Failed to fetch saved albums page: {e}")))?;

            let page_count = page.items.len();
            let has_next = page.next.is_some();

            for item in page.items {
                let full_album = item.album;
                let artist_name = full_album
                    .artists
                    .iter()
                    .map(|a| a.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");

                let image_url = full_album.images.first().map(|img| img.url.clone());
                let album_id = full_album.id.to_string();

                #[allow(deprecated)]
                let total_tracks = full_album.tracks.total;

                albums.push(AlbumSummary {
                    id: album_id,
                    name: full_album.name,
                    artist_name,
                    image_url,
                    total_tracks,
                    release_date: full_album.release_date,
                });
            }

            if !has_next || page_count < limit as usize {
                break;
            }

            offset += limit;
        }

        Ok(albums)
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_album_summary_struct() {
        let a = AlbumSummary {
            id: "alb_1".to_string(),
            name: "Endless Summer".to_string(),
            artist_name: "The Midnight".to_string(),
            image_url: Some("https://example.com/cover.jpg".to_string()),
            total_tracks: 12,
            release_date: "2016-08-05".to_string(),
        };
        assert_eq!(a.name, "Endless Summer");
        assert_eq!(a.total_tracks, 12);
    }
}
