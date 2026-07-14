use crate::audio::engine::{AudioCommand, AudioEngine};
use crate::audio::session::{AudioSession, PlayerCommand};
use crate::error::AppError;
use crate::ui::login;
use iced::{Element, Task};
use librespot::playback::player::PlayerEvent;
use rspotify::clients::BaseClient;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavigationItem {
    Home,
    #[allow(dead_code)]
    Search,
    #[allow(dead_code)]
    Library,
}

#[derive(Debug, Clone)]
pub struct TrackInfo {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_ms: u32,
}

#[derive(Debug, Clone)]
pub struct PlaybackState {
    pub is_playing: bool,
    pub current_track: Option<TrackInfo>,
    pub progress_ms: u32,
    pub volume: f32,
    pub current_track_uri: Option<String>,
}

impl Default for PlaybackState {
    fn default() -> Self {
        Self {
            is_playing: false,
            current_track: None,
            progress_ms: 0,
            volume: 1.0,
            current_track_uri: None,
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
        audio_session: Option<AudioSession>,
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
    AudioSessionConnected(AudioSession),
    PlayerEventReceived(PlayerEvent),
    // Main UI Messages
    NavigationSelected(NavigationItem),
    TogglePlayback,
    SkipNext,
    SkipPrev,
    SeekTo(f32),        // 0.0 to 1.0
    VolumeChanged(f32), // 0.0 to 1.0
    // Mock UI Actions
    MockAction,
}

struct PlayerEventsRecipe {
    events: Arc<tokio::sync::Mutex<tokio::sync::mpsc::Receiver<PlayerEvent>>>,
}

impl iced::advanced::subscription::Recipe for PlayerEventsRecipe {
    type Output = Message;

    fn hash(&self, state: &mut iced::advanced::subscription::Hasher) {
        use std::hash::Hash;
        std::any::TypeId::of::<Self>().hash(state);
        (Arc::as_ptr(&self.events) as u64).hash(state);
    }

    fn stream(
        self: Box<Self>,
        _input: iced::advanced::subscription::EventStream,
    ) -> futures::stream::BoxStream<'static, Self::Output> {
        let events = self.events;
        Box::pin(iced::stream::channel(32, async move |mut output| {
            loop {
                let maybe_event = events.lock().await.recv().await;
                match maybe_event {
                    Some(ev) => {
                        use iced::futures::SinkExt;
                        if output.send(Message::PlayerEventReceived(ev)).await.is_err() {
                            break;
                        }
                    }
                    None => break,
                }
            }
        }))
    }
}

impl App {
    pub fn new() -> (Self, Task<Message>) {
        let audio_tx = AudioEngine::spawn();

        (
            Self {
                state: AppState::Login {
                    is_loading: true,
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
            AppState::Main {
                audio_session: Some(session),
                ..
            } => iced::advanced::subscription::from_recipe(PlayerEventsRecipe {
                events: Arc::clone(&session.events),
            }),
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
            Message::CheckLoginFailed | Message::MockAction => Task::none(),
            Message::LoginSuccess(spotify) => {
                let mock_playback = PlaybackState {
                    is_playing: false,
                    current_track: Some(TrackInfo {
                        title: "Neon Nights".to_string(),
                        artist: "Synthwave Architect".to_string(),
                        album: "Neon Dreams".to_string(),
                        duration_ms: 225_000,
                    }),
                    progress_ms: 85_000,
                    volume: 0.8,
                    current_track_uri: None,
                };

                self.state = AppState::Main {
                    nav_item: NavigationItem::Home,
                    playback: mock_playback,
                    audio_session: None,
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
            Message::AudioSessionConnected(session) => {
                if let AppState::Main { audio_session, .. } = &mut self.state {
                    *audio_session = Some(session);
                }
                Task::none()
            }
            Message::PlayerEventReceived(event) => {
                match &event {
                    PlayerEvent::Playing {
                        track_id,
                        position_ms,
                        ..
                    } => {
                        if let AppState::Main { playback, .. } = &mut self.state {
                            playback.is_playing = true;
                            playback.progress_ms = *position_ms;
                            playback.current_track_uri = Some(track_id.to_uri());
                        }
                    }
                    PlayerEvent::Paused { position_ms, .. } => {
                        if let AppState::Main { playback, .. } = &mut self.state {
                            playback.is_playing = false;
                            playback.progress_ms = *position_ms;
                        }
                    }
                    PlayerEvent::TrackChanged { audio_item } => {
                        if let AppState::Main { playback, .. } = &mut self.state {
                            use librespot::metadata::audio::UniqueFields;
                            let (artist, album) = match &audio_item.unique_fields {
                                UniqueFields::Track { artists, album, .. } => {
                                    let artist_names: Vec<&str> =
                                        artists.iter().map(|a| a.name.as_str()).collect();
                                    (artist_names.join(", "), album.clone())
                                }
                                UniqueFields::Episode { show_name, .. } => {
                                    (show_name.clone(), String::new())
                                }
                                UniqueFields::Local { artists, album, .. } => (
                                    artists.clone().unwrap_or_default(),
                                    album.clone().unwrap_or_default(),
                                ),
                            };
                            playback.current_track = Some(TrackInfo {
                                title: audio_item.name.clone(),
                                artist,
                                album,
                                duration_ms: audio_item.duration_ms,
                            });
                        }
                    }
                    PlayerEvent::Stopped { .. } => {
                        if let AppState::Main { playback, .. } = &mut self.state {
                            playback.is_playing = false;
                            playback.progress_ms = 0;
                        }
                    }
                    PlayerEvent::EndOfTrack { .. } => {
                        if let AppState::Main { playback, .. } = &mut self.state {
                            playback.is_playing = false;
                        }
                    }
                    _ => {}
                }
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
                if let AppState::Main {
                    playback,
                    audio_session,
                    ..
                } = &mut self.state
                {
                    let was_playing = playback.is_playing;
                    playback.is_playing = !was_playing;

                    if let Some(session) = audio_session {
                        let cmd = if was_playing {
                            PlayerCommand::Pause
                        } else {
                            PlayerCommand::Resume
                        };
                        let _ = session.cmd_tx.try_send(cmd);
                    } else {
                        let legacy_cmd = if playback.is_playing {
                            AudioCommand::Play
                        } else {
                            AudioCommand::Pause
                        };
                        let _ = self.audio_tx.try_send(legacy_cmd);
                    }
                }
                Task::none()
            }
            Message::SkipNext => {
                if let AppState::Main {
                    audio_session: Some(session),
                    ..
                } = &mut self.state
                {
                    let _ = session.cmd_tx.try_send(PlayerCommand::SkipNext);
                }
                Task::none()
            }
            Message::SkipPrev => {
                if let AppState::Main {
                    audio_session: Some(session),
                    ..
                } = &mut self.state
                {
                    let _ = session.cmd_tx.try_send(PlayerCommand::SkipPrev);
                }
                Task::none()
            }
            #[allow(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                clippy::cast_precision_loss
            )]
            Message::SeekTo(percent) => {
                if let AppState::Main {
                    playback,
                    audio_session,
                    ..
                } = &mut self.state
                {
                    if let Some(track) = &playback.current_track {
                        let pos_ms = (percent * track.duration_ms as f32) as u32;
                        playback.progress_ms = pos_ms;

                        if let Some(session) = audio_session {
                            let _ = session.cmd_tx.try_send(PlayerCommand::Seek(pos_ms));
                        }
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
            AppState::Main {
                nav_item, playback, ..
            } => crate::ui::main_layout::view(nav_item, playback),
        }
    }
}
