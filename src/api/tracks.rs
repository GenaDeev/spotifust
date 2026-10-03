use crate::api::auth::{map_rspotify_error, with_auto_reauth};
use crate::error::AppError;
use rspotify::prelude::Id;
use rspotify::{AuthCodePkceSpotify, clients::OAuthClient};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TopTrack {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_ms: u32,
    pub uri: String,
    pub image_url: Option<String>,
    #[serde(default)]
    pub explicit: bool,
}

/// Fetches the user's top tracks (`/me/top/tracks`).
#[allow(clippy::missing_errors_doc)]
pub async fn fetch_top_tracks(spotify: &AuthCodePkceSpotify) -> Result<Vec<TopTrack>, AppError> {
    with_auto_reauth(spotify, || async {
        let page = spotify
            .current_user_top_tracks_manual(None, Some(20), None)
            .await
            .map_err(map_rspotify_error)?;

        let mut tracks = Vec::new();
        for full_track in page.items {
            let artist = full_track
                .artists
                .iter()
                .map(|a| a.name.as_str())
                .collect::<Vec<_>>()
                .join(", ");

            let image_url = crate::api::best_image_url(&full_track.album.images);
            let track_id = full_track
                .id
                .as_ref()
                .map_or_else(String::new, ToString::to_string);
            let uri = full_track.id.as_ref().map_or_else(String::new, Id::uri);

            tracks.push(TopTrack {
                id: track_id,
                title: full_track.name,
                artist,
                album: full_track.album.name,
                duration_ms: u32::try_from(full_track.duration.num_milliseconds()).unwrap_or(0),
                uri,
                image_url,
                explicit: full_track.explicit,
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
    pub image_url: Option<String>,
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
            .map_err(map_rspotify_error)?;

        let Some(ctx) = playing_context else {
            return Ok(None);
        };
        let Some(item) = ctx.item else {
            return Ok(None);
        };

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

            let image_url = crate::api::best_image_url(&full_track.album.images);

            Ok(Some(CurrentlyPlayingInfo {
                title: full_track.name,
                artist,
                album: full_track.album.name,
                duration_ms,
                progress_ms,
                is_playing: ctx.is_playing,
                uri,
                image_url,
            }))
        } else {
            Ok(None)
        }
    })
    .await
}

#[allow(clippy::missing_errors_doc, clippy::too_many_lines)]
pub async fn fetch_recommendations(
    spotify: &AuthCodePkceSpotify,
    seed_track_ids: &[String],
) -> Result<Vec<TopTrack>, AppError> {
    use rspotify::clients::BaseClient;
    use rspotify::model::TrackId;

    let track_ids: Vec<TrackId<'_>> = seed_track_ids
        .iter()
        .filter_map(|id| TrackId::from_id_or_uri(id).ok())
        .collect();

    if track_ids.is_empty() {
        return Ok(Vec::new());
    }

    with_auto_reauth(spotify, || async {
        let recs_res = spotify
            .recommendations(
                std::iter::empty(),
                None::<Vec<rspotify::model::ArtistId<'_>>>,
                None::<Vec<&str>>,
                Some(track_ids.clone()),
                None,
                Some(20),
            )
            .await;

        if let Ok(recs) = recs_res {
            let mut tracks = Vec::new();
            for track in recs.tracks {
                let artist = track
                    .artists
                    .iter()
                    .map(|a| a.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");

                let track_id = track
                    .id
                    .as_ref()
                    .map_or_else(String::new, ToString::to_string);
                let uri = track.id.as_ref().map_or_else(String::new, Id::uri);

                tracks.push(TopTrack {
                    id: track_id,
                    title: track.name,
                    artist,
                    album: String::new(),
                    duration_ms: u32::try_from(track.duration.num_milliseconds()).unwrap_or(0),
                    uri,
                    image_url: None,
                    explicit: track.explicit,
                });
            }
            if !tracks.is_empty() {
                return Ok(tracks);
            }
        }

        if let Some(first_track_id) = track_ids.first() {
            if let Ok(full_track) = spotify.track(first_track_id.clone(), None).await {
                if let Some(first_artist) = full_track.artists.first() {
                    let search_query = format!("artist:\"{}\"", first_artist.name);
                    if let Ok(rspotify::model::SearchResult::Tracks(tracks_page)) = spotify
                        .search(
                            &search_query,
                            rspotify::model::SearchType::Track,
                            None,
                            None,
                            Some(20),
                            Some(0),
                        )
                        .await
                    {
                        let mut fallback_tracks = Vec::new();
                        for t in tracks_page.items {
                            let artist = t
                                .artists
                                .iter()
                                .map(|a| a.name.as_str())
                                .collect::<Vec<_>>()
                                .join(", ");
                            let track_id =
                                t.id.as_ref().map_or_else(String::new, ToString::to_string);
                            let uri = t.id.as_ref().map_or_else(String::new, Id::uri);
                            let image_url = crate::api::best_image_url(&t.album.images);
                            fallback_tracks.push(TopTrack {
                                id: track_id,
                                title: t.name,
                                artist,
                                album: t.album.name,
                                duration_ms: u32::try_from(t.duration.num_milliseconds())
                                    .unwrap_or(0),
                                uri,
                                image_url,
                                explicit: t.explicit,
                            });
                        }
                        if !fallback_tracks.is_empty() {
                            return Ok(fallback_tracks);
                        }
                    }
                }
            }
        }

        let top = spotify
            .current_user_top_tracks_manual(None, Some(20), None)
            .await
            .map_err(map_rspotify_error)?;

        let mut tracks = Vec::new();
        for t in top.items {
            let artist = t
                .artists
                .iter()
                .map(|a| a.name.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            let track_id = t.id.as_ref().map_or_else(String::new, ToString::to_string);
            let uri = t.id.as_ref().map_or_else(String::new, Id::uri);
            let image_url = crate::api::best_image_url(&t.album.images);
            tracks.push(TopTrack {
                id: track_id,
                title: t.name,
                artist,
                album: t.album.name,
                duration_ms: u32::try_from(t.duration.num_milliseconds()).unwrap_or(0),
                uri,
                image_url,
                explicit: t.explicit,
            });
        }
        Ok(tracks)
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
            explicit: false,
        };
        assert_eq!(t.title, "Stardust");
        assert!(!t.explicit);
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
            image_url: None,
        };
        assert_eq!(cp.title, "Nightcall");
        assert!(cp.is_playing);
    }

    #[tokio::test]
    async fn test_fetch_recommendations_empty_seeds() {
        let creds = rspotify::Credentials::new("mock_client_id", "mock_client_secret");
        let oauth = rspotify::OAuth::default();
        let spotify = rspotify::AuthCodePkceSpotify::new(creds, oauth);
        let res = fetch_recommendations(&spotify, &[]).await;
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), Vec::new());
    }
}
