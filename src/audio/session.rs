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

#[derive(Clone)]
pub struct AudioSession {
    #[allow(dead_code)]
    pub player: Arc<Player>,
    pub cmd_tx: mpsc::Sender<PlayerCommand>,
    pub events: Arc<tokio::sync::Mutex<mpsc::Receiver<PlayerEvent>>>,
}

impl std::fmt::Debug for AudioSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AudioSession").finish_non_exhaustive()
    }
}

pub async fn connect_with_token(access_token: &str) -> Result<AudioSession, AppError> {
    let credentials = Credentials::with_access_token(access_token);
    let session_config = SessionConfig::default();

    let session = Session::new(session_config, None);
    session
        .connect(credentials, false)
        .await
        .map_err(|e| AppError::Playback(format!("Librespot login failed: {e}")))?;

    let player_config = PlayerConfig::default();

    let (audio_tx, audio_rx) = mpsc::channel::<Vec<f32>>(256);
    crate::audio::sink::spawn_rodio_thread(audio_rx);

    let player = Player::new(
        player_config,
        session,
        Box::new(NoOpVolume) as Box<dyn VolumeGetter + Send>,
        move || Box::new(crate::audio::sink::MpscSink::new(audio_tx.clone())),
    );

    let (cmd_tx, mut cmd_rx) = mpsc::channel::<PlayerCommand>(16);
    let (event_tx, event_rx) = mpsc::channel::<PlayerEvent>(32);

    let mut librespot_rx = player.get_player_event_channel();
    tokio::spawn(async move {
        while let Some(event) = librespot_rx.recv().await {
            if event_tx.send(event).await.is_err() {
                break;
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
