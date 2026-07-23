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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum RightPanelTab {
    NowPlaying,
    Queue,
    Lyrics,
}

#[derive(Debug, Clone)]
pub struct TrackInfo {
    pub title: String,
    pub artist: String,
    #[allow(dead_code)]
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
pub struct SelectedPlaylistState {
    pub id: String,
    pub name: String,
    pub tracks: Vec<crate::api::playlist::PlaylistTrack>,
    pub is_loading: bool,
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
        user_profile: Option<crate::api::user::UserProfile>,
        user_playlists: Vec<crate::api::playlist::PlaylistSummary>,
        selected_playlist: Option<SelectedPlaylistState>,
        spotify_client: Option<Arc<rspotify::AuthCodePkceSpotify>>,
        sidebar_width: f32,
        right_panel_width: f32,
        active_right_panel: Option<RightPanelTab>,
        dragging_sidebar: bool,
        dragging_right_panel: bool,
        window_width: f32,
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
    UserProfileFetched(Result<crate::api::user::UserProfile, AppError>),
    UserPlaylistsFetched(Result<Vec<crate::api::playlist::PlaylistSummary>, AppError>),
    SelectPlaylist(String),
    PlaylistTracksFetched(
        String,
        Result<Vec<crate::api::playlist::PlaylistTrack>, AppError>,
    ),
    // Audio Messages
    AudioSessionConnected(AudioSession),
    PlayerEventReceived(PlayerEvent),
    PlaybackPositionReceived(u32),
    SessionExpired,
    // Main UI Messages
    NavigationSelected(NavigationItem),
    TogglePlayback,
    SkipNext,
    SkipPrev,
    SeekTo(f32),        // 0.0 to 1.0
    VolumeChanged(f32), // 0.0 to 1.0
    // Mock UI Actions
    MockAction,
    // Error Actions
    DismissError,
    // Panel Layout Messages
    StartSidebarDrag,
    StartRightPanelDrag,
    EndPanelDrag,
    PanelDragMoved(f32),
    ToggleRightPanel(RightPanelTab),
    WindowResized(f32),
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
                            AudioSessionEvent::SessionExpired => Message::SessionExpired,
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
            AppState::Main { audio_session, .. } => {
                let mut subs = vec![];
                if let Some(session) = audio_session {
                    subs.push(iced::advanced::subscription::from_recipe(
                        PlayerEventsRecipe {
                            events: Arc::clone(&session.events),
                        },
                    ));
                }
                subs.push(iced::event::listen().filter_map(|event| match event {
                    iced::Event::Mouse(iced::mouse::Event::CursorMoved { position }) => {
                        Some(Message::PanelDragMoved(position.x))
                    }
                    iced::Event::Mouse(iced::mouse::Event::ButtonReleased(
                        iced::mouse::Button::Left,
                    )) => Some(Message::EndPanelDrag),
                    iced::Event::Window(iced::window::Event::Resized(size)) => {
                        Some(Message::WindowResized(size.width))
                    }
                    _ => None,
                }));
                iced::Subscription::batch(subs)
            }
            AppState::Login { .. } => iced::Subscription::none(),
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

                let (sw, rw) = load_layout();

                let spotify_arc = Arc::new(*spotify);

                self.state = AppState::Main {
                    nav_item: NavigationItem::Home,
                    playback: mock_playback,
                    audio_session: None,
                    user_profile: None,
                    user_playlists: Vec::new(),
                    selected_playlist: None,
                    spotify_client: Some(Arc::clone(&spotify_arc)),
                    sidebar_width: sw,
                    right_panel_width: rw,
                    active_right_panel: None,
                    dragging_sidebar: false,
                    dragging_right_panel: false,
                    window_width: 1200.0,
                };

                let spotify_1 = Arc::clone(&spotify_arc);
                let spotify_2 = Arc::clone(&spotify_arc);
                let spotify_3 = Arc::clone(&spotify_arc);

                Task::batch([
                    Task::perform(
                        async move {
                            let token_mutex = spotify_1.get_token();
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
                    ),
                    Task::perform(
                        async move { crate::api::user::fetch_user_profile(&spotify_2).await },
                        Message::UserProfileFetched,
                    ),
                    Task::perform(
                        async move { crate::api::playlist::fetch_user_playlists(&spotify_3).await },
                        Message::UserPlaylistsFetched,
                    ),
                ])
            }
            Message::UserProfileFetched(res) => {
                if let Ok(profile) = res {
                    if let AppState::Main { user_profile, .. } = &mut self.state {
                        *user_profile = Some(profile);
                    }
                }
                Task::none()
            }
            Message::UserPlaylistsFetched(res) => {
                if let Ok(playlists) = res {
                    if let AppState::Main { user_playlists, .. } = &mut self.state {
                        *user_playlists = playlists;
                    }
                }
                Task::none()
            }
            Message::SelectPlaylist(playlist_id) => {
                if let AppState::Main {
                    user_playlists,
                    selected_playlist,
                    spotify_client,
                    ..
                } = &mut self.state
                {
                    let playlist_name = user_playlists
                        .iter()
                        .find(|p| p.id == playlist_id)
                        .map_or_else(|| "Playlist".to_string(), |p| p.name.clone());

                    *selected_playlist = Some(SelectedPlaylistState {
                        id: playlist_id.clone(),
                        name: playlist_name,
                        tracks: Vec::new(),
                        is_loading: true,
                    });

                    if let Some(client) = spotify_client.clone() {
                        let pid = playlist_id.clone();
                        return Task::perform(
                            async move {
                                let res =
                                    crate::api::playlist::fetch_playlist_tracks(&client, &pid)
                                        .await;
                                (pid, res)
                            },
                            |(pid, res)| Message::PlaylistTracksFetched(pid, res),
                        );
                    }
                }
                Task::none()
            }
            Message::PlaylistTracksFetched(playlist_id, res) => {
                if let AppState::Main {
                    selected_playlist: Some(selected),
                    ..
                } = &mut self.state
                {
                    if selected.id == playlist_id {
                        selected.is_loading = false;
                        if let Ok(tracks) = res {
                            selected.tracks = tracks;
                        }
                    }
                }
                Task::none()
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
                    PlayerEvent::Seeked { position_ms, .. } => {
                        if let AppState::Main { playback, .. } = &mut self.state {
                            playback.progress_ms = *position_ms;
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
                            playback.progress_ms = 0;
                        }
                        return self.update(Message::SkipNext);
                    }
                    _ => {}
                }
                Task::none()
            }
            Message::PlaybackPositionReceived(pos) => {
                if let AppState::Main { playback, .. } = &mut self.state {
                    if let Some(track) = &playback.current_track {
                        if track.duration_ms > 0 {
                            playback.progress_ms = pos.min(track.duration_ms);
                        } else {
                            playback.progress_ms = pos;
                        }
                    } else {
                        playback.progress_ms = pos;
                    }
                }
                Task::none()
            }
            Message::SessionExpired => {
                if let AppState::Main {
                    audio_session,
                    playback,
                    ..
                } = &mut self.state
                {
                    *audio_session = None;
                    playback.is_playing = false;
                }
                self.active_error = Some(
                    "Spotify audio session expired or disconnected. Re-connection required."
                        .to_string(),
                );
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
                        let clamped_percent = percent.clamp(0.0, 1.0);
                        let pos_ms = (clamped_percent * track.duration_ms as f32) as u32;
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
                    let clamped_vol = vol.clamp(0.0, 1.0);
                    playback.volume = clamped_vol;
                    if let Some(session) = audio_session {
                        let _ = session.cmd_tx.try_send(PlayerCommand::Volume(clamped_vol));
                    }
                }
                Task::none()
            }
            Message::StartSidebarDrag => {
                if let AppState::Main {
                    dragging_sidebar, ..
                } = &mut self.state
                {
                    *dragging_sidebar = true;
                }
                Task::none()
            }
            Message::StartRightPanelDrag => {
                if let AppState::Main {
                    dragging_right_panel,
                    ..
                } = &mut self.state
                {
                    *dragging_right_panel = true;
                }
                Task::none()
            }
            Message::EndPanelDrag => {
                if let AppState::Main {
                    dragging_sidebar,
                    dragging_right_panel,
                    sidebar_width,
                    right_panel_width,
                    ..
                } = &mut self.state
                {
                    if *dragging_sidebar || *dragging_right_panel {
                        *dragging_sidebar = false;
                        *dragging_right_panel = false;
                        let _ = save_layout(*sidebar_width, *right_panel_width);
                    }
                }
                Task::none()
            }
            Message::PanelDragMoved(x) => {
                if let AppState::Main {
                    dragging_sidebar,
                    dragging_right_panel,
                    sidebar_width,
                    right_panel_width,
                    window_width,
                    ..
                } = &mut self.state
                {
                    if *dragging_sidebar {
                        let new_w = x.clamp(80.0, 400.0);
                        *sidebar_width = if new_w < 120.0 { 80.0 } else { new_w };
                    }
                    if *dragging_right_panel {
                        let new_w = (*window_width - x).clamp(200.0, 500.0);
                        *right_panel_width = new_w;
                    }
                }
                Task::none()
            }
            Message::ToggleRightPanel(tab) => {
                if let AppState::Main {
                    active_right_panel, ..
                } = &mut self.state
                {
                    if *active_right_panel == Some(tab) {
                        *active_right_panel = None;
                    } else {
                        *active_right_panel = Some(tab);
                    }
                }
                Task::none()
            }
            Message::WindowResized(w) => {
                if let AppState::Main { window_width, .. } = &mut self.state {
                    *window_width = w;
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
                sidebar_width,
                right_panel_width,
                active_right_panel,
                user_profile,
                user_playlists,
                selected_playlist,
                ..
            } => crate::ui::main_layout::view(
                nav_item,
                playback,
                *sidebar_width,
                *right_panel_width,
                *active_right_panel,
                user_profile.as_ref(),
                user_playlists,
                selected_playlist.as_ref(),
            ),
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

pub fn save_layout(sidebar_width: f32, right_panel_width: f32) -> Result<(), std::io::Error> {
    let path = get_layout_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = File::create(path)?;
    writeln!(file, "{sidebar_width},{right_panel_width}")?;
    Ok(())
}

pub fn load_layout() -> (f32, f32) {
    let default_sidebar = 280.0;
    let default_right = 320.0;
    let path = get_layout_path();
    if !path.exists() {
        return (default_sidebar, default_right);
    }
    if let Ok(file) = File::open(path) {
        let reader = BufReader::new(file);
        if let Some(Ok(line)) = reader.lines().next() {
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() == 2 {
                if let (Ok(sw), Ok(rw)) = (parts[0].parse::<f32>(), parts[1].parse::<f32>()) {
                    return (sw, rw);
                }
            }
        }
    }
    (default_sidebar, default_right)
}
