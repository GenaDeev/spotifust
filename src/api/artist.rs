use crate::api::auth::with_auto_reauth;
use crate::error::AppError;
use rspotify::model::{ArtistId, Market};
use rspotify::prelude::Id;
use rspotify::{AuthCodePkceSpotify, clients::BaseClient};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtistTopTrack {
    pub id: String,
    pub title: String,
    pub album: String,
    pub duration_ms: u32,
    pub uri: String,
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

/// Fetches detailed artist profile, top tracks, and discography (`/artists/{id}`).
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
            .map_err(|e| AppError::Network(format!("Failed to fetch artist profile: {e}")))?;

        let top_tracks_raw = spotify
            .artist_top_tracks(aid.clone(), Some(Market::FromToken))
            .await
            .map_err(|e| AppError::Network(format!("Failed to fetch artist top tracks: {e}")))?;

        let mut top_tracks = Vec::new();
        for t in top_tracks_raw {
            let track_id = t.id.as_ref().map_or_else(String::new, ToString::to_string);
            let uri = t.id.as_ref().map_or_else(String::new, Id::uri);

            top_tracks.push(ArtistTopTrack {
                id: track_id,
                title: t.name,
                album: t.album.name,
                duration_ms: u32::try_from(t.duration.num_milliseconds()).unwrap_or(0),
                uri,
            });
        }

        let albums_page = spotify
            .artist_albums_manual(aid.clone(), None, None, Some(20), Some(0))
            .await
            .map_err(|e| AppError::Network(format!("Failed to fetch artist albums: {e}")))?;

        let mut albums = Vec::new();
        for a in albums_page.items {
            let album_id = a.id.as_ref().map_or_else(String::new, ToString::to_string);
            let image_url = a.images.first().map(|img| img.url.clone());
            let release_date = a.release_date.unwrap_or_default();

            albums.push(ArtistAlbum {
                id: album_id,
                name: a.name,
                image_url,
                release_date,
            });
        }

        let image_url = full_artist.images.first().map(|img| img.url.clone());
        let followers = full_artist.followers.total;

        Ok(ArtistDetail {
            id: artist_id_str.to_string(),
            name: full_artist.name,
            image_url,
            genres: full_artist.genres,
            followers,
            top_tracks,
            albums,
        })
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

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
