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
    Play(String),
    /// Load a track and start playing at the given position (ms).
    PlayFrom(String, u32),
    Pause,
    Resume,
    #[allow(dead_code)]
    Stop,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
pub enum AudioBitrate {
    Normal96k,
    #[default]
    High160k,
    VeryHigh320k,
}

impl AudioBitrate {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Normal96k => "96 kbps (Normal)",
            Self::High160k => "160 kbps (High)",
            Self::VeryHigh320k => "320 kbps (Very High)",
        }
    }

    #[must_use]
    pub const fn to_librespot_bitrate(self) -> Bitrate {
        match self {
            Self::Normal96k => Bitrate::Bitrate96,
            Self::High160k => Bitrate::Bitrate160,
            Self::VeryHigh320k => Bitrate::Bitrate320,
        }
    }

    pub const ALL: [Self; 3] = [Self::Normal96k, Self::High160k, Self::VeryHigh320k];
}

/// Connects a librespot session from the reusable credentials stored in the keychain.
///
/// Returns [`AppError::PlaybackPairing`] when there are no usable credentials, which
/// tells the UI to start the device pairing flow.
#[allow(clippy::missing_errors_doc)]
pub async fn connect_stored(
    bitrate: AudioBitrate,
    normalisation: bool,
    gapless: bool,
) -> Result<AudioSession, AppError> {
    let credentials = crate::audio::credentials::load_stored_credentials()
        .ok_or_else(|| AppError::PlaybackPairing("No playback credentials stored".to_string()))?;
    match connect_with_credentials(credentials, bitrate, normalisation, gapless).await {
        Err(AppError::PlaybackPairing(reason)) => {
            crate::audio::credentials::delete_stored_credentials();
            Err(AppError::PlaybackPairing(reason))
        }
        other => other,
    }
}

#[allow(clippy::too_many_lines, clippy::missing_errors_doc)]
pub async fn connect_with_credentials(
    credentials: Credentials,
    bitrate: AudioBitrate,
    normalisation: bool,
    gapless: bool,
) -> Result<AudioSession, AppError> {
    let session = Session::new(SessionConfig::default(), None);
    session
        .connect(credentials, false)
        .await
        .map_err(|e| AppError::PlaybackPairing(format!("Spotify rejected the login: {e}")))?;

    // Every track load needs a login5 token; verify it now so a bad credential
    // surfaces as "pair again" instead of every track being "unavailable".
    session
        .login5()
        .auth_token()
        .await
        .map_err(|e| AppError::PlaybackPairing(format!("Spotify denied playback access: {e}")))?;

    crate::audio::credentials::store_session_credentials(&session)?;

    let player_config = PlayerConfig {
        bitrate: bitrate.to_librespot_bitrate(),
        normalisation,
        gapless,
        ..PlayerConfig::default()
    };

    let clock = Arc::new(crate::audio::sink::PlaybackClock::default());
    let (audio_tx, audio_rx) = std::sync::mpsc::sync_channel::<crate::audio::sink::PcmPacket>(8);
    let rodio_sink = crate::audio::sink::spawn_rodio_thread(audio_rx, Arc::clone(&clock))?;
    let sink_clock = Arc::clone(&clock);

    let watch_session = session.clone();
    let player = Player::new(
        player_config,
        session,
        Box::new(NoOpVolume) as Box<dyn VolumeGetter + Send>,
        move || {
            Box::new(crate::audio::sink::MpscSink::new(
                audio_tx.clone(),
                Arc::clone(&sink_clock),
            ))
        },
    );

    let (cmd_tx, mut cmd_rx) = mpsc::channel::<PlayerCommand>(16);
    let (event_tx, event_rx) = mpsc::channel::<AudioSessionEvent>(32);

    let mut librespot_rx = player.get_player_event_channel();
    let player_cmd = Arc::clone(&player);
    let rodio_sink_cmd = Arc::clone(&rodio_sink);

    tokio::spawn(async move {
        let mut is_playing = false;
        let mut track_ended = false;
        // Generation of the current track/seek and the track position it starts at.
        let mut generation = 0_u64;
        let mut base_ms = 0_u32;
        let mut last_sent: Option<u32> = None;
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(250));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        // Position of what is audible right now. Until a new track's first samples
        // reach the speakers this stays at its start, so the bar never runs ahead.
        let audible = |generation: u64, base_ms: u32| {
            clock
                .played_ms(generation)
                .map_or(base_ms, |played| base_ms.saturating_add(played))
        };

        loop {
            tokio::select! {
                maybe_cmd = cmd_rx.recv() => {
                    let Some(cmd) = maybe_cmd else { break };
                    let cmd = match cmd {
                        PlayerCommand::Play(uri) => PlayerCommand::PlayFrom(uri, 0),
                        other => other,
                    };
                    match cmd {
                        PlayerCommand::PlayFrom(uri, start_ms) => {
                            if watch_session.is_invalid() {
                                let _ = event_tx.send(AudioSessionEvent::SessionExpired).await;
                                break;
                            }
                            if uri.trim().is_empty() {
                                eprintln!("Cannot play track with empty Spotify URI");
                                continue;
                            }
                            let uri_to_parse = if uri.starts_with("spotify:") {
                                uri.clone()
                            } else {
                                format!("spotify:track:{uri}")
                            };
                            match SpotifyUri::from_uri(&uri_to_parse) {
                                Ok(spotify_uri) => {
                                    // After a natural end of track the sink still holds
                                    // the song's last moments: let them play out
                                    // instead of clipping it when auto-advancing.
                                    let flush = !track_ended;
                                    if flush {
                                        rodio_sink_cmd.clear();
                                    }
                                    track_ended = false;
                                    generation = clock.next_generation(flush);
                                    base_ms = start_ms;
                                    rodio_sink_cmd.play();
                                    player_cmd.load(spotify_uri, true, start_ms);
                                    is_playing = true;
                                    last_sent = Some(start_ms);
                                    let _ = event_tx
                                        .send(AudioSessionEvent::PositionMs(start_ms))
                                        .await;
                                }
                                Err(e) => eprintln!("Invalid Spotify URI '{uri}': {e}"),
                            }
                        }
                        PlayerCommand::Pause => {
                            player_cmd.pause();
                            rodio_sink_cmd.pause();
                            is_playing = false;
                            let pos = audible(generation, base_ms);
                            last_sent = Some(pos);
                            let _ = event_tx.send(AudioSessionEvent::PositionMs(pos)).await;
                        }
                        PlayerCommand::Resume => {
                            rodio_sink_cmd.play();
                            player_cmd.play();
                            is_playing = true;
                        }
                        PlayerCommand::Stop => {
                            player_cmd.stop();
                            rodio_sink_cmd.stop();
                            is_playing = false;
                            generation = clock.next_generation(true);
                            base_ms = 0;
                            last_sent = Some(0);
                            let _ = event_tx.send(AudioSessionEvent::PositionMs(0)).await;
                        }
                        PlayerCommand::Play(_)
                        | PlayerCommand::SkipNext
                        | PlayerCommand::SkipPrev => {}
                        PlayerCommand::Seek(pos_ms) => {
                            track_ended = false;
                            rodio_sink_cmd.clear();
                            generation = clock.next_generation(true);
                            base_ms = pos_ms;
                            player_cmd.seek(pos_ms);
                            if is_playing {
                                rodio_sink_cmd.play();
                            }
                            last_sent = Some(pos_ms);
                            let _ = event_tx.send(AudioSessionEvent::PositionMs(pos_ms)).await;
                        }
                        PlayerCommand::Volume(vol) => {
                            rodio_sink_cmd.set_volume(vol.clamp(0.0, 1.0));
                        }
                    }
                }
                maybe_event = librespot_rx.recv() => {
                    let Some(event) = maybe_event else {
                        let _ = event_tx.send(AudioSessionEvent::SessionExpired).await;
                        break;
                    };
                    // librespot's own `position_ms` values describe the decoder, which
                    // runs ahead of the speakers; positions come from the clock instead.
                    match &event {
                        PlayerEvent::Playing { .. } => is_playing = true,
                        PlayerEvent::Paused { .. } | PlayerEvent::Unavailable { .. } => {
                            is_playing = false;
                        }
                        PlayerEvent::EndOfTrack { .. } => track_ended = true,
                        PlayerEvent::Stopped { .. } => {
                            is_playing = false;
                            track_ended = false;
                        }
                        _ => {}
                    }
                    if event_tx.send(AudioSessionEvent::Player(event)).await.is_err() {
                        break;
                    }
                }
                _ = interval.tick() => {
                    let pos = audible(generation, base_ms);
                    if last_sent != Some(pos) {
                        last_sent = Some(pos);
                        if event_tx.send(AudioSessionEvent::PositionMs(pos)).await.is_err() {
                            break;
                        }
                    }
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
