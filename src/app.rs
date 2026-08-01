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
    Settings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum RightPanelTab {
    NowPlaying,
    Queue,
    Lyrics,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SidebarFilter {
    #[default]
    All,
    Playlists,
    Albums,
}

#[derive(Debug, Clone)]
pub struct SelectedAlbumState {
    pub id: String,
    pub name: String,
    pub artist_name: String,
    pub image_url: Option<String>,
    pub release_date: String,
    pub tracks: Vec<crate::api::album::AlbumDetailTrack>,
    pub is_loading: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TrackInfo {
    pub title: String,
    pub artist: String,
    #[allow(dead_code)]
    pub album: String,
    pub duration_ms: u32,
    pub image_url: Option<String>,
    pub uri: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LastPlaybackState {
    pub track: TrackInfo,
    pub track_uri: String,
    pub progress_ms: u32,
}

pub fn save_last_playback_state(playback: &PlaybackState) {
    if let (Some(track), Some(uri)) = (&playback.current_track, &playback.current_track_uri) {
        let state = LastPlaybackState {
            track: track.clone(),
            track_uri: uri.clone(),
            progress_ms: playback.progress_ms,
        };
        let _ = crate::api::cache::DiskMetadataCache::save("last_playback_state", &state);
    }
}

pub fn save_saved_volume(volume: f32) {
    let _ = crate::api::cache::DiskMetadataCache::save("saved_volume", &volume);
}

#[must_use]
pub fn load_saved_volume() -> f32 {
    crate::api::cache::DiskMetadataCache::load::<f32>("saved_volume").unwrap_or(0.8)
}

pub fn load_last_playback_state(playback: &mut PlaybackState) {
    let saved_vol = load_saved_volume();
    playback.volume = saved_vol;
    playback.last_volume = saved_vol;
    if let Some(state) =
        crate::api::cache::DiskMetadataCache::load::<LastPlaybackState>("last_playback_state")
    {
        playback.current_track = Some(state.track);
        playback.current_track_uri = Some(state.track_uri);
        playback.progress_ms = state.progress_ms;
        playback.is_playing = false;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RepeatMode {
    #[default]
    Off,
    Context,
    One,
}

#[derive(Debug, Clone)]
pub struct PlaybackState {
    pub is_playing: bool,
    pub current_track: Option<TrackInfo>,
    pub progress_ms: u32,
    pub volume: f32,
    pub current_track_uri: Option<String>,
    pub is_muted: bool,
    pub last_volume: f32,
    pub is_shuffled: bool,
    pub repeat_mode: RepeatMode,
}

impl Default for PlaybackState {
    fn default() -> Self {
        let vol = load_saved_volume();
        Self {
            is_playing: false,
            current_track: None,
            progress_ms: 0,
            volume: vol,
            current_track_uri: None,
            is_muted: false,
            last_volume: vol,
            is_shuffled: false,
            repeat_mode: RepeatMode::Off,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SelectedPlaylistState {
    pub id: String,
    pub name: String,
    pub image_url: Option<String>,
    pub tracks: Vec<crate::api::playlist::PlaylistTrack>,
    pub is_loading: bool,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum ContextMenuTarget {
    Track {
        track: TrackInfo,
        from_playlist_id: Option<String>,
    },
    Album(crate::api::album::AlbumSummary),
    Artist {
        artist_id: String,
        artist_name: String,
    },
    Playlist(crate::api::playlist::PlaylistSummary),
}

#[derive(Debug, Clone)]
pub struct ContextMenuState {
    pub target: ContextMenuTarget,
    pub position: iced::Point,
}

#[derive(Debug, Clone)]
pub enum ActiveModal {
    AddToPlaylist {
        track_uris: Vec<String>,
        search_query: String,
    },
    EditPlaylist {
        playlist_id: String,
        name_input: String,
        description_input: String,
    },
    ConfirmDeletePlaylist {
        playlist_id: String,
        playlist_name: String,
    },
    CopyPlaylistToAnother {
        source_playlist_id: String,
        source_playlist_name: String,
        search_query: String,
    },
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
        user_albums: Vec<crate::api::album::AlbumSummary>,
        user_top_tracks: Vec<crate::api::tracks::TopTrack>,
        featured_playlists: Vec<crate::api::playlist::PlaylistSummary>,
        featured_albums: Vec<crate::api::album::AlbumSummary>,
        search_query: String,
        search_results: crate::api::search::SearchResults,
        is_searching: bool,
        sidebar_filter: SidebarFilter,
        selected_playlist: Option<SelectedPlaylistState>,
        selected_album: Option<SelectedAlbumState>,
        user_queue: Vec<TrackInfo>,
        context_queue: Vec<TrackInfo>,
        original_context_queue: Vec<TrackInfo>,
        context_index: usize,
        history: Vec<TrackInfo>,
        active_context_menu: Option<ContextMenuState>,
        active_modal: Option<ActiveModal>,
        toast_notification: Option<String>,
        loaded_images: std::collections::HashMap<String, iced::widget::image::Handle>,
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
#[allow(dead_code)]
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
    UserAlbumsFetched(Result<Vec<crate::api::album::AlbumSummary>, AppError>),
    UserTopTracksFetched(Result<Vec<crate::api::tracks::TopTrack>, AppError>),
    FeaturedPlaylistsFetched(Result<Vec<crate::api::playlist::PlaylistSummary>, AppError>),
    NewReleasesFetched(Result<Vec<crate::api::album::AlbumSummary>, AppError>),
    CurrentlyPlayingFetched(Result<Option<crate::api::tracks::CurrentlyPlayingInfo>, AppError>),
    SearchInputChanged(String),
    SearchResultsFetched(Result<crate::api::search::SearchResults, AppError>),
    SelectPlaylist(String),
    PlaylistTracksFetched(
        String,
        Result<Vec<crate::api::playlist::PlaylistTrack>, AppError>,
    ),
    SelectAlbum(String),
    SelectArtist(String),
    AlbumDetailsFetched(String, Result<crate::api::album::AlbumDetail, AppError>),
    PlayTrack(String),
    SidebarFilterSelected(SidebarFilter),
    ImageLoaded(Result<(String, Vec<u8>), AppError>),
    ClearSelection,
    // Audio Messages
    AudioSessionConnected(AudioSession),
    PlayerEventReceived(PlayerEvent),
    PlaybackPositionReceived(u32),
    PlaybackTick,
    SessionExpired,
    // Context Menu & Modal Messages
    OpenTrackContextMenu {
        track: TrackInfo,
        from_playlist_id: Option<String>,
        position: iced::Point,
    },
    OpenAlbumContextMenu {
        album: crate::api::album::AlbumSummary,
        position: iced::Point,
    },
    OpenPlaylistContextMenu {
        playlist: crate::api::playlist::PlaylistSummary,
        position: iced::Point,
    },
    OpenArtistContextMenu {
        artist_id: String,
        artist_name: String,
        position: iced::Point,
    },
    CloseContextMenu,
    CopyShareLink(String, String),
    OpenAddToPlaylistModal(Vec<String>),
    OpenEditPlaylistModal(String, String, String),
    OpenConfirmDeletePlaylistModal(String, String),
    OpenCopyPlaylistModal(String, String),
    CloseModal,
    ModalSearchInputChanged(String),
    ModalNameInputChanged(String),
    ModalDescInputChanged(String),
    AddTracksToPlaylistAction(String, Vec<String>),
    RemoveTrackFromCurrentPlaylist(String, String),
    SaveAlbumToggle(String, bool),
    SavePlaylistDetailsAction(String, String, String),
    DeletePlaylistConfirmed(String),
    TogglePlaylistPrivacy(String, bool),
    CopyPlaylistTracksAction(String, String),
    FollowArtistToggle(String, bool),
    OpenQueuePanel,
    ShowToast(String),
    DismissToast,
    OperationFinished(Result<String, AppError>),
    // Main UI Messages
    NavigationSelected(NavigationItem),
    TogglePlayback,
    SkipNext,
    SkipPrev,
    SeekTo(f32),        // 0.0 to 1.0
    VolumeChanged(f32), // 0.0 to 1.0
    AdjustVolume(f32),  // relative delta e.g. +0.05 / -0.05
    ToggleMute,
    ToggleShuffle,
    ToggleRepeat,
    AddToQueue(TrackInfo),
    RemoveFromQueue(usize),
    MoveQueueItemUp(usize),
    MoveQueueItemDown(usize),
    PlayQueueIndex(usize),
    ClearQueue,
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
            AppState::Main {
                audio_session,
                playback,
                ..
            } => {
                let mut subs = vec![];
                if playback.is_playing {
                    subs.push(
                        iced::time::every(std::time::Duration::from_millis(200))
                            .map(|_| Message::PlaybackTick),
                    );
                }
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
                    iced::Event::Keyboard(iced::keyboard::Event::KeyPressed { key, .. }) => {
                        match key {
                            iced::keyboard::Key::Named(iced::keyboard::key::Named::Space) => {
                                Some(Message::TogglePlayback)
                            }
                            iced::keyboard::Key::Named(iced::keyboard::key::Named::ArrowRight) => {
                                Some(Message::SkipNext)
                            }
                            iced::keyboard::Key::Named(iced::keyboard::key::Named::ArrowLeft) => {
                                Some(Message::SkipPrev)
                            }
                            iced::keyboard::Key::Named(iced::keyboard::key::Named::ArrowUp) => {
                                Some(Message::AdjustVolume(0.05))
                            }
                            iced::keyboard::Key::Named(iced::keyboard::key::Named::ArrowDown) => {
                                Some(Message::AdjustVolume(-0.05))
                            }
                            _ => None,
                        }
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
            Message::CheckLoginFailed | Message::MockAction | Message::PlaybackTick => Task::none(),

            Message::LoginSuccess(spotify) => {
                let mut initial_playback = PlaybackState::default();
                load_last_playback_state(&mut initial_playback);

                let (sw, rw) = load_layout();
                let spotify_arc = Arc::new(*spotify);

                let cached_playlists = crate::api::cache::DiskMetadataCache::load::<
                    Vec<crate::api::playlist::PlaylistSummary>,
                >("user_playlists")
                .unwrap_or_default();
                let cached_albums = crate::api::cache::DiskMetadataCache::load::<
                    Vec<crate::api::album::AlbumSummary>,
                >("user_albums")
                .unwrap_or_default();
                let cached_top_tracks = crate::api::cache::DiskMetadataCache::load::<
                    Vec<crate::api::tracks::TopTrack>,
                >("user_top_tracks")
                .unwrap_or_default();
                let cached_featured_playlists = crate::api::cache::DiskMetadataCache::load::<
                    Vec<crate::api::playlist::PlaylistSummary>,
                >("featured_playlists")
                .unwrap_or_default();
                let cached_featured_albums = crate::api::cache::DiskMetadataCache::load::<
                    Vec<crate::api::album::AlbumSummary>,
                >("featured_albums")
                .unwrap_or_default();
                let cached_profile = crate::api::cache::DiskMetadataCache::load::<
                    crate::api::user::UserProfile,
                >("user_profile");

                self.state = AppState::Main {
                    nav_item: NavigationItem::Home,
                    playback: initial_playback,
                    audio_session: None,
                    user_profile: cached_profile,
                    user_playlists: cached_playlists,
                    user_albums: cached_albums,
                    user_top_tracks: cached_top_tracks,
                    featured_playlists: cached_featured_playlists,
                    featured_albums: cached_featured_albums,
                    search_query: String::new(),
                    search_results: crate::api::search::SearchResults::default(),
                    is_searching: false,
                    sidebar_filter: SidebarFilter::All,
                    selected_playlist: None,
                    selected_album: None,
                    user_queue: Vec::new(),
                    context_queue: Vec::new(),
                    original_context_queue: Vec::new(),
                    context_index: 0,
                    history: Vec::new(),
                    active_context_menu: None,
                    active_modal: None,
                    toast_notification: None,
                    loaded_images: std::collections::HashMap::new(),
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
                let spotify_4 = Arc::clone(&spotify_arc);
                let spotify_5 = Arc::clone(&spotify_arc);
                let spotify_6 = Arc::clone(&spotify_arc);
                let spotify_7 = Arc::clone(&spotify_arc);
                let spotify_8 = Arc::clone(&spotify_arc);

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
                    Task::perform(
                        async move { crate::api::album::fetch_saved_albums(&spotify_4).await },
                        Message::UserAlbumsFetched,
                    ),
                    Task::perform(
                        async move { crate::api::tracks::fetch_top_tracks(&spotify_5).await },
                        Message::UserTopTracksFetched,
                    ),
                    Task::perform(
                        async move { crate::api::playlist::fetch_featured_playlists(&spotify_6).await },
                        Message::FeaturedPlaylistsFetched,
                    ),
                    Task::perform(
                        async move { crate::api::album::fetch_new_releases(&spotify_7).await },
                        Message::NewReleasesFetched,
                    ),
                    Task::perform(
                        async move { crate::api::tracks::fetch_currently_playing(&spotify_8).await },
                        Message::CurrentlyPlayingFetched,
                    ),
                ])
            }
            Message::FeaturedPlaylistsFetched(res) => {
                let mut tasks = Vec::new();
                if let Ok(playlists) = res {
                    let _ = crate::api::cache::DiskMetadataCache::save(
                        "featured_playlists",
                        &playlists,
                    );
                    if let AppState::Main {
                        featured_playlists,
                        loaded_images,
                        ..
                    } = &mut self.state
                    {
                        tasks.extend(load_image_tasks(
                            playlists.iter().map(|p| p.image_url.clone()),
                            loaded_images,
                        ));
                        *featured_playlists = playlists;
                    }
                }
                if tasks.is_empty() {
                    Task::none()
                } else {
                    Task::batch(tasks)
                }
            }
            Message::NewReleasesFetched(res) => {
                let mut tasks = Vec::new();
                if let Ok(albums) = res {
                    let _ = crate::api::cache::DiskMetadataCache::save("featured_albums", &albums);
                    if let AppState::Main {
                        featured_albums,
                        loaded_images,
                        ..
                    } = &mut self.state
                    {
                        tasks.extend(load_image_tasks(
                            albums.iter().map(|a| a.image_url.clone()),
                            loaded_images,
                        ));
                        *featured_albums = albums;
                    }
                }
                if tasks.is_empty() {
                    Task::none()
                } else {
                    Task::batch(tasks)
                }
            }
            Message::UserProfileFetched(res) => {
                let mut tasks = Vec::new();
                if let Ok(profile) = res {
                    let _ = crate::api::cache::DiskMetadataCache::save("user_profile", &profile);
                    if let AppState::Main {
                        user_profile,
                        loaded_images,
                        ..
                    } = &mut self.state
                    {
                        tasks.extend(load_image_tasks(
                            std::iter::once(profile.avatar_url.clone()),
                            loaded_images,
                        ));
                        *user_profile = Some(profile);
                    }
                }
                if tasks.is_empty() {
                    Task::none()
                } else {
                    Task::batch(tasks)
                }
            }
            Message::UserPlaylistsFetched(res) => {
                let mut tasks = Vec::new();
                if let Ok(playlists) = res {
                    let _ =
                        crate::api::cache::DiskMetadataCache::save("user_playlists", &playlists);
                    if let AppState::Main {
                        user_playlists,
                        loaded_images,
                        ..
                    } = &mut self.state
                    {
                        tasks.extend(load_image_tasks(
                            playlists.iter().map(|p| p.image_url.clone()),
                            loaded_images,
                        ));
                        *user_playlists = playlists;
                    }
                }
                if tasks.is_empty() {
                    Task::none()
                } else {
                    Task::batch(tasks)
                }
            }
            Message::UserAlbumsFetched(res) => {
                let mut tasks = Vec::new();
                if let Ok(albums) = res {
                    let _ = crate::api::cache::DiskMetadataCache::save("user_albums", &albums);
                    if let AppState::Main {
                        user_albums,
                        loaded_images,
                        ..
                    } = &mut self.state
                    {
                        tasks.extend(load_image_tasks(
                            albums.iter().map(|a| a.image_url.clone()),
                            loaded_images,
                        ));
                        *user_albums = albums;
                    }
                }
                if tasks.is_empty() {
                    Task::none()
                } else {
                    Task::batch(tasks)
                }
            }
            Message::UserTopTracksFetched(res) => {
                let mut tasks = Vec::new();
                if let Ok(tracks) = res {
                    let _ = crate::api::cache::DiskMetadataCache::save("user_top_tracks", &tracks);
                    if let AppState::Main {
                        user_top_tracks,
                        loaded_images,
                        ..
                    } = &mut self.state
                    {
                        tasks.extend(load_image_tasks(
                            tracks.iter().map(|t| t.image_url.clone()),
                            loaded_images,
                        ));
                        *user_top_tracks = tracks;
                    }
                }
                if tasks.is_empty() {
                    Task::none()
                } else {
                    Task::batch(tasks)
                }
            }
            Message::CurrentlyPlayingFetched(res) => {
                let mut tasks = Vec::new();
                if let Ok(Some(info)) = res {
                    if let AppState::Main {
                        playback,
                        loaded_images,
                        ..
                    } = &mut self.state
                    {
                        tasks.extend(load_image_tasks(
                            std::iter::once(info.image_url.clone()),
                            loaded_images,
                        ));
                        playback.current_track = Some(TrackInfo {
                            title: info.title,
                            artist: info.artist,
                            album: info.album,
                            duration_ms: info.duration_ms,
                            image_url: info.image_url,
                            uri: info.uri.clone(),
                        });
                        playback.progress_ms = info.progress_ms;
                        playback.is_playing = info.is_playing;
                        playback.current_track_uri = Some(info.uri);
                    }
                }
                if tasks.is_empty() {
                    Task::none()
                } else {
                    Task::batch(tasks)
                }
            }
            Message::SearchInputChanged(query) => {
                if let AppState::Main {
                    search_query,
                    search_results,
                    is_searching,
                    spotify_client,
                    nav_item,
                    ..
                } = &mut self.state
                {
                    search_query.clone_from(&query);
                    *nav_item = NavigationItem::Search;

                    if query.trim().is_empty() {
                        *search_results = crate::api::search::SearchResults::default();
                        *is_searching = false;
                    } else {
                        *is_searching = true;
                        if let Some(client) = spotify_client.clone() {
                            let q = query;
                            return Task::perform(
                                async move { crate::api::search::execute_search(&client, &q).await },
                                Message::SearchResultsFetched,
                            );
                        }
                    }
                }
                Task::none()
            }
            Message::SearchResultsFetched(res) => {
                let mut tasks = Vec::new();
                if let AppState::Main {
                    search_results,
                    is_searching,
                    loaded_images,
                    ..
                } = &mut self.state
                {
                    *is_searching = false;
                    if let Ok(results) = res {
                        tasks.extend(load_image_tasks(
                            results
                                .tracks
                                .iter()
                                .map(|t| t.image_url.clone())
                                .chain(results.albums.iter().map(|a| a.image_url.clone()))
                                .chain(results.artists.iter().map(|a| a.image_url.clone())),
                            loaded_images,
                        ));
                        *search_results = results;
                    }
                }
                if tasks.is_empty() {
                    Task::none()
                } else {
                    Task::batch(tasks)
                }
            }
            Message::SelectPlaylist(playlist_id) => {
                if let AppState::Main {
                    user_playlists,
                    selected_playlist,
                    selected_album,
                    loaded_images,
                    spotify_client,
                    nav_item,
                    ..
                } = &mut self.state
                {
                    *nav_item = NavigationItem::Home;
                    *selected_album = None;
                    let (playlist_name, image_url) = user_playlists
                        .iter()
                        .find(|p| p.id == playlist_id)
                        .map_or_else(
                            || ("Playlist".to_string(), None),
                            |p| (p.name.clone(), p.image_url.clone()),
                        );

                    *selected_playlist = Some(SelectedPlaylistState {
                        id: playlist_id.clone(),
                        name: playlist_name,
                        image_url: image_url.clone(),
                        tracks: Vec::new(),
                        is_loading: true,
                    });

                    let mut tasks = load_image_tasks(std::iter::once(image_url), loaded_images);

                    if let Some(client) = spotify_client.clone() {
                        let pid = playlist_id.clone();
                        tasks.push(Task::perform(
                            async move {
                                let res =
                                    crate::api::playlist::fetch_playlist_tracks(&client, &pid)
                                        .await;
                                (pid, res)
                            },
                            |(pid, res)| Message::PlaylistTracksFetched(pid, res),
                        ));
                    }
                    if !tasks.is_empty() {
                        return Task::batch(tasks);
                    }
                }
                Task::none()
            }
            Message::PlaylistTracksFetched(playlist_id, res) => {
                let mut tasks = Vec::new();
                if let AppState::Main {
                    selected_playlist: Some(selected),
                    loaded_images,
                    ..
                } = &mut self.state
                {
                    if selected.id == playlist_id {
                        selected.is_loading = false;
                        if let Ok(mut tracks) = res {
                            crate::api::local_files::match_and_persist_local_tracks(&mut tracks);

                            tasks.extend(load_image_tasks(
                                tracks.iter().map(|t| t.image_url.clone()),
                                loaded_images,
                            ));
                            selected.tracks = tracks;
                        }
                    }
                }
                if tasks.is_empty() {
                    Task::none()
                } else {
                    Task::batch(tasks)
                }
            }
            Message::SelectAlbum(album_id) => {
                if let AppState::Main {
                    user_albums,
                    selected_album,
                    selected_playlist,
                    spotify_client,
                    nav_item,
                    ..
                } = &mut self.state
                {
                    *nav_item = NavigationItem::Home;
                    *selected_playlist = None;
                    let (name, artist, image_url, release_date) =
                        user_albums.iter().find(|a| a.id == album_id).map_or_else(
                            || ("Album".to_string(), String::new(), None, String::new()),
                            |a| {
                                (
                                    a.name.clone(),
                                    a.artist_name.clone(),
                                    a.image_url.clone(),
                                    a.release_date.clone(),
                                )
                            },
                        );

                    *selected_album = Some(SelectedAlbumState {
                        id: album_id.clone(),
                        name,
                        artist_name: artist,
                        image_url,
                        release_date,
                        tracks: Vec::new(),
                        is_loading: true,
                    });

                    if let Some(client) = spotify_client.clone() {
                        let aid = album_id.clone();
                        return Task::perform(
                            async move {
                                let res =
                                    crate::api::album::fetch_album_details(&client, &aid).await;
                                (aid, res)
                            },
                            |(aid, res)| Message::AlbumDetailsFetched(aid, res),
                        );
                    }
                }
                Task::none()
            }
            Message::SelectArtist(artist_name) => {
                self.update(Message::SearchInputChanged(artist_name))
            }
            Message::AlbumDetailsFetched(album_id, res) => {
                let mut tasks = Vec::new();
                if let AppState::Main {
                    selected_album: Some(selected),
                    loaded_images,
                    ..
                } = &mut self.state
                {
                    if selected.id == album_id {
                        selected.is_loading = false;
                        if let Ok(detail) = res {
                            selected.name = detail.name;
                            selected.artist_name = detail.artist_name;
                            selected.image_url.clone_from(&detail.image_url);
                            selected.release_date = detail.release_date;
                            selected.tracks = detail.tracks;
                            tasks.extend(load_image_tasks(
                                std::iter::once(selected.image_url.clone()),
                                loaded_images,
                            ));
                        }
                    }
                }
                if tasks.is_empty() {
                    Task::none()
                } else {
                    Task::batch(tasks)
                }
            }
            Message::PlayTrack(uri) => {
                if let AppState::Main {
                    audio_session,
                    playback,
                    user_top_tracks,
                    selected_playlist,
                    selected_album,
                    search_results,
                    loaded_images,
                    context_queue,
                    original_context_queue,
                    context_index,
                    history,
                    ..
                } = &mut self.state
                {
                    if let Some(curr) = playback.current_track.clone() {
                        history.push(curr);
                    }

                    playback.current_track_uri = Some(uri.clone());
                    playback.is_playing = true;
                    playback.progress_ms = 0;

                    let mut found_info: Option<TrackInfo> = None;

                    if let Some(sp) = selected_playlist {
                        let new_ctx: Vec<TrackInfo> = sp
                            .tracks
                            .iter()
                            .map(|t| TrackInfo {
                                title: t.title.clone(),
                                artist: t.artist.clone(),
                                album: t.album.clone(),
                                duration_ms: t.duration_ms,
                                image_url: t.image_url.clone(),
                                uri: t.uri.clone(),
                            })
                            .collect();
                        if let Some(idx) = new_ctx.iter().position(|t| t.uri == uri) {
                            *context_index = idx;
                            found_info = Some(new_ctx[idx].clone());
                            original_context_queue.clone_from(&new_ctx);
                            *context_queue = new_ctx;
                            if playback.is_shuffled && *context_index + 1 < context_queue.len() {
                                shuffle_slice(&mut context_queue[*context_index + 1..]);
                            }
                        }
                    } else if let Some(sa) = selected_album {
                        let new_ctx: Vec<TrackInfo> = sa
                            .tracks
                            .iter()
                            .map(|t| TrackInfo {
                                title: t.title.clone(),
                                artist: t.artist.clone(),
                                album: sa.name.clone(),
                                duration_ms: t.duration_ms,
                                image_url: sa.image_url.clone(),
                                uri: t.uri.clone(),
                            })
                            .collect();
                        if let Some(idx) = new_ctx.iter().position(|t| t.uri == uri) {
                            *context_index = idx;
                            found_info = Some(new_ctx[idx].clone());
                            original_context_queue.clone_from(&new_ctx);
                            *context_queue = new_ctx;
                            if playback.is_shuffled && *context_index + 1 < context_queue.len() {
                                shuffle_slice(&mut context_queue[*context_index + 1..]);
                            }
                        }
                    }

                    if found_info.is_none() {
                        if let Some(t) = user_top_tracks.iter().find(|t| t.uri == uri) {
                            found_info = Some(TrackInfo {
                                title: t.title.clone(),
                                artist: t.artist.clone(),
                                album: t.album.clone(),
                                duration_ms: t.duration_ms,
                                image_url: t.image_url.clone(),
                                uri: uri.clone(),
                            });
                        } else if let Some(t) = search_results.tracks.iter().find(|t| t.uri == uri)
                        {
                            found_info = Some(TrackInfo {
                                title: t.title.clone(),
                                artist: t.artist.clone(),
                                album: t.album.clone(),
                                duration_ms: t.duration_ms,
                                image_url: t.image_url.clone(),
                                uri: uri.clone(),
                            });
                        }
                    }

                    let mut tasks = Vec::new();
                    if let Some(info) = found_info {
                        if let Some(ref img_url) = info.image_url {
                            tasks.extend(load_image_tasks(
                                std::iter::once(Some(img_url.clone())),
                                loaded_images,
                            ));
                        }
                        playback.current_track = Some(info);
                    }

                    if let Some(session) = audio_session {
                        let _ = session.cmd_tx.try_send(PlayerCommand::Play(uri));
                    }

                    if !tasks.is_empty() {
                        return Task::batch(tasks);
                    }
                }
                Task::none()
            }
            Message::SidebarFilterSelected(filter) => {
                if let AppState::Main { sidebar_filter, .. } = &mut self.state {
                    *sidebar_filter = filter;
                }
                Task::none()
            }
            Message::ImageLoaded(res) => {
                if let Ok((url, bytes)) = res {
                    if let AppState::Main { loaded_images, .. } = &mut self.state {
                        if loaded_images.len() >= 64 {
                            if let Some(key_to_remove) = loaded_images.keys().next().cloned() {
                                loaded_images.remove(&key_to_remove);
                            }
                        }
                        loaded_images.insert(url, iced::widget::image::Handle::from_bytes(bytes));
                    }
                }
                Task::none()
            }
            Message::ClearSelection => {
                if let AppState::Main {
                    selected_playlist,
                    selected_album,
                    ..
                } = &mut self.state
                {
                    *selected_playlist = None;
                    *selected_album = None;
                }
                Task::none()
            }
            Message::AudioSessionConnected(session) => {
                if let AppState::Main {
                    audio_session,
                    playback,
                    ..
                } = &mut self.state
                {
                    let vol = if playback.is_muted {
                        0.0
                    } else {
                        playback.volume
                    };
                    let _ = session.cmd_tx.try_send(PlayerCommand::Volume(vol));
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
                            save_last_playback_state(playback);
                        }
                    }
                    PlayerEvent::TrackChanged { audio_item } => {
                        let mut tasks = Vec::new();
                        if let AppState::Main {
                            playback,
                            user_top_tracks,
                            selected_playlist,
                            selected_album,
                            search_results,
                            loaded_images,
                            ..
                        } = &mut self.state
                        {
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

                            let mut image_url = playback
                                .current_track
                                .as_ref()
                                .and_then(|t| t.image_url.clone());

                            if image_url.is_none() {
                                if let Some(ref uri) = playback.current_track_uri {
                                    if let Some(t) = user_top_tracks.iter().find(|t| &t.uri == uri)
                                    {
                                        image_url.clone_from(&t.image_url);
                                    } else if let Some(sp) = selected_playlist {
                                        if let Some(t) = sp.tracks.iter().find(|t| &t.uri == uri) {
                                            image_url.clone_from(&t.image_url);
                                        }
                                    } else if let Some(sa) = selected_album {
                                        if sa.tracks.iter().any(|t| &t.uri == uri) {
                                            image_url.clone_from(&sa.image_url);
                                        }
                                    } else if let Some(t) =
                                        search_results.tracks.iter().find(|t| &t.uri == uri)
                                    {
                                        image_url.clone_from(&t.image_url);
                                    }
                                }
                            }

                            if let Some(ref img_url) = image_url {
                                tasks.extend(load_image_tasks(
                                    std::iter::once(Some(img_url.clone())),
                                    loaded_images,
                                ));
                            }

                            playback.current_track = Some(TrackInfo {
                                title: audio_item.name.clone(),
                                artist,
                                album,
                                duration_ms: audio_item.duration_ms,
                                image_url,
                                uri: playback.current_track_uri.clone().unwrap_or_default(),
                            });
                        }
                        if !tasks.is_empty() {
                            return Task::batch(tasks);
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
                    let max_dur = playback.current_track.as_ref().map_or(u32::MAX, |t| {
                        if t.duration_ms > 0 {
                            t.duration_ms
                        } else {
                            u32::MAX
                        }
                    });
                    playback.progress_ms = pos.min(max_dur);
                    if playback.is_playing && playback.progress_ms % 4000 < 500 {
                        save_last_playback_state(playback);
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
                if let AppState::Main {
                    nav_item,
                    selected_playlist,
                    selected_album,
                    ..
                } = &mut self.state
                {
                    *nav_item = item;
                    if item == NavigationItem::Home {
                        *selected_playlist = None;
                        *selected_album = None;
                    }
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
                    user_queue,
                    context_queue,
                    context_index,
                    history,
                    playback,
                    audio_session,
                    loaded_images,
                    ..
                } = &mut self.state
                {
                    if let Some(curr) = playback.current_track.clone() {
                        history.push(curr);
                    }

                    let next_track_opt = if !user_queue.is_empty() {
                        Some(user_queue.remove(0))
                    } else if playback.repeat_mode == RepeatMode::One
                        && playback.current_track.is_some()
                    {
                        playback.current_track.clone()
                    } else if *context_index + 1 < context_queue.len() {
                        *context_index += 1;
                        Some(context_queue[*context_index].clone())
                    } else if playback.repeat_mode == RepeatMode::Context
                        && !context_queue.is_empty()
                    {
                        *context_index = 0;
                        Some(context_queue[0].clone())
                    } else {
                        None
                    };

                    if let Some(next_track) = next_track_opt {
                        playback.current_track = Some(next_track.clone());
                        playback.progress_ms = 0;
                        playback.is_playing = true;
                        if let Some(session) = audio_session {
                            let _ = session
                                .cmd_tx
                                .try_send(PlayerCommand::Play(next_track.uri.clone()));
                        }
                        if let Some(ref img) = next_track.image_url {
                            return Task::batch(load_image_tasks(
                                std::iter::once(Some(img.clone())),
                                loaded_images,
                            ));
                        }
                    } else {
                        playback.is_playing = false;
                        playback.progress_ms = 0;
                    }
                }
                Task::none()
            }
            Message::SkipPrev => {
                if let AppState::Main {
                    history,
                    context_queue,
                    context_index,
                    playback,
                    audio_session,
                    loaded_images,
                    ..
                } = &mut self.state
                {
                    if playback.progress_ms > 3000 {
                        playback.progress_ms = 0;
                        if let Some(session) = audio_session {
                            let _ = session.cmd_tx.try_send(PlayerCommand::Seek(0));
                        }
                    } else if let Some(prev_track) = history.pop() {
                        playback.current_track = Some(prev_track.clone());
                        playback.progress_ms = 0;
                        playback.is_playing = true;
                        if let Some(session) = audio_session {
                            let _ = session
                                .cmd_tx
                                .try_send(PlayerCommand::Play(prev_track.uri.clone()));
                        }
                        if let Some(ref img) = prev_track.image_url {
                            return Task::batch(load_image_tasks(
                                std::iter::once(Some(img.clone())),
                                loaded_images,
                            ));
                        }
                    } else if *context_index > 0 && *context_index < context_queue.len() {
                        *context_index -= 1;
                        let prev_track = context_queue[*context_index].clone();
                        playback.current_track = Some(prev_track.clone());
                        playback.progress_ms = 0;
                        playback.is_playing = true;
                        if let Some(session) = audio_session {
                            let _ = session
                                .cmd_tx
                                .try_send(PlayerCommand::Play(prev_track.uri.clone()));
                        }
                        if let Some(ref img) = prev_track.image_url {
                            return Task::batch(load_image_tasks(
                                std::iter::once(Some(img.clone())),
                                loaded_images,
                            ));
                        }
                    } else {
                        playback.progress_ms = 0;
                        if let Some(session) = audio_session {
                            let _ = session.cmd_tx.try_send(PlayerCommand::Seek(0));
                        }
                    }
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
                    if clamped_vol > 0.0 {
                        playback.is_muted = false;
                        playback.last_volume = clamped_vol;
                    }
                    save_saved_volume(clamped_vol);
                    if let Some(session) = audio_session {
                        let _ = session.cmd_tx.try_send(PlayerCommand::Volume(clamped_vol));
                    }
                }
                Task::none()
            }
            Message::AdjustVolume(delta) => {
                if let AppState::Main {
                    playback,
                    audio_session,
                    ..
                } = &mut self.state
                {
                    let new_vol = (playback.volume + delta).clamp(0.0, 1.0);
                    playback.volume = new_vol;
                    if new_vol > 0.0 {
                        playback.is_muted = false;
                        playback.last_volume = new_vol;
                    }
                    save_saved_volume(new_vol);
                    if let Some(session) = audio_session {
                        let _ = session.cmd_tx.try_send(PlayerCommand::Volume(new_vol));
                    }
                }
                Task::none()
            }
            Message::ToggleMute => {
                if let AppState::Main {
                    playback,
                    audio_session,
                    ..
                } = &mut self.state
                {
                    if playback.is_muted || playback.volume == 0.0 {
                        playback.is_muted = false;
                        let target_vol = if playback.last_volume <= 0.01 {
                            0.8
                        } else {
                            playback.last_volume
                        };
                        playback.volume = target_vol;
                        save_saved_volume(target_vol);
                        if let Some(session) = audio_session {
                            let _ = session.cmd_tx.try_send(PlayerCommand::Volume(target_vol));
                        }
                    } else {
                        playback.is_muted = true;
                        playback.last_volume = playback.volume;
                        playback.volume = 0.0;
                        save_saved_volume(0.0);
                        if let Some(session) = audio_session {
                            let _ = session.cmd_tx.try_send(PlayerCommand::Volume(0.0));
                        }
                    }
                }
                Task::none()
            }
            Message::OpenTrackContextMenu {
                track,
                from_playlist_id,
                position,
            } => {
                if let AppState::Main {
                    active_context_menu,
                    ..
                } = &mut self.state
                {
                    *active_context_menu = Some(ContextMenuState {
                        target: ContextMenuTarget::Track {
                            track,
                            from_playlist_id,
                        },
                        position,
                    });
                }
                Task::none()
            }
            Message::OpenAlbumContextMenu { album, position } => {
                if let AppState::Main {
                    active_context_menu,
                    ..
                } = &mut self.state
                {
                    *active_context_menu = Some(ContextMenuState {
                        target: ContextMenuTarget::Album(album),
                        position,
                    });
                }
                Task::none()
            }
            Message::OpenPlaylistContextMenu { playlist, position } => {
                if let AppState::Main {
                    active_context_menu,
                    ..
                } = &mut self.state
                {
                    *active_context_menu = Some(ContextMenuState {
                        target: ContextMenuTarget::Playlist(playlist),
                        position,
                    });
                }
                Task::none()
            }
            Message::OpenArtistContextMenu {
                artist_id,
                artist_name,
                position,
            } => {
                if let AppState::Main {
                    active_context_menu,
                    ..
                } = &mut self.state
                {
                    *active_context_menu = Some(ContextMenuState {
                        target: ContextMenuTarget::Artist {
                            artist_id,
                            artist_name,
                        },
                        position,
                    });
                }
                Task::none()
            }
            Message::CloseContextMenu => {
                if let AppState::Main {
                    active_context_menu,
                    ..
                } = &mut self.state
                {
                    *active_context_menu = None;
                }
                Task::none()
            }

            Message::CopyShareLink(title, url) => {
                if let AppState::Main {
                    active_context_menu,
                    toast_notification,
                    ..
                } = &mut self.state
                {
                    *active_context_menu = None;
                    *toast_notification =
                        Some(format!("Enlace de '{title}' copiado al portapapeles"));
                }
                Task::batch(vec![
                    iced::clipboard::write(url),
                    Task::perform(
                        async {
                            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                        },
                        |()| Message::DismissToast,
                    ),
                ])
            }

            Message::OpenAddToPlaylistModal(uris) => {
                if let AppState::Main {
                    active_context_menu,
                    active_modal,
                    ..
                } = &mut self.state
                {
                    *active_context_menu = None;
                    *active_modal = Some(ActiveModal::AddToPlaylist {
                        track_uris: uris,
                        search_query: String::new(),
                    });
                }
                Task::none()
            }

            Message::OpenEditPlaylistModal(pid, name, desc) => {
                if let AppState::Main {
                    active_context_menu,
                    active_modal,
                    ..
                } = &mut self.state
                {
                    *active_context_menu = None;
                    *active_modal = Some(ActiveModal::EditPlaylist {
                        playlist_id: pid,
                        name_input: name,
                        description_input: desc,
                    });
                }
                Task::none()
            }

            Message::OpenConfirmDeletePlaylistModal(pid, name) => {
                if let AppState::Main {
                    active_context_menu,
                    active_modal,
                    ..
                } = &mut self.state
                {
                    *active_context_menu = None;
                    *active_modal = Some(ActiveModal::ConfirmDeletePlaylist {
                        playlist_id: pid,
                        playlist_name: name,
                    });
                }
                Task::none()
            }

            Message::OpenCopyPlaylistModal(pid, name) => {
                if let AppState::Main {
                    active_context_menu,
                    active_modal,
                    ..
                } = &mut self.state
                {
                    *active_context_menu = None;
                    *active_modal = Some(ActiveModal::CopyPlaylistToAnother {
                        source_playlist_id: pid,
                        source_playlist_name: name,
                        search_query: String::new(),
                    });
                }
                Task::none()
            }

            Message::CloseModal => {
                if let AppState::Main { active_modal, .. } = &mut self.state {
                    *active_modal = None;
                }
                Task::none()
            }

            Message::ModalSearchInputChanged(query) => {
                if let AppState::Main {
                    active_modal:
                        Some(
                            ActiveModal::AddToPlaylist { search_query, .. }
                            | ActiveModal::CopyPlaylistToAnother { search_query, .. },
                        ),
                    ..
                } = &mut self.state
                {
                    *search_query = query;
                }
                Task::none()
            }

            Message::ModalNameInputChanged(val) => {
                if let AppState::Main {
                    active_modal: Some(ActiveModal::EditPlaylist { name_input, .. }),
                    ..
                } = &mut self.state
                {
                    *name_input = val;
                }
                Task::none()
            }

            Message::ModalDescInputChanged(val) => {
                if let AppState::Main {
                    active_modal:
                        Some(ActiveModal::EditPlaylist {
                            description_input, ..
                        }),
                    ..
                } = &mut self.state
                {
                    *description_input = val;
                }
                Task::none()
            }

            Message::AddTracksToPlaylistAction(playlist_id, uris) => {
                let client = if let AppState::Main { spotify_client, .. } = &self.state {
                    spotify_client.clone()
                } else {
                    None
                };

                if let AppState::Main { active_modal, .. } = &mut self.state {
                    *active_modal = None;
                }

                if let Some(spotify) = client {
                    Task::perform(
                        async move {
                            crate::api::playlist::add_tracks_to_playlist(
                                &spotify,
                                &playlist_id,
                                &uris,
                            )
                            .await?;
                            Ok("Canciones agregadas a la playlist".to_string())
                        },
                        Message::OperationFinished,
                    )
                } else {
                    Task::none()
                }
            }

            Message::RemoveTrackFromCurrentPlaylist(playlist_id, track_uri) => {
                let client = if let AppState::Main { spotify_client, .. } = &self.state {
                    spotify_client.clone()
                } else {
                    None
                };

                if let AppState::Main {
                    active_context_menu,
                    ..
                } = &mut self.state
                {
                    *active_context_menu = None;
                }

                if let Some(spotify) = client {
                    let pl_id = playlist_id.clone();
                    Task::perform(
                        async move {
                            crate::api::playlist::remove_tracks_from_playlist(
                                &spotify,
                                &pl_id,
                                &[track_uri],
                            )
                            .await?;
                            Ok("Canción eliminada de la playlist".to_string())
                        },
                        Message::OperationFinished,
                    )
                } else {
                    Task::none()
                }
            }

            Message::SaveAlbumToggle(album_id, currently_saved) => {
                let client = if let AppState::Main { spotify_client, .. } = &self.state {
                    spotify_client.clone()
                } else {
                    None
                };

                if let AppState::Main {
                    active_context_menu,
                    ..
                } = &mut self.state
                {
                    *active_context_menu = None;
                }

                if let Some(spotify) = client {
                    Task::perform(
                        async move {
                            if currently_saved {
                                crate::api::album::remove_album(&spotify, &album_id).await?;
                                Ok("Álbum eliminado de tu biblioteca".to_string())
                            } else {
                                crate::api::album::save_album(&spotify, &album_id).await?;
                                Ok("Álbum guardado en tu biblioteca".to_string())
                            }
                        },
                        Message::OperationFinished,
                    )
                } else {
                    Task::none()
                }
            }

            Message::SavePlaylistDetailsAction(playlist_id, name, desc) => {
                let client = if let AppState::Main { spotify_client, .. } = &self.state {
                    spotify_client.clone()
                } else {
                    None
                };

                if let AppState::Main { active_modal, .. } = &mut self.state {
                    *active_modal = None;
                }

                if let Some(spotify) = client {
                    Task::perform(
                        async move {
                            crate::api::playlist::change_playlist_details(
                                &spotify,
                                &playlist_id,
                                Some(&name),
                                Some(&desc),
                                None,
                            )
                            .await?;
                            Ok("Playlist actualizada con éxito".to_string())
                        },
                        Message::OperationFinished,
                    )
                } else {
                    Task::none()
                }
            }

            Message::DeletePlaylistConfirmed(playlist_id) => {
                let client = if let AppState::Main { spotify_client, .. } = &self.state {
                    spotify_client.clone()
                } else {
                    None
                };

                if let AppState::Main { active_modal, .. } = &mut self.state {
                    *active_modal = None;
                }

                if let Some(spotify) = client {
                    Task::perform(
                        async move {
                            crate::api::playlist::delete_playlist(&spotify, &playlist_id).await?;
                            Ok("Playlist eliminada".to_string())
                        },
                        Message::OperationFinished,
                    )
                } else {
                    Task::none()
                }
            }

            Message::TogglePlaylistPrivacy(playlist_id, currently_public) => {
                let client = if let AppState::Main { spotify_client, .. } = &self.state {
                    spotify_client.clone()
                } else {
                    None
                };

                if let AppState::Main {
                    active_context_menu,
                    ..
                } = &mut self.state
                {
                    *active_context_menu = None;
                }

                if let Some(spotify) = client {
                    let target_public = !currently_public;
                    Task::perform(
                        async move {
                            crate::api::playlist::change_playlist_details(
                                &spotify,
                                &playlist_id,
                                None,
                                None,
                                Some(target_public),
                            )
                            .await?;
                            Ok("Privacidad de la playlist actualizada".to_string())
                        },
                        Message::OperationFinished,
                    )
                } else {
                    Task::none()
                }
            }

            Message::CopyPlaylistTracksAction(source_pid, target_pid) => {
                let client = if let AppState::Main { spotify_client, .. } = &self.state {
                    spotify_client.clone()
                } else {
                    None
                };

                if let AppState::Main { active_modal, .. } = &mut self.state {
                    *active_modal = None;
                }

                if let Some(spotify) = client {
                    Task::perform(
                        async move {
                            let tracks =
                                crate::api::playlist::fetch_playlist_tracks(&spotify, &source_pid)
                                    .await?;
                            let uris: Vec<String> = tracks.into_iter().map(|t| t.uri).collect();
                            crate::api::playlist::add_tracks_to_playlist(
                                &spotify,
                                &target_pid,
                                &uris,
                            )
                            .await?;
                            Ok("Canciones copiadas a la otra playlist".to_string())
                        },
                        Message::OperationFinished,
                    )
                } else {
                    Task::none()
                }
            }

            Message::FollowArtistToggle(artist_id, currently_followed) => {
                let client = if let AppState::Main { spotify_client, .. } = &self.state {
                    spotify_client.clone()
                } else {
                    None
                };

                if let AppState::Main {
                    active_context_menu,
                    ..
                } = &mut self.state
                {
                    *active_context_menu = None;
                }

                if let Some(spotify) = client {
                    Task::perform(
                        async move {
                            if currently_followed {
                                crate::api::artist::unfollow_artist(&spotify, &artist_id).await?;
                                Ok("Dejaste de seguir al artista".to_string())
                            } else {
                                crate::api::artist::follow_artist(&spotify, &artist_id).await?;
                                Ok("Siguiendo al artista".to_string())
                            }
                        },
                        Message::OperationFinished,
                    )
                } else {
                    Task::none()
                }
            }

            Message::OpenQueuePanel => {
                if let AppState::Main {
                    active_context_menu,
                    active_right_panel,
                    ..
                } = &mut self.state
                {
                    *active_context_menu = None;
                    *active_right_panel = Some(RightPanelTab::Queue);
                }
                Task::none()
            }

            Message::ShowToast(msg) => {
                if let AppState::Main {
                    toast_notification, ..
                } = &mut self.state
                {
                    *toast_notification = Some(msg);
                }
                Task::perform(
                    async {
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    },
                    |()| Message::DismissToast,
                )
            }

            Message::DismissToast => {
                if let AppState::Main {
                    toast_notification, ..
                } = &mut self.state
                {
                    *toast_notification = None;
                }
                Task::none()
            }

            Message::OperationFinished(res) => {
                if let AppState::Main {
                    toast_notification, ..
                } = &mut self.state
                {
                    match res {
                        Ok(msg) => *toast_notification = Some(msg),
                        Err(e) => *toast_notification = Some(format!("Error: {e}")),
                    }
                }
                Task::perform(
                    async {
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    },
                    |()| Message::DismissToast,
                )
            }
            Message::ToggleShuffle => {
                if let AppState::Main {
                    playback,
                    context_queue,
                    original_context_queue,
                    context_index,
                    ..
                } = &mut self.state
                {
                    playback.is_shuffled = !playback.is_shuffled;
                    if playback.is_shuffled {
                        if original_context_queue.is_empty() {
                            original_context_queue.clone_from(context_queue);
                        }
                        if *context_index + 1 < context_queue.len() {
                            shuffle_slice(&mut context_queue[*context_index + 1..]);
                        }
                    } else if !original_context_queue.is_empty() {
                        let curr_uri = playback.current_track.as_ref().map(|t| t.uri.clone());
                        *context_queue = original_context_queue.clone();
                        if let Some(uri) = curr_uri {
                            if let Some(pos) = context_queue.iter().position(|t| t.uri == uri) {
                                *context_index = pos;
                            }
                        }
                    }
                }
                Task::none()
            }
            Message::ToggleRepeat => {
                if let AppState::Main { playback, .. } = &mut self.state {
                    playback.repeat_mode = match playback.repeat_mode {
                        RepeatMode::Off => RepeatMode::Context,
                        RepeatMode::Context => RepeatMode::One,
                        RepeatMode::One => RepeatMode::Off,
                    };
                }
                Task::none()
            }
            Message::AddToQueue(track) => {
                let mut tasks = Vec::new();
                if let AppState::Main {
                    user_queue,
                    active_context_menu,
                    toast_notification,
                    loaded_images,
                    ..
                } = &mut self.state
                {
                    let title = track.title.clone();
                    if let Some(ref img) = track.image_url {
                        tasks.extend(load_image_tasks(
                            std::iter::once(Some(img.clone())),
                            loaded_images,
                        ));
                    }
                    user_queue.push(track);
                    *active_context_menu = None;
                    *toast_notification = Some(format!("Added to queue: {title}"));
                }
                tasks.push(Task::perform(
                    async {
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    },
                    |()| Message::DismissToast,
                ));
                Task::batch(tasks)
            }
            Message::RemoveFromQueue(idx) => {
                if let AppState::Main { user_queue, .. } = &mut self.state {
                    if idx < user_queue.len() {
                        user_queue.remove(idx);
                    }
                }
                Task::none()
            }
            Message::MoveQueueItemUp(idx) => {
                if let AppState::Main { user_queue, .. } = &mut self.state {
                    if idx > 0 && idx < user_queue.len() {
                        user_queue.swap(idx, idx - 1);
                    }
                }
                Task::none()
            }
            Message::MoveQueueItemDown(idx) => {
                if let AppState::Main { user_queue, .. } = &mut self.state {
                    if idx + 1 < user_queue.len() {
                        user_queue.swap(idx, idx + 1);
                    }
                }
                Task::none()
            }
            Message::PlayQueueIndex(idx) => {
                if let AppState::Main {
                    user_queue,
                    playback,
                    audio_session,
                    loaded_images,
                    history,
                    ..
                } = &mut self.state
                {
                    if idx < user_queue.len() {
                        if let Some(curr) = playback.current_track.clone() {
                            history.push(curr);
                        }
                        let next_track = user_queue.remove(idx);
                        playback.current_track = Some(next_track.clone());
                        playback.progress_ms = 0;
                        playback.is_playing = true;
                        if let Some(session) = audio_session {
                            let _ = session
                                .cmd_tx
                                .try_send(PlayerCommand::Play(next_track.uri.clone()));
                        }
                        if let Some(ref img) = next_track.image_url {
                            return Task::batch(load_image_tasks(
                                std::iter::once(Some(img.clone())),
                                loaded_images,
                            ));
                        }
                    }
                }
                Task::none()
            }
            Message::ClearQueue => {
                if let AppState::Main { user_queue, .. } = &mut self.state {
                    user_queue.clear();
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

    #[allow(clippy::too_many_lines)]
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
                user_albums,
                user_top_tracks,
                featured_playlists,
                featured_albums,
                search_query,
                search_results,
                is_searching,
                sidebar_filter,
                selected_playlist,
                selected_album,
                user_queue,
                context_queue,
                context_index,
                loaded_images,
                window_width,
                active_context_menu,
                active_modal,
                toast_notification,
                ..
            } => crate::ui::main_layout::view(
                nav_item,
                playback,
                *sidebar_width,
                *right_panel_width,
                *active_right_panel,
                user_profile.as_ref(),
                user_playlists,
                user_albums,
                user_top_tracks,
                featured_playlists,
                featured_albums,
                search_query,
                search_results,
                *is_searching,
                *sidebar_filter,
                selected_playlist.as_ref(),
                selected_album.as_ref(),
                user_queue,
                context_queue,
                *context_index,
                loaded_images,
                *window_width,
                active_context_menu.as_ref(),
                active_modal.as_ref(),
                toast_notification.as_ref(),
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
                            .size(13)
                            .font(iced::Font {
                                weight: iced::font::Weight::Bold,
                                ..Default::default()
                            })
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
                                            background: Some(Background::Color(
                                                theme::SURFACE_HOVER,
                                            )),
                                            ..base
                                        }
                                    }
                                    _ => base,
                                }
                            }),
                    ),
            )
            .padding([10, 16])
            .width(Length::Fill)
            .style(|_theme| container::Style {
                background: Some(Background::Color(theme::COLOR_ERROR)),
                border: Border {
                    radius: theme::RADIUS_MD.into(),
                    color: theme::BORDER_SUBTLE,
                    width: 1.0,
                },
                text_color: Some(theme::TEXT_PRIMARY),
                ..Default::default()
            });

            Column::new()
                .spacing(8)
                .push(Container::new(error_banner).padding([8, 12]))
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

fn load_image_tasks(
    urls: impl IntoIterator<Item = Option<String>>,
    loaded_images: &std::collections::HashMap<String, iced::widget::image::Handle>,
) -> Vec<Task<Message>> {
    let mut tasks = Vec::new();
    for url in urls.into_iter().flatten() {
        if !url.is_empty() && !loaded_images.contains_key(&url) {
            let u = url.clone();
            tasks.push(Task::perform(
                async move { crate::api::cache::ImageCache::fetch_image_bytes(u).await },
                Message::ImageLoaded,
            ));
        }
    }
    tasks
}

#[allow(clippy::cast_possible_truncation)]
fn shuffle_slice<T>(slice: &mut [T]) {
    use std::time::{SystemTime, UNIX_EPOCH};
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(12345, |d| d.as_nanos() as u64);
    let mut state = seed;
    let len = slice.len();
    for i in (1..len).rev() {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let j = (state % (i as u64 + 1)) as usize;
        slice.swap(i, j);
    }
}
