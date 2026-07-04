use crate::audio::engine::{AudioCommand, AudioEngine};
use crate::error::AppError;
use crate::ui::login;
use iced::{Element, Task};
use rspotify::clients::BaseClient;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavigationItem {
    Home,
    Search,
    Library,
}

#[derive(Debug, Clone)]
pub struct TrackInfo {
    pub title: String,
    pub artist: String,
    pub duration_ms: u32,
}

#[derive(Debug, Clone)]
pub struct PlaybackState {
    pub is_playing: bool,
    pub current_track: Option<TrackInfo>,
    pub progress_ms: u32,
    pub volume: f32,
}

impl Default for PlaybackState {
    fn default() -> Self {
        Self {
            is_playing: false,
            current_track: None,
            progress_ms: 0,
            volume: 1.0,
        }
    }
}

pub enum AppState {
    Login {
        is_loading: bool,
        error: Option<String>,
    },
    Main {
        nav_item: NavigationItem,
        playback: PlaybackState,
        // session: Option<crate::audio::session::AudioSession>, // Kept for Phase 3 integration
    },
}

pub struct App {
    pub state: AppState,
    #[allow(dead_code)]
    pub audio_tx: tokio::sync::mpsc::Sender<AudioCommand>,
}

#[derive(Debug, Clone)]
pub enum Message {
    #[allow(dead_code)]
    ErrorEncountered(AppError),
    // Login Messages
    LoginRequested,
    CheckLogin,
    CheckLoginFailed,
    LoginSuccess(Box<rspotify::AuthCodePkceSpotify>),
    LoginFailed(String),
    // Audio Messages
    AudioSessionConnected(crate::audio::session::AudioSession),
    // Main UI Messages
    NavigationSelected(NavigationItem),
    TogglePlayback,
    SkipNext,
    SkipPrev,
    SeekTo(f32),        // 0.0 to 1.0
    VolumeChanged(f32), // 0.0 to 1.0
}

impl App {
    pub fn new() -> (Self, Task<Message>) {
        let audio_tx = AudioEngine::spawn();
        // Removed auto-play of test sine wave

        (
            Self {
                state: AppState::Login {
                    is_loading: true, // Initial check
                    error: None,
                },
                audio_tx,
            },
            Task::perform(
                async { crate::api::auth::check_existing_login().await },
                |res| match res {
                    Ok(spotify) => Message::LoginSuccess(Box::new(spotify)),
                    Err(_) => Message::LoginFailed("No token".to_string()),
                },
            ),
        )
    }

    pub fn subscription(&self) -> iced::Subscription<Message> {
        match &self.state {
            AppState::Login {
                is_loading: true, ..
            } => iced::time::every(std::time::Duration::from_secs(2)).map(|_| Message::CheckLogin),
            _ => iced::Subscription::none(),
        }
    }

    #[allow(clippy::too_many_lines)]
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ErrorEncountered(e) => {
                eprintln!("Error encountered: {e:?}");
                Task::none()
            }
            Message::LoginRequested => {
                if let AppState::Login { is_loading, .. } = &mut self.state {
                    *is_loading = true;

                    return Task::perform(
                        async { crate::api::auth::do_login_flow().await },
                        |res| match res {
                            Ok(spotify) => Message::LoginSuccess(Box::new(spotify)),
                            Err(e) => Message::LoginFailed(e.to_string()),
                        },
                    );
                }
                Task::none()
            }
            Message::CheckLogin => Task::perform(
                async { crate::api::auth::check_existing_login().await },
                |res| match res {
                    Ok(spotify) => Message::LoginSuccess(Box::new(spotify)),
                    Err(_) => Message::CheckLoginFailed,
                },
            ),
            Message::CheckLoginFailed => {
                // Do nothing, keep polling
                Task::none()
            }
            Message::LoginSuccess(spotify) => {
                // Pre-fill some mock data so the UI looks complete while we wait for backend
                let mock_playback = PlaybackState {
                    is_playing: false,
                    current_track: Some(TrackInfo {
                        title: "Neon Nights".to_string(),
                        artist: "Synthwave Architect".to_string(),
                        duration_ms: 225_000,
                    }),
                    progress_ms: 85_000,
                    volume: 0.8,
                };

                self.state = AppState::Main {
                    nav_item: NavigationItem::Home,
                    playback: mock_playback,
                };

                Task::perform(
                    async move {
                        let token_mutex = spotify.get_token();
                        let token_guard = token_mutex.lock().await.unwrap();
                        let access_token = token_guard.as_ref().unwrap().access_token.clone();
                        crate::audio::session::connect_with_token(&access_token).await
                    },
                    |res| match res {
                        Ok(audio_session) => Message::AudioSessionConnected(audio_session),
                        Err(e) => Message::ErrorEncountered(e),
                    },
                )
            }
            Message::AudioSessionConnected(_session) => {
                println!("Audio session connected successfully!");
                Task::none()
            }
            Message::LoginFailed(err) => {
                if let AppState::Login {
                    is_loading, error, ..
                } = &mut self.state
                {
                    *is_loading = false;
                    if err != "No token" {
                        *error = Some(err);
                    }
                }
                Task::none()
            }
            Message::NavigationSelected(item) => {
                if let AppState::Main { nav_item, .. } = &mut self.state {
                    *nav_item = item;
                }
                Task::none()
            }
            Message::TogglePlayback => {
                if let AppState::Main { playback, .. } = &mut self.state {
                    playback.is_playing = !playback.is_playing;

                    // Send command to audio engine for visual feedback
                    let cmd = if playback.is_playing {
                        AudioCommand::Play
                    } else {
                        AudioCommand::Pause
                    };
                    let _ = self.audio_tx.try_send(cmd);
                }
                Task::none()
            }
            Message::SkipNext | Message::SkipPrev => Task::none(),
            #[allow(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                clippy::cast_precision_loss
            )]
            Message::SeekTo(percent) => {
                if let AppState::Main { playback, .. } = &mut self.state {
                    if let Some(track) = &playback.current_track {
                        playback.progress_ms = (percent * track.duration_ms as f32) as u32;
                    }
                }
                Task::none()
            }
            Message::VolumeChanged(vol) => {
                if let AppState::Main { playback, .. } = &mut self.state {
                    playback.volume = vol;
                }
                Task::none()
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        match &self.state {
            AppState::Login { is_loading, error } => {
                login::view("", "", *is_loading, error.as_deref())
            }
            AppState::Main { nav_item, playback } => {
                crate::ui::main_layout::view(nav_item, playback)
            }
        }
    }
}
