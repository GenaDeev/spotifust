use crate::error::AppError;
use librespot::core::authentication::Credentials;
use librespot::core::config::SessionConfig;
use librespot::core::session::Session;
use librespot::core::spotify_uri::SpotifyUri;
use librespot::playback::config::{Bitrate, PlayerConfig};
use librespot::playback::mixer::{NoOpVolume, VolumeGetter};
use librespot::playback::player::{Player, PlayerEvent};
use std::sync::Arc;
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub enum PlayerCommand {
    #[allow(dead_code)]
    Play(String),
    Pause,
    Resume,
    #[allow(dead_code)]
    SkipNext,
    #[allow(dead_code)]
    SkipPrev,
    Seek(u32),
    Volume(f32),
}

#[derive(Debug, Clone)]
pub enum AudioSessionEvent {
    Player(PlayerEvent),
    PositionMs(u32),
    SessionExpired,
}

#[derive(Clone)]
pub struct AudioSession {
    #[allow(dead_code)]
    pub player: Arc<Player>,
    pub cmd_tx: mpsc::Sender<PlayerCommand>,
    pub events: Arc<tokio::sync::Mutex<mpsc::Receiver<AudioSessionEvent>>>,
}

impl std::fmt::Debug for AudioSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AudioSession").finish_non_exhaustive()
    }
}

#[allow(clippy::too_many_lines)]
pub async fn connect_with_token(access_token: &str) -> Result<AudioSession, AppError> {
    let credentials = Credentials::with_access_token(access_token);
    let session_config = SessionConfig::default();

    let session = Session::new(session_config, None);
    session
        .connect(credentials, false)
        .await
        .map_err(|e| AppError::Playback(format!("Librespot login failed: {e}")))?;

    let player_config = PlayerConfig {
        bitrate: Bitrate::Bitrate320,
        ..PlayerConfig::default()
    };

    let (audio_tx, audio_rx) = std::sync::mpsc::sync_channel::<Vec<f32>>(8);
    let rodio_sink = crate::audio::sink::spawn_rodio_thread(audio_rx)?;

    let player = Player::new(
        player_config,
        session,
        Box::new(NoOpVolume) as Box<dyn VolumeGetter + Send>,
        move || Box::new(crate::audio::sink::MpscSink::new(audio_tx.clone())),
    );

    let (cmd_tx, mut cmd_rx) = mpsc::channel::<PlayerCommand>(16);
    let (event_tx, event_rx) = mpsc::channel::<AudioSessionEvent>(32);

    let mut librespot_rx = player.get_player_event_channel();
    let player_cmd = Arc::clone(&player);
    let rodio_sink_cmd = Arc::clone(&rodio_sink);

    tokio::spawn(async move {
        let mut is_playing = false;
        let mut position_ms = 0;
        let mut last_update = tokio::time::Instant::now();
        let mut current_uri: Option<String> = None;
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(500));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        loop {
            tokio::select! {
                maybe_cmd = cmd_rx.recv() => {
                    if let Some(cmd) = maybe_cmd {
                        match cmd {
                            PlayerCommand::Play(uri) => {
                                rodio_sink_cmd.play();
                                if uri.trim().is_empty() {
                                    eprintln!("Cannot play track with empty Spotify URI");
                                } else {
                                    let uri_to_parse = if uri.starts_with("spotify:") {
                                        uri.clone()
                                    } else {
                                        format!("spotify:track:{uri}")
                                    };
                                    match SpotifyUri::from_uri(&uri_to_parse) {
                                        Ok(spotify_uri) => {
                                            player_cmd.load(spotify_uri, true, 0);
                                            player_cmd.play();
                                            current_uri = Some(uri);
                                            is_playing = true;
                                            position_ms = 0;
                                            last_update = tokio::time::Instant::now();
                                        }
                                        Err(e) => {
                                            eprintln!("Invalid Spotify URI '{uri}': {e}");
                                        }
                                    }
                                }
                            }
                            PlayerCommand::Pause => {
                                player_cmd.pause();
                                rodio_sink_cmd.pause();
                                is_playing = false;
                                last_update = tokio::time::Instant::now();
                                let _ = event_tx.send(AudioSessionEvent::PositionMs(position_ms)).await;
                            }
                            PlayerCommand::Resume => {
                                player_cmd.play();
                                rodio_sink_cmd.play();
                                is_playing = true;
                                last_update = tokio::time::Instant::now();
                                let _ = event_tx.send(AudioSessionEvent::PositionMs(position_ms)).await;
                            }
                            PlayerCommand::SkipNext | PlayerCommand::SkipPrev => {
                                if let Some(ref uri) = current_uri {
                                    let uri_to_parse = if uri.starts_with("spotify:") {
                                        uri.clone()
                                    } else {
                                        format!("spotify:track:{uri}")
                                    };
                                    match SpotifyUri::from_uri(&uri_to_parse) {
                                        Ok(spotify_uri) => {
                                            player_cmd.load(spotify_uri, true, 0);
                                            is_playing = true;
                                            position_ms = 0;
                                            last_update = tokio::time::Instant::now();
                                        }
                                        Err(e) => {
                                            eprintln!("Invalid Spotify URI '{uri}' on skip: {e}");
                                        }
                                    }
                                }
                            }
                            PlayerCommand::Seek(pos_ms) => {
                                player_cmd.seek(pos_ms);
                                position_ms = pos_ms;
                                last_update = tokio::time::Instant::now();
                                let _ = event_tx.send(AudioSessionEvent::PositionMs(position_ms)).await;
                            }
                            PlayerCommand::Volume(vol) => {
                                rodio_sink_cmd.set_volume(vol.clamp(0.0, 1.0));
                            }
                        }
                    } else {
                        break;
                    }
                }
                maybe_event = librespot_rx.recv() => {
                    if let Some(event) = maybe_event {
                        match &event {
                            PlayerEvent::Seeked { position_ms: pos, .. }
                            | PlayerEvent::Playing { position_ms: pos, .. } => {
                                is_playing = true;
                                position_ms = *pos;
                                last_update = tokio::time::Instant::now();
                                let _ = event_tx.send(AudioSessionEvent::PositionMs(position_ms)).await;
                            }
                            PlayerEvent::Paused { position_ms: pos, .. } => {
                                is_playing = false;
                                position_ms = *pos;
                                last_update = tokio::time::Instant::now();
                                let _ = event_tx.send(AudioSessionEvent::PositionMs(position_ms)).await;
                            }
                            PlayerEvent::Stopped { .. } | PlayerEvent::EndOfTrack { .. } => {
                                is_playing = false;
                                position_ms = 0;
                                let _ = event_tx.send(AudioSessionEvent::PositionMs(0)).await;
                            }
                            PlayerEvent::Unavailable { .. } => {
                                is_playing = false;
                                let _ = event_tx.send(AudioSessionEvent::SessionExpired).await;
                            }
                            _ => {}
                        }

                        if event_tx.send(AudioSessionEvent::Player(event)).await.is_err() {
                            break;
                        }
                    } else {
                        let _ = event_tx.send(AudioSessionEvent::SessionExpired).await;
                        break;
                    }
                }
                _ = interval.tick() => {
                    let now = tokio::time::Instant::now();
                    if is_playing {
                        #[allow(clippy::cast_possible_truncation)]
                        let elapsed = now.duration_since(last_update).as_millis() as u32;
                        position_ms += elapsed;

                        if event_tx.send(AudioSessionEvent::PositionMs(position_ms)).await.is_err() {
                            break;
                        }
                    }
                    last_update = now;
                }
            }
        }
    });

    Ok(AudioSession {
        player,
        cmd_tx,
        events: Arc::new(tokio::sync::Mutex::new(event_rx)),
    })
}
