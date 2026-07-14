use crate::error::AppError;
use librespot::core::authentication::Credentials;
use librespot::core::config::SessionConfig;
use librespot::core::session::Session;
use librespot::core::spotify_uri::SpotifyUri;
use librespot::playback::config::PlayerConfig;
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
    SkipNext,
    SkipPrev,
    Seek(u32),
}

#[derive(Debug, Clone)]
pub enum AudioSessionEvent {
    Player(PlayerEvent),
    PositionMs(u32),
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

    let player_config = PlayerConfig::default();

    let builder = rodio::OutputStreamBuilder::from_default_device()
        .map_err(|e| AppError::Playback(format!("Failed to get default audio device: {e}")))?;
    let stream = builder.open_stream()
        .map_err(|e| AppError::Playback(format!("Failed to open audio stream: {e}")))?;
    let rodio_sink = rodio::Sink::connect_new(stream.mixer());

    let (audio_tx, audio_rx) = mpsc::channel::<Vec<f32>>(8);
    crate::audio::sink::spawn_rodio_thread(audio_rx, rodio_sink, stream);

    let player = Player::new(
        player_config,
        session,
        Box::new(NoOpVolume) as Box<dyn VolumeGetter + Send>,
        move || Box::new(crate::audio::sink::MpscSink::new(audio_tx.clone())),
    );

    let (cmd_tx, mut cmd_rx) = mpsc::channel::<PlayerCommand>(16);
    let (event_tx, event_rx) = mpsc::channel::<AudioSessionEvent>(32);

    let mut librespot_rx = player.get_player_event_channel();
    tokio::spawn(async move {
        let mut is_playing = false;
        let mut position_ms = 0;
        let mut last_update = tokio::time::Instant::now();
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(500));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        loop {
            tokio::select! {
                maybe_event = librespot_rx.recv() => {
                    match maybe_event {
                        Some(event) => {
                            match &event {
                                PlayerEvent::Playing { position_ms: pos, .. } => {
                                    is_playing = true;
                                    position_ms = *pos;
                                    last_update = tokio::time::Instant::now();
                                    let _ = event_tx.send(AudioSessionEvent::PositionMs(position_ms)).await;
                                }
                                PlayerEvent::Paused { position_ms: pos, .. } => {
                                    is_playing = false;
                                    position_ms = *pos;
                                    let _ = event_tx.send(AudioSessionEvent::PositionMs(position_ms)).await;
                                }
                                PlayerEvent::Stopped { .. } => {
                                    is_playing = false;
                                    position_ms = 0;
                                    let _ = event_tx.send(AudioSessionEvent::PositionMs(position_ms)).await;
                                }
                                PlayerEvent::EndOfTrack { .. } => {
                                    is_playing = false;
                                }
                                _ => {}
                            }

                            if event_tx.send(AudioSessionEvent::Player(event)).await.is_err() {
                                break;
                            }
                        }
                        None => break,
                    }
                }
                _ = interval.tick() => {
                    if is_playing {
                        let now = tokio::time::Instant::now();
                        #[allow(clippy::cast_possible_truncation)]
                        let elapsed = now.duration_since(last_update).as_millis() as u32;
                        position_ms += elapsed;
                        last_update = now;

                        if event_tx.send(AudioSessionEvent::PositionMs(position_ms)).await.is_err() {
                            break;
                        }
                    }
                }
            }
        }
    });

    let player_cmd = Arc::clone(&player);
    tokio::spawn(async move {
        let mut current_uri: Option<String> = None;

        while let Some(cmd) = cmd_rx.recv().await {
            match cmd {
                PlayerCommand::Play(uri) => match SpotifyUri::from_uri(&uri) {
                    Ok(spotify_uri) => {
                        player_cmd.load(spotify_uri, true, 0);
                        current_uri = Some(uri);
                    }
                    Err(e) => {
                        eprintln!("Invalid Spotify URI '{uri}': {e}");
                    }
                },
                PlayerCommand::Pause => {
                    player_cmd.pause();
                }
                PlayerCommand::Resume => {
                    player_cmd.play();
                }
                PlayerCommand::SkipNext | PlayerCommand::SkipPrev => {
                    if let Some(ref uri) = current_uri {
                        match SpotifyUri::from_uri(uri) {
                            Ok(spotify_uri) => {
                                player_cmd.load(spotify_uri, true, 0);
                            }
                            Err(e) => {
                                eprintln!("Invalid Spotify URI '{uri}' on skip: {e}");
                            }
                        }
                    }
                }
                PlayerCommand::Seek(pos_ms) => {
                    player_cmd.seek(pos_ms);
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
