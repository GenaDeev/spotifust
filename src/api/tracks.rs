use crate::api::auth::with_auto_reauth;
use crate::error::AppError;
use rspotify::prelude::Id;
use rspotify::{AuthCodePkceSpotify, clients::OAuthClient};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TopTrack {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_ms: u32,
    pub uri: String,
    pub image_url: Option<String>,
}

/// Fetches the user's top tracks (`/me/top/tracks`).
#[allow(clippy::missing_errors_doc)]
pub async fn fetch_top_tracks(
    spotify: &AuthCodePkceSpotify,
) -> Result<Vec<TopTrack>, AppError> {
    with_auto_reauth(spotify, || async {
        let page = spotify
            .current_user_top_tracks_manual(None, Some(20), None)
            .await
            .map_err(|e| AppError::Network(format!("Failed to fetch top tracks: {e}")))?;

        let mut tracks = Vec::new();
        for full_track in page.items {
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
            let uri = full_track
                .id
                .as_ref()
                .map_or_else(String::new, Id::uri);

            tracks.push(TopTrack {
                id: track_id,
                title: full_track.name,
                artist,
                album: full_track.album.name,
                duration_ms: u32::try_from(full_track.duration.num_milliseconds()).unwrap_or(0),
                uri,
                image_url,
            });
        }

        Ok(tracks)
    })
    .await
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrentlyPlayingInfo {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_ms: u32,
    pub progress_ms: u32,
    pub is_playing: bool,
    pub uri: String,
}

/// Fetches the user's currently playing track (`/me/player/currently-playing`).
#[allow(clippy::missing_errors_doc)]
pub async fn fetch_currently_playing(
    spotify: &AuthCodePkceSpotify,
) -> Result<Option<CurrentlyPlayingInfo>, AppError> {
    use rspotify::clients::OAuthClient;
    use rspotify::model::PlayableItem;

    with_auto_reauth(spotify, || async {
        let playing_context = spotify
            .current_playing(None, None::<Vec<_>>)
            .await
            .map_err(|e| AppError::Network(format!("Failed to fetch currently playing: {e}")))?;

        let Some(ctx) = playing_context else { return Ok(None) };
        let Some(item) = ctx.item else { return Ok(None) };

        if let PlayableItem::Track(full_track) = item {
            let artist = full_track
                .artists
                .iter()
                .map(|a| a.name.as_str())
                .collect::<Vec<_>>()
                .join(", ");

            let progress_ms =
                u32::try_from(ctx.progress.map_or(0, |d| d.num_milliseconds())).unwrap_or(0);
            let duration_ms = u32::try_from(full_track.duration.num_milliseconds()).unwrap_or(0);
            let uri = full_track.id.as_ref().map_or_else(String::new, Id::uri);

            Ok(Some(CurrentlyPlayingInfo {
                title: full_track.name,
                artist,
                album: full_track.album.name,
                duration_ms,
                progress_ms,
                is_playing: ctx.is_playing,
                uri,
            }))
        } else {
            Ok(None)
        }
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_top_track_struct() {
        let t = TopTrack {
            id: "tt_1".to_string(),
            title: "Stardust".to_string(),
            artist: "Kavinsky".to_string(),
            album: "OutRun".to_string(),
            duration_ms: 180_000,
            uri: "spotify:track:tt_1".to_string(),
            image_url: None,
        };
        assert_eq!(t.title, "Stardust");
    }

    #[test]
    fn test_currently_playing_info_struct() {
        let cp = CurrentlyPlayingInfo {
            title: "Nightcall".to_string(),
            artist: "Kavinsky".to_string(),
            album: "Nightcall".to_string(),
            duration_ms: 259_000,
            progress_ms: 30_000,
            is_playing: true,
            uri: "spotify:track:cp_1".to_string(),
        };
        assert_eq!(cp.title, "Nightcall");
        assert!(cp.is_playing);
    }
}
