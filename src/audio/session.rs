use librespot::core::authentication::Credentials;
use librespot::core::config::SessionConfig;
use librespot::core::session::Session;
use librespot::playback::config::PlayerConfig;
use librespot::playback::mixer::{NoOpVolume, VolumeGetter};
use librespot::playback::player::{Player, PlayerEvent};
use std::sync::Arc;
use tokio::sync::mpsc;
use crate::error::AppError;

#[derive(Clone)]
pub struct AudioSession {
    pub player: Arc<Player>,
    pub events: Arc<tokio::sync::Mutex<mpsc::Receiver<PlayerEvent>>>,
}

impl std::fmt::Debug for AudioSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AudioSession").finish_non_exhaustive()
    }
}

#[allow(dead_code)]
pub async fn connect_with_token(access_token: &str) -> Result<AudioSession, AppError> {
    let credentials = Credentials::with_access_token(access_token);
    let session_config = SessionConfig::default();

    let session = Session::new(session_config, None);
    session.connect(credentials, false)
        .await
        .map_err(|e| AppError::Playback(format!("Librespot login failed: {e}")))?;

    let player_config = PlayerConfig::default();

    // Create the bounded audio pipeline channel
    let (audio_tx, audio_rx) = mpsc::channel::<Vec<f32>>(256);
    
    // Spawn the rodio consumer thread
    crate::audio::sink::spawn_rodio_thread(audio_rx);

    let player = Player::new(
        player_config,
        session,
        Box::new(NoOpVolume) as Box<dyn VolumeGetter + Send>,
        move || Box::new(crate::audio::sink::MpscSink::new(audio_tx)),
    );

    let (tx, rx) = mpsc::channel::<PlayerEvent>(32);
    let mut librespot_rx = player.get_player_event_channel();

    // Bridge unbounded librespot events to bounded iced/UI subscription channel
    tokio::spawn(async move {
        while let Some(event) = librespot_rx.recv().await {
            if tx.send(event).await.is_err() {
                break;
            }
        }
    });

    Ok(AudioSession { player, events: Arc::new(tokio::sync::Mutex::new(rx)) })
}
