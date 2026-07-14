use crate::audio::engine::{AudioCommand, AudioEngine};
use crate::audio::session::{AudioSession, AudioSessionEvent, PlayerCommand};
use crate::error::AppError;
use crate::ui::login;
use iced::{Element, Task};
use librespot::playback::player::PlayerEvent;
use rspotify::clients::BaseClient;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
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

#[derive(Debug, Clone)]
pub struct Card {
    pub id: String,
    pub title: String,
    pub subtitle: String,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub dragging: bool,
    pub resizing: bool,
    pub hovered: bool,
    pub drag_offset: Option<(f32, f32)>,
}

#[allow(clippy::large_enum_variant)]
pub enum AppState {
    Login {
        is_loading: bool,
        error: Option<String>,
    },
    Main {
        nav_item: NavigationItem,
        playback: PlaybackState,
        audio_session: Option<AudioSession>,
        cards: Vec<Card>,
        canvas_cache: iced::widget::canvas::Cache,
        grid_size: f32,
    },
}

pub struct App {
    pub state: AppState,
    #[allow(dead_code)]
    pub audio_tx: tokio::sync::mpsc::Sender<AudioCommand>,
    pub active_error: Option<String>,
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
    PlaybackPositionReceived(u32),
    // Main UI Messages
    NavigationSelected(NavigationItem),
    TogglePlayback,
    SkipNext,
    SkipPrev,
    SeekTo(f32),        // 0.0 to 1.0
    VolumeChanged(f32), // 0.0 to 1.0
    // Card Layout Messages
    CardPressed {
        id: String,
        is_resize: bool,
        offset_x: f32,
        offset_y: f32,
    },
    CardMoved {
        x: f32,
        y: f32,
    },
    CardReleased,
    CardHovered(Option<String>),
    // Mock UI Actions
    MockAction,
    // Error Actions
    DismissError,
    // Grid Actions
    CycleGridSize,
}

struct PlayerEventsRecipe {
    events: Arc<tokio::sync::Mutex<tokio::sync::mpsc::Receiver<AudioSessionEvent>>>,
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
                        let msg = match ev {
                            AudioSessionEvent::Player(pe) => Message::PlayerEventReceived(pe),
                            AudioSessionEvent::PositionMs(pos) => {
                                Message::PlaybackPositionReceived(pos)
                            }
                        };
                        if output.send(msg).await.is_err() {
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
                active_error: None,
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
                self.active_error = Some(e.to_string());
                Task::none()
            }
            Message::DismissError => {
                self.active_error = None;
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

                let mut default_cards = vec![
                    Card {
                        id: "liked_songs".to_string(),
                        title: "Liked Songs".to_string(),
                        subtitle: "Your favorite tracks".to_string(),
                        x: 40.0,
                        y: 40.0,
                        width: 250.0,
                        height: 180.0,
                        dragging: false,
                        resizing: false,
                        hovered: false,
                        drag_offset: None,
                    },
                    Card {
                        id: "synthwave".to_string(),
                        title: "Synthwave Architect".to_string(),
                        subtitle: "Album • Neon Dreams".to_string(),
                        x: 320.0,
                        y: 40.0,
                        width: 250.0,
                        height: 180.0,
                        dragging: false,
                        resizing: false,
                        hovered: false,
                        drag_offset: None,
                    },
                    Card {
                        id: "recently_played".to_string(),
                        title: "Recently Played".to_string(),
                        subtitle: "Carpenter Brut, The Midnight...".to_string(),
                        x: 40.0,
                        y: 260.0,
                        width: 530.0,
                        height: 220.0,
                        dragging: false,
                        resizing: false,
                        hovered: false,
                        drag_offset: None,
                    },
                ];

                let _ = load_layout(&mut default_cards);

                self.state = AppState::Main {
                    nav_item: NavigationItem::Home,
                    playback: mock_playback,
                    audio_session: None,
                    cards: default_cards,
                    canvas_cache: iced::widget::canvas::Cache::default(),
                    grid_size: 20.0,
                };

                Task::perform(
                    async move {
                        let token_mutex = spotify.get_token();
                        let token_guard = token_mutex.lock().await.map_err(|e| {
                            AppError::Auth(format!("Failed to lock token mutex: {e:?}"))
                        })?;
                        let token_ref = (*token_guard).as_ref().ok_or_else(|| {
                            AppError::Auth("No access token available".to_string())
                        })?;
                        let access_token = token_ref.access_token.clone();
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
            Message::PlaybackPositionReceived(pos) => {
                if let AppState::Main { playback, .. } = &mut self.state {
                    playback.progress_ms = pos;
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
                if let AppState::Main {
                    playback,
                    audio_session,
                    ..
                } = &mut self.state
                {
                    playback.volume = vol;
                    if let Some(session) = audio_session {
                        let _ = session.cmd_tx.try_send(PlayerCommand::Volume(vol));
                    }
                }
                Task::none()
            }
            Message::CardPressed {
                id,
                is_resize,
                offset_x,
                offset_y,
            } => {
                if let AppState::Main {
                    cards,
                    canvas_cache,
                    ..
                } = &mut self.state
                {
                    if let Some(pos) = cards.iter().position(|c| c.id == id) {
                        let mut card = cards.remove(pos);
                        card.dragging = !is_resize;
                        card.resizing = is_resize;
                        card.drag_offset = Some((offset_x, offset_y));
                        cards.push(card);
                        canvas_cache.clear();
                    }
                }
                Task::none()
            }
            Message::CardMoved { x, y } => {
                if let AppState::Main {
                    cards,
                    canvas_cache,
                    ..
                } = &mut self.state
                {
                    for card in cards.iter_mut() {
                        if card.dragging {
                            if let Some((offset_x, offset_y)) = card.drag_offset {
                                card.x = x - offset_x;
                                card.y = y - offset_y;
                                canvas_cache.clear();
                            }
                        } else if card.resizing {
                            card.width = (x - card.x).max(120.0);
                            card.height = (y - card.y).max(80.0);
                            canvas_cache.clear();
                        }
                    }
                }
                Task::none()
            }
            Message::CardReleased => {
                if let AppState::Main {
                    cards,
                    canvas_cache,
                    grid_size,
                    ..
                } = &mut self.state
                {
                    let gs = *grid_size;
                    for card in cards.iter_mut() {
                        if card.dragging {
                            card.x = (card.x / gs).round() * gs;
                            card.y = (card.y / gs).round() * gs;
                        }
                        if card.resizing {
                            card.width = ((card.width / gs).round() * gs).max(120.0);
                            card.height = ((card.height / gs).round() * gs).max(80.0);
                        }
                        card.dragging = false;
                        card.resizing = false;
                        card.drag_offset = None;
                    }
                    canvas_cache.clear();

                    // Persist the new cards layout to disk
                    if let Err(e) = save_layout(cards) {
                        eprintln!("Failed to save layout: {e}");
                    }
                }
                Task::none()
            }
            Message::CycleGridSize => {
                if let AppState::Main { grid_size, .. } = &mut self.state {
                    *grid_size = match *grid_size {
                        1.0 => 10.0,
                        10.0 => 20.0,
                        20.0 => 50.0,
                        50.0 => 1.0,
                        _ => 20.0,
                    };
                }
                Task::none()
            }
            Message::CardHovered(hovered_id) => {
                if let AppState::Main { cards, .. } = &mut self.state {
                    for card in cards.iter_mut() {
                        card.hovered = Some(card.id.clone()) == hovered_id;
                    }
                }
                Task::none()
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let content = match &self.state {
            AppState::Login { is_loading, error } => {
                login::view("", "", *is_loading, error.as_deref())
            }
            AppState::Main {
                nav_item,
                playback,
                cards,
                canvas_cache,
                grid_size,
                ..
            } => crate::ui::main_layout::view(nav_item, playback, cards, canvas_cache, *grid_size),
        };

        if let Some(err) = &self.active_error {
            use crate::ui::icons::Icon;
            use crate::ui::theme;
            use iced::widget::{Button, Column, Container, Row, Text, container};
            use iced::{Alignment, Background, Border, Length};

            let error_banner = Container::new(
                Row::new()
                    .spacing(12)
                    .align_y(Alignment::Center)
                    .push(Icon::X.view_colored(16.0, theme::TEXT_PRIMARY))
                    .push(
                        Text::new(err)
                            .size(14)
                            .color(theme::TEXT_PRIMARY)
                            .width(Length::Fill),
                    )
                    .push(
                        Button::new(Icon::X.view_colored(14.0, theme::TEXT_SECONDARY))
                            .padding(4)
                            .on_press(Message::DismissError)
                            .style(|_theme, status| {
                                let base = iced::widget::button::Style {
                                    background: Some(Background::Color(iced::Color::TRANSPARENT)),
                                    ..Default::default()
                                };
                                match status {
                                    iced::widget::button::Status::Hovered => {
                                        iced::widget::button::Style {
                                            background: Some(Background::Color(theme::SURFACE_2)),
                                            ..base
                                        }
                                    }
                                    _ => base,
                                }
                            }),
                    ),
            )
            .padding([8, 16])
            .width(Length::Fill)
            .style(|_theme| container::Style {
                background: Some(Background::Color(iced::Color {
                    r: 0.7,
                    g: 0.15,
                    b: 0.15,
                    a: 1.0,
                })),
                border: Border {
                    radius: 4.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            });

            Column::new()
                .spacing(8)
                .push(error_banner)
                .push(content)
                .into()
        } else {
            content
        }
    }
}

fn get_layout_path() -> PathBuf {
    let home =
        std::env::var("HOME").unwrap_or_else(|_| std::env::var("USERPROFILE").unwrap_or_default());
    std::path::Path::new(&home).join(".spotifust_layout")
}

pub fn save_layout(cards: &[Card]) -> Result<(), std::io::Error> {
    let path = get_layout_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = File::create(path)?;
    for card in cards {
        writeln!(
            file,
            "{},{},{},{},{}",
            card.id, card.x, card.y, card.width, card.height
        )?;
    }
    Ok(())
}

pub fn load_layout(cards: &mut [Card]) -> Result<(), std::io::Error> {
    let path = get_layout_path();
    if !path.exists() {
        return Ok(());
    }
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    for line in reader.lines() {
        let line = line?;
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() == 5 {
            let id = parts[0];
            if let (Ok(x), Ok(y), Ok(w), Ok(h)) = (
                parts[1].parse::<f32>(),
                parts[2].parse::<f32>(),
                parts[3].parse::<f32>(),
                parts[4].parse::<f32>(),
            ) {
                if let Some(card) = cards.iter_mut().find(|c| c.id == id) {
                    card.x = x;
                    card.y = y;
                    card.width = w;
                    card.height = h;
                }
            }
        }
    }
    Ok(())
}
