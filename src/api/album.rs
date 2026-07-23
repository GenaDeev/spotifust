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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlbumDetailTrack {
    pub id: String,
    pub track_number: u32,
    pub title: String,
    pub artist: String,
    pub duration_ms: u32,
    pub uri: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlbumDetail {
    pub id: String,
    pub name: String,
    pub artist_name: String,
    pub image_url: Option<String>,
    pub release_date: String,
    pub total_tracks: u32,
    pub tracks: Vec<AlbumDetailTrack>,
}

/// Fetches detailed album information and its tracks (`/albums/{id}`).
#[allow(clippy::missing_errors_doc)]
pub async fn fetch_album_details(
    spotify: &AuthCodePkceSpotify,
    album_id_str: &str,
) -> Result<AlbumDetail, AppError> {
    use rspotify::clients::BaseClient;
    use rspotify::model::AlbumId;
    use rspotify::prelude::Id;

    let aid = AlbumId::from_id(album_id_str)
        .map_err(|e| AppError::Network(format!("Invalid album ID '{album_id_str}': {e}")))?;

    with_auto_reauth(spotify, || async {
        let full_album = spotify
            .album(aid.clone(), None)
            .await
            .map_err(|e| AppError::Network(format!("Failed to fetch album details: {e}")))?;

        let artist_name = full_album
            .artists
            .iter()
            .map(|a| a.name.as_str())
            .collect::<Vec<_>>()
            .join(", ");

        let image_url = full_album.images.first().map(|img| img.url.clone());
        let total_tracks = full_album.tracks.total;

        let mut tracks = Vec::new();
        for simple_track in full_album.tracks.items {
            let artist = simple_track
                .artists
                .iter()
                .map(|a| a.name.as_str())
                .collect::<Vec<_>>()
                .join(", ");

            let track_id = simple_track
                .id
                .as_ref()
                .map_or_else(String::new, ToString::to_string);
            let uri = simple_track
                .id
                .as_ref()
                .map_or_else(String::new, Id::uri);

            tracks.push(AlbumDetailTrack {
                id: track_id,
                track_number: simple_track.track_number,
                title: simple_track.name,
                artist,
                duration_ms: u32::try_from(simple_track.duration.num_milliseconds()).unwrap_or(0),
                uri,
            });
        }

        Ok(AlbumDetail {
            id: album_id_str.to_string(),
            name: full_album.name,
            artist_name,
            image_url,
            release_date: full_album.release_date,
            total_tracks,
            tracks,
        })
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

    #[test]
    fn test_album_detail_struct() {
        let ad = AlbumDetail {
            id: "alb_2".to_string(),
            name: "Monsters".to_string(),
            artist_name: "The Midnight".to_string(),
            image_url: None,
            release_date: "2020-07-10".to_string(),
            total_tracks: 15,
            tracks: vec![AlbumDetailTrack {
                id: "t_1".to_string(),
                track_number: 1,
                title: "1984".to_string(),
                artist: "The Midnight".to_string(),
                duration_ms: 200_000,
                uri: "spotify:track:t_1".to_string(),
            }],
        };
        assert_eq!(ad.tracks.len(), 1);
        assert_eq!(ad.tracks[0].title, "1984");
    }
}
