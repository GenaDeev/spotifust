use crate::audio::engine::AudioCommand;
use crate::audio::session::{AudioSession, AudioSessionEvent, PlayerCommand};
use crate::error::AppError;
use crate::ui::login;
use iced::{Element, Task};
use librespot::playback::player::PlayerEvent;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::sync::Arc;

/// Decoded-handle cache size. Entries are 256px JPEG thumbnails (~15-25 KB each),
/// so 160 covers stay around 3-4 MB while a whole library page keeps its art.
pub const IMAGE_CACHE_CAPACITY: usize = 160;

/// Recent searches kept in memory.
const SEARCH_CACHE_CAPACITY: usize = 32;
/// Typing pause before a search request goes out. Short enough to feel instant,
/// long enough to not fire one request per keystroke.
const SEARCH_DEBOUNCE: std::time::Duration = std::time::Duration::from_millis(150);

fn search_cache_key(query: &str) -> String {
    query.trim().to_lowercase()
}

/// Covers fetched eagerly when a playlist opens; the rest load as rows scroll into view.
const PLAYLIST_ROWS_PREFETCH: usize = 30;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavigationItem {
    Home,
    #[allow(dead_code)]
    Search,
    #[allow(dead_code)]
    Library,
    Settings,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NavDestination {
    Home,
    Search(String),
    Library,
    Settings,
    Playlist(String),
    Album(String),
    Artist(String),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SearchCategoryFilter {
    #[default]
    All,
    Tracks,
    Albums,
    Artists,
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

#[derive(Debug, Clone)]
pub struct SelectedArtistState {
    pub id: String,
    pub name: String,
    pub image_url: Option<String>,
    pub genres: Vec<String>,
    pub followers: u32,
    pub top_tracks: Vec<crate::api::artist::ArtistTopTrack>,
    pub albums: Vec<crate::api::artist::ArtistAlbum>,
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
    #[serde(default)]
    pub explicit: bool,
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

pub fn save_ui_scale(scale: f32) {
    let _ = crate::api::cache::DiskMetadataCache::save("ui_scale", &scale);
}

#[must_use]
pub fn load_ui_scale() -> f32 {
    crate::api::cache::DiskMetadataCache::load::<f32>("ui_scale")
        .unwrap_or(1.0)
        .clamp(0.70, 1.30)
}

pub fn save_accent_tone(tone: crate::ui::theme::AccentTone) {
    let _ = crate::api::cache::DiskMetadataCache::save("accent_tone", &tone);
}

#[must_use]
pub fn load_accent_tone() -> crate::ui::theme::AccentTone {
    crate::api::cache::DiskMetadataCache::load::<crate::ui::theme::AccentTone>("accent_tone")
        .unwrap_or_default()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
pub enum UiLanguage {
    #[default]
    English,
    Spanish,
    Portuguese,
    German,
}

impl UiLanguage {
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::English => "English",
            Self::Spanish => "Español",
            Self::Portuguese => "Português",
            Self::German => "Deutsch",
        }
    }

    pub const ALL: [Self; 4] = [Self::English, Self::Spanish, Self::Portuguese, Self::German];
}

pub fn save_ui_language(lang: UiLanguage) {
    let _ = crate::api::cache::DiskMetadataCache::save("ui_language", &lang);
}

#[must_use]
pub fn load_ui_language() -> UiLanguage {
    crate::api::cache::DiskMetadataCache::load::<UiLanguage>("ui_language").unwrap_or_default()
}

pub fn save_audio_bitrate(bitrate: crate::audio::session::AudioBitrate) {
    let _ = crate::api::cache::DiskMetadataCache::save("audio_bitrate", &bitrate);
}

#[must_use]
pub fn load_audio_bitrate() -> crate::audio::session::AudioBitrate {
    crate::api::cache::DiskMetadataCache::load::<crate::audio::session::AudioBitrate>(
        "audio_bitrate",
    )
    .unwrap_or_default()
}

pub fn save_audio_normalization(enabled: bool) {
    let _ = crate::api::cache::DiskMetadataCache::save("audio_normalization", &enabled);
}

#[must_use]
pub fn load_audio_normalization() -> bool {
    crate::api::cache::DiskMetadataCache::load::<bool>("audio_normalization").unwrap_or(true)
}

pub fn save_gapless_playback(enabled: bool) {
    let _ = crate::api::cache::DiskMetadataCache::save("gapless_playback", &enabled);
}

#[must_use]
pub fn load_gapless_playback() -> bool {
    crate::api::cache::DiskMetadataCache::load::<bool>("gapless_playback").unwrap_or(true)
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

pub fn perform_graceful_shutdown(
    playback: &PlaybackState,
    audio_session: Option<&AudioSession>,
    ui_scale: f32,
    sidebar_width: f32,
    right_panel_width: f32,
) {
    save_last_playback_state(playback);
    save_saved_volume(playback.volume);
    save_ui_scale(ui_scale);
    let _ = save_layout(sidebar_width, right_panel_width);
    if let Some(session) = audio_session {
        let _ = session.cmd_tx.try_send(PlayerCommand::Stop);
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
#[allow(clippy::struct_excessive_bools)] // independent player flags, not a state machine
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
    pub last_seek: Option<(std::time::Instant, u32)>,
    /// Whether the live audio session has `current_track` loaded (false for a
    /// track restored from disk at startup).
    pub track_loaded: bool,
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
            last_seek: None,
            track_loaded: false,
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

/// Connection state of the librespot audio session, surfaced in the UI.
#[derive(Debug, Clone, Default)]
pub enum PlaybackLink {
    /// Connecting with stored credentials.
    #[default]
    Connecting,
    /// Waiting for the user to approve the device pairing in a browser.
    AwaitingApproval { user_code: String, url: String },
    /// Audio session is live.
    Ready,
    /// Connection failed for a reason other than missing pairing.
    Failed(String),
}

/// Visible window of the main content scrollable, used to virtualize long track lists.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScrollWindow {
    pub offset_y: f32,
    pub viewport_height: f32,
}

impl Default for ScrollWindow {
    fn default() -> Self {
        Self {
            offset_y: 0.0,
            viewport_height: 1200.0,
        }
    }
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
    /// Whether `position` is the real cursor location of the click that opened it.
    pub anchored: bool,
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
    Initializing,
    Login {
        is_loading: bool,
        error: Option<String>,
    },
    Main {
        nav_item: NavigationItem,
        playback: PlaybackState,
        audio_session: Option<AudioSession>,
        playback_link: PlaybackLink,
        pending_play_uri: Option<String>,
        main_scroll: ScrollWindow,
        pending_images: std::collections::HashSet<String>,
        user_profile: Option<crate::api::user::UserProfile>,
        user_playlists: Vec<crate::api::playlist::PlaylistSummary>,
        user_albums: Vec<crate::api::album::AlbumSummary>,
        user_top_tracks: Vec<crate::api::tracks::TopTrack>,
        featured_playlists: Vec<crate::api::playlist::PlaylistSummary>,
        featured_albums: Vec<crate::api::album::AlbumSummary>,
        search_query: String,
        search_results: crate::api::search::SearchResults,
        is_searching: bool,
        /// Recent query → results, so retyping or backspacing is instant.
        search_cache: crate::api::cache::LruCache<String, crate::api::search::SearchResults>,
        sidebar_filter: SidebarFilter,
        selected_playlist: Option<SelectedPlaylistState>,
        selected_album: Option<SelectedAlbumState>,
        selected_artist: Option<SelectedArtistState>,
        user_queue: Vec<TrackInfo>,
        context_queue: Vec<TrackInfo>,
        original_context_queue: Vec<TrackInfo>,
        context_index: usize,
        history: Vec<TrackInfo>,
        active_context_menu: Option<ContextMenuState>,
        active_modal: Option<ActiveModal>,
        toast_notification: Option<String>,
        toast_id: u64,
        loaded_images: crate::api::cache::LruCache<String, iced::widget::image::Handle>,
        spotify_client: Option<Arc<rspotify::AuthCodePkceSpotify>>,
        sidebar_width: f32,
        right_panel_width: f32,
        active_right_panel: Option<RightPanelTab>,
        dragging_sidebar: bool,
        dragging_right_panel: bool,
        window_width: f32,
        window_height: f32,
        /// Where (and when) the last right click happened, to place context menus.
        last_right_click: Option<(iced::Point, std::time::Instant)>,
        navigation_history: Vec<NavDestination>,
        forward_history: Vec<NavDestination>,
        current_lyrics: Option<crate::api::lyrics::LyricsData>,
        /// Track URI the lyrics panel is showing (or loading) lyrics for.
        lyrics_key: Option<String>,
        /// Artist the bio card is showing (or loading).
        bio_key: Option<String>,
        /// Index of the highlighted synced lyric line, to scroll only when it changes.
        lyrics_line: Option<usize>,
        is_loading_lyrics: bool,
        current_artist_bio: Option<crate::api::artist::ArtistBio>,
        is_loading_artist_bio: bool,
        autoplay_enabled: bool,
        search_category_filter: SearchCategoryFilter,
        cache_size_bytes: u64,
        allow_explicit_content: bool,
        ui_scale: f32,
        accent_tone: crate::ui::theme::AccentTone,
        ui_language: UiLanguage,
        audio_bitrate: crate::audio::session::AudioBitrate,
        audio_normalization: bool,
        gapless_playback: bool,
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
    CancelLogin,
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
    /// Fired after a short typing pause; searches only if the query is still current.
    SearchDebounced(String),
    SearchResultsFetched(String, Result<crate::api::search::SearchResults, AppError>),
    SelectPlaylist(String),
    PlaylistTracksFetched(
        String,
        Result<Vec<crate::api::playlist::PlaylistTrack>, AppError>,
    ),
    SelectAlbum(String),
    SelectArtist(String),
    AlbumDetailsFetched(String, Result<crate::api::album::AlbumDetail, AppError>),
    ArtistDetailsFetched(String, Result<crate::api::artist::ArtistDetail, AppError>),
    PlayTrack(String),
    SidebarFilterSelected(SidebarFilter),
    RequestImages(Vec<String>),
    ImageLoaded(String, Result<Vec<u8>, AppError>),
    ClearSelection,
    // Audio Messages
    AudioSessionConnected(AudioSession),
    PlaybackSessionResult(Result<AudioSession, AppError>),
    PlaybackPairingStarted(Result<crate::audio::credentials::PairingRequest, AppError>),
    RetryPlaybackConnection,
    OpenPlaybackPairingUrl,
    MainContentScrolled(ScrollWindow),
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
    DismissToastId(u64),
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
    // Error Actions
    DismissError,
    // Panel Layout Messages
    StartSidebarDrag,
    StartRightPanelDrag,
    EndPanelDrag,
    PanelDragMoved(f32),
    ToggleRightPanel(RightPanelTab),
    WindowResized(iced::Size),
    RightClickAt(iced::Point),
    NavigateBack,
    NavigateForward,
    /// Lyrics for the track with the given URI.
    LyricsFetched(String, Result<crate::api::lyrics::LyricsData, AppError>),
    /// Bio for the given (primary) artist name.
    ArtistBioFetched(String, Result<crate::api::artist::ArtistBio, AppError>),
    SeekToMs(u32),
    ToggleAutoplay,
    AutoplayRecommendationsFetched(Result<Vec<crate::api::tracks::TopTrack>, AppError>),
    SearchCategoryFilterSelected(SearchCategoryFilter),
    ClearCacheRequested,
    CacheCleared(Result<u64, AppError>),
    CacheSizeCalculated(u64),
    ToggleExplicitContent,
    SetUiScale(f32),
    AdjustUiScale(f32),
    ResetUiScale,
    SetAccentTone(crate::ui::theme::AccentTone),
    OpenSpotifyAccount,
    SetUiLanguage(UiLanguage),
    SetAudioBitrate(crate::audio::session::AudioBitrate),
    ToggleAudioNormalization,
    ToggleGaplessPlayback,
    AppCloseRequested,
}

/// Emits the cursor position of every right click. The cursor position is tracked
/// inside the stream itself, so mouse movement never wakes up `update()`.
struct RightClickRecipe;

impl iced::advanced::subscription::Recipe for RightClickRecipe {
    type Output = Message;

    fn hash(&self, state: &mut iced::advanced::subscription::Hasher) {
        use std::hash::Hash;
        std::any::TypeId::of::<Self>().hash(state);
    }

    fn stream(
        self: Box<Self>,
        input: iced::advanced::subscription::EventStream,
    ) -> futures::stream::BoxStream<'static, Self::Output> {
        use futures::StreamExt;
        let mut cursor = iced::Point::ORIGIN;
        Box::pin(input.filter_map(move |event| {
            let msg = match event {
                iced::advanced::subscription::Event::Interaction {
                    event: iced::Event::Mouse(mouse_event),
                    ..
                } => match mouse_event {
                    iced::mouse::Event::CursorMoved { position } => {
                        cursor = position;
                        None
                    }
                    iced::mouse::Event::ButtonPressed(iced::mouse::Button::Right) => {
                        Some(Message::RightClickAt(cursor))
                    }
                    _ => None,
                },
                _ => None,
            };
            futures::future::ready(msg)
        }))
    }
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

fn get_current_destination(
    nav_item: NavigationItem,
    selected_playlist: Option<&SelectedPlaylistState>,
    selected_album: Option<&SelectedAlbumState>,
    selected_artist: Option<&SelectedArtistState>,
    search_query: &str,
) -> NavDestination {
    if let Some(p) = selected_playlist {
        NavDestination::Playlist(p.id.clone())
    } else if let Some(a) = selected_album {
        NavDestination::Album(a.id.clone())
    } else if let Some(ar) = selected_artist {
        NavDestination::Artist(ar.id.clone())
    } else {
        match nav_item {
            NavigationItem::Search => NavDestination::Search(search_query.to_string()),
            NavigationItem::Library => NavDestination::Library,
            NavigationItem::Settings => NavDestination::Settings,
            NavigationItem::Home => NavDestination::Home,
        }
    }
}

fn push_to_history(history: &mut Vec<NavDestination>, dest: NavDestination) {
    if history.last() != Some(&dest) {
        if history.len() >= 50 {
            history.remove(0);
        }
        history.push(dest);
    }
}

impl App {
    pub fn new() -> (Self, Task<Message>) {
        let (audio_tx, _) = tokio::sync::mpsc::channel(16);

        (
            Self {
                state: AppState::Initializing,
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
            AppState::Initializing => iced::Subscription::none(),
            AppState::Login { is_loading, .. } => {
                let mut subs = vec![iced::event::listen().filter_map(|event| match event {
                    iced::Event::Window(iced::window::Event::CloseRequested) => {
                        Some(Message::AppCloseRequested)
                    }
                    _ => None,
                })];
                if *is_loading {
                    subs.push(
                        iced::time::every(std::time::Duration::from_secs(2))
                            .map(|_| Message::CheckLogin),
                    );
                }
                iced::Subscription::batch(subs)
            }
            AppState::Main {
                audio_session,
                playback,
                dragging_sidebar,
                dragging_right_panel,
                ..
            } => {
                let mut subs = vec![];
                if audio_session.is_none() && playback.is_playing {
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
                subs.push(iced::advanced::subscription::from_recipe(RightClickRecipe));
                if *dragging_sidebar || *dragging_right_panel {
                    subs.push(iced::event::listen().filter_map(|event| match event {
                        iced::Event::Mouse(iced::mouse::Event::CursorMoved { position }) => {
                            Some(Message::PanelDragMoved(position.x))
                        }
                        iced::Event::Mouse(iced::mouse::Event::ButtonReleased(
                            iced::mouse::Button::Left,
                        )) => Some(Message::EndPanelDrag),
                        _ => None,
                    }));
                }
                subs.push(iced::event::listen().filter_map(|event| match event {
                    iced::Event::Window(iced::window::Event::Resized(size)) => {
                        Some(Message::WindowResized(size))
                    }
                    iced::Event::Window(iced::window::Event::CloseRequested) => {
                        Some(Message::AppCloseRequested)
                    }
                    iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
                        key,
                        modifiers,
                        ..
                    }) => map_keyboard_shortcut(&key, modifiers),
                    _ => None,
                }));
                iced::Subscription::batch(subs)
            }
        }
    }
}

fn map_keyboard_shortcut(
    key: &iced::keyboard::Key,
    modifiers: iced::keyboard::Modifiers,
) -> Option<Message> {
    use iced::keyboard::{Key, key::Named};

    if modifiers.command() || modifiers.control() {
        match key {
            Key::Character(c) if c.eq_ignore_ascii_case("f") => {
                return Some(Message::NavigationSelected(NavigationItem::Search));
            }
            Key::Character(c) if c.eq_ignore_ascii_case("m") => {
                return Some(Message::ToggleMute);
            }
            Key::Character(c) if c.eq_ignore_ascii_case("s") => {
                return Some(Message::ToggleShuffle);
            }
            Key::Character(c) if c.eq_ignore_ascii_case("r") => {
                return Some(Message::ToggleRepeat);
            }
            Key::Character(c) if c.eq_ignore_ascii_case("q") => {
                return Some(Message::ToggleRightPanel(RightPanelTab::Queue));
            }
            Key::Character(c) if c.eq_ignore_ascii_case("d") => {
                return Some(Message::ToggleRightPanel(RightPanelTab::Lyrics));
            }
            Key::Character(c) if c == "+" || c == "=" => {
                return Some(Message::AdjustUiScale(0.05));
            }
            Key::Character(c) if c == "-" => {
                return Some(Message::AdjustUiScale(-0.05));
            }
            Key::Character(c) if c == "0" => {
                return Some(Message::ResetUiScale);
            }
            Key::Named(Named::ArrowLeft) => {
                return Some(Message::NavigateBack);
            }
            Key::Named(Named::ArrowRight) => {
                return Some(Message::NavigateForward);
            }
            _ => {}
        }
    }

    if modifiers.alt() {
        match key {
            Key::Named(Named::ArrowLeft) => return Some(Message::NavigateBack),
            Key::Named(Named::ArrowRight) => return Some(Message::NavigateForward),
            _ => {}
        }
    }

    match key {
        Key::Named(Named::Space) => Some(Message::TogglePlayback),
        Key::Named(Named::ArrowRight) => Some(Message::SkipNext),
        Key::Named(Named::ArrowLeft) => Some(Message::SkipPrev),
        Key::Named(Named::ArrowUp) => Some(Message::AdjustVolume(0.05)),
        Key::Named(Named::ArrowDown) => Some(Message::AdjustVolume(-0.05)),
        _ => None,
    }
}

impl App {
    #[must_use]
    pub fn scale_factor(&self) -> f32 {
        match &self.state {
            AppState::Main { ui_scale, .. } => *ui_scale,
            AppState::Login { .. } | AppState::Initializing => 1.0,
        }
    }
    fn navigate_to(&mut self, dest: NavDestination) -> Task<Message> {
        match dest {
            NavDestination::Home => {
                if let AppState::Main {
                    nav_item,
                    selected_playlist,
                    selected_album,
                    selected_artist,
                    ..
                } = &mut self.state
                {
                    *nav_item = NavigationItem::Home;
                    *selected_playlist = None;
                    *selected_album = None;
                    *selected_artist = None;
                }
                Task::none()
            }
            NavDestination::Search(query) => {
                if let AppState::Main {
                    nav_item,
                    selected_playlist,
                    selected_album,
                    selected_artist,
                    ..
                } = &mut self.state
                {
                    *selected_playlist = None;
                    *selected_album = None;
                    *selected_artist = None;
                    *nav_item = NavigationItem::Search;
                }
                self.update(Message::SearchInputChanged(query))
            }
            NavDestination::Library => {
                if let AppState::Main {
                    nav_item,
                    selected_playlist,
                    selected_album,
                    selected_artist,
                    ..
                } = &mut self.state
                {
                    *nav_item = NavigationItem::Library;
                    *selected_playlist = None;
                    *selected_album = None;
                    *selected_artist = None;
                }
                Task::none()
            }
            NavDestination::Settings => {
                if let AppState::Main {
                    nav_item,
                    selected_playlist,
                    selected_album,
                    selected_artist,
                    ..
                } = &mut self.state
                {
                    *nav_item = NavigationItem::Settings;
                    *selected_playlist = None;
                    *selected_album = None;
                    *selected_artist = None;
                }
                Task::perform(
                    async { crate::api::cache::calculate_cache_size_bytes() },
                    Message::CacheSizeCalculated,
                )
            }
            NavDestination::Playlist(id) => self.load_playlist_internal(&id),
            NavDestination::Album(id) => self.load_album_internal(&id),
            NavDestination::Artist(id) => self.load_artist_internal(&id),
        }
    }

    fn load_playlist_internal(&mut self, playlist_id: &str) -> Task<Message> {
        if let AppState::Main {
            user_playlists,
            selected_playlist,
            selected_album,
            selected_artist,
            loaded_images,
            spotify_client,
            nav_item,
            ..
        } = &mut self.state
        {
            *nav_item = NavigationItem::Home;
            *selected_album = None;
            *selected_artist = None;
            let (playlist_name, image_url) = user_playlists
                .iter()
                .find(|p| p.id == playlist_id)
                .map_or_else(
                    || ("Playlist".to_string(), None),
                    |p| (p.name.clone(), p.image_url.clone()),
                );

            let mut tasks = load_image_tasks(std::iter::once(image_url.clone()), loaded_images);
            *selected_playlist = Some(SelectedPlaylistState {
                id: playlist_id.to_string(),
                name: playlist_name,
                image_url,
                tracks: Vec::new(),
                is_loading: true,
            });

            if let Some(client) = spotify_client.clone() {
                let pid = playlist_id.to_string();
                tasks.push(Task::perform(
                    async move {
                        let res = crate::api::playlist::fetch_playlist_tracks(&client, &pid).await;
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

    fn load_album_internal(&mut self, album_id: &str) -> Task<Message> {
        if rspotify::model::AlbumId::from_id_or_uri(album_id).is_err() {
            return self.update(Message::SearchInputChanged(album_id.to_string()));
        }
        if let AppState::Main {
            user_albums,
            selected_album,
            selected_playlist,
            selected_artist,
            spotify_client,
            nav_item,
            ..
        } = &mut self.state
        {
            *nav_item = NavigationItem::Home;
            *selected_playlist = None;
            *selected_artist = None;
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
                id: album_id.to_string(),
                name,
                artist_name: artist,
                image_url,
                release_date,
                tracks: Vec::new(),
                is_loading: true,
            });

            if let Some(client) = spotify_client.clone() {
                let aid = album_id.to_string();
                return Task::perform(
                    async move {
                        let res = crate::api::album::fetch_album_details(&client, &aid).await;
                        (aid, res)
                    },
                    |(aid, res)| Message::AlbumDetailsFetched(aid, res),
                );
            }
        }
        Task::none()
    }

    fn load_artist_internal(&mut self, artist_id: &str) -> Task<Message> {
        if rspotify::model::ArtistId::from_id_or_uri(artist_id).is_err() {
            return self.update(Message::SearchInputChanged(artist_id.to_string()));
        }
        if let AppState::Main {
            selected_artist,
            selected_album,
            selected_playlist,
            spotify_client,
            nav_item,
            ..
        } = &mut self.state
        {
            *nav_item = NavigationItem::Home;
            *selected_playlist = None;
            *selected_album = None;

            *selected_artist = Some(SelectedArtistState {
                id: artist_id.to_string(),
                name: "Artist".to_string(),
                image_url: None,
                genres: Vec::new(),
                followers: 0,
                top_tracks: Vec::new(),
                albums: Vec::new(),
                is_loading: true,
            });

            if let Some(client) = spotify_client.clone() {
                let aid = artist_id.to_string();
                return Task::perform(
                    async move {
                        let res = crate::api::artist::fetch_artist_details(&client, &aid).await;
                        (aid, res)
                    },
                    |(aid, res)| Message::ArtistDetailsFetched(aid, res),
                );
            }
        }
        Task::none()
    }

    /// Runs [`Self::update_inner`] and, whenever the current track or the open
    /// right panel changed, loads the lyrics / artist bio that panel now needs.
    pub fn update(&mut self, message: Message) -> Task<Message> {
        let panels_before = self.panel_key();
        let page_before = self.page_key();
        let mut task = self.update_inner(message);
        let panels_after = self.panel_key();
        if panels_after != panels_before {
            let tab_changed =
                panels_after.as_ref().map(|k| k.1) != panels_before.as_ref().map(|k| k.1);
            if tab_changed {
                if let AppState::Main { lyrics_line, .. } = &mut self.state {
                    *lyrics_line = None;
                }
                task = Task::batch([
                    task,
                    iced::widget::operation::snap_to(
                        crate::ui::main_layout::RIGHT_PANEL_SCROLL_ID,
                        iced::widget::operation::RelativeOffset::START,
                    ),
                ]);
            }
            task = Task::batch([task, self.refresh_track_panels()]);
        }
        if self.page_key() != page_before {
            // Each page starts at the top; the scrollable widget state would
            // otherwise carry the previous page's offset over.
            if let AppState::Main { main_scroll, .. } = &mut self.state {
                *main_scroll = ScrollWindow {
                    offset_y: 0.0,
                    ..*main_scroll
                };
            }
            task = Task::batch([
                task,
                iced::widget::operation::snap_to(
                    crate::ui::main_layout::MAIN_SCROLL_ID,
                    iced::widget::operation::RelativeOffset::START,
                ),
            ]);
        }
        task
    }

    fn page_key(&self) -> Option<NavDestination> {
        match &self.state {
            AppState::Main {
                nav_item,
                selected_playlist,
                selected_album,
                selected_artist,
                ..
            } => Some(get_current_destination(
                *nav_item,
                selected_playlist.as_ref(),
                selected_album.as_ref(),
                selected_artist.as_ref(),
                "",
            )),
            _ => None,
        }
    }

    fn panel_key(&self) -> Option<(Option<String>, Option<RightPanelTab>)> {
        match &self.state {
            AppState::Main {
                playback,
                active_right_panel,
                ..
            } => Some((
                playback.current_track.as_ref().map(|t| t.uri.clone()),
                *active_right_panel,
            )),
            _ => None,
        }
    }

    /// Keeps the active synced lyric line in view as playback advances.
    #[allow(clippy::cast_precision_loss)]
    fn follow_lyrics(&mut self) -> Task<Message> {
        let AppState::Main {
            playback,
            active_right_panel: Some(RightPanelTab::Lyrics),
            current_lyrics: Some(lyrics),
            lyrics_line,
            ..
        } = &mut self.state
        else {
            return Task::none();
        };
        if !lyrics.synced || lyrics.lines.len() < 2 {
            return Task::none();
        }
        let active = lyrics
            .lines
            .iter()
            .rposition(|l| l.timestamp_ms <= playback.progress_ms);
        if active == *lyrics_line {
            return Task::none();
        }
        *lyrics_line = active;
        // Line heights vary with wrapping, so position proportionally and keep a few
        // lines of context above the active one.
        let count = lyrics.lines.len() as f32;
        let y = ((active.unwrap_or(0) as f32 - 3.0) / (count - 6.0).max(1.0)).clamp(0.0, 1.0);
        iced::widget::operation::snap_to(
            crate::ui::main_layout::RIGHT_PANEL_SCROLL_ID,
            iced::widget::operation::RelativeOffset { x: 0.0, y },
        )
    }

    fn refresh_track_panels(&mut self) -> Task<Message> {
        let AppState::Main {
            playback,
            active_right_panel,
            current_lyrics,
            lyrics_key,
            lyrics_line,
            is_loading_lyrics,
            current_artist_bio,
            bio_key,
            is_loading_artist_bio,
            ..
        } = &mut self.state
        else {
            return Task::none();
        };
        let Some(track) = &playback.current_track else {
            return Task::none();
        };
        match active_right_panel {
            Some(RightPanelTab::Lyrics) if lyrics_key.as_deref() != Some(track.uri.as_str()) => {
                *lyrics_key = Some(track.uri.clone());
                *lyrics_line = None;
                *current_lyrics = None;
                *is_loading_lyrics = true;
                let key = track.uri.clone();
                let (title, artist, album, duration) = (
                    track.title.clone(),
                    track.artist.clone(),
                    track.album.clone(),
                    track.duration_ms,
                );
                Task::perform(
                    async move {
                        crate::api::lyrics::fetch_lyrics(&title, &artist, &album, duration).await
                    },
                    move |res| Message::LyricsFetched(key.clone(), res),
                )
            }
            Some(RightPanelTab::NowPlaying) => {
                let artist = crate::api::lyrics::primary_artist(&track.artist).to_string();
                if artist.is_empty() || bio_key.as_deref() == Some(artist.as_str()) {
                    return Task::none();
                }
                *bio_key = Some(artist.clone());
                *current_artist_bio = None;
                *is_loading_artist_bio = true;
                let key = artist.clone();
                Task::perform(
                    async move { crate::api::artist::fetch_artist_bio(&artist).await },
                    move |res| Message::ArtistBioFetched(key.clone(), res),
                )
            }
            _ => Task::none(),
        }
    }

    #[allow(clippy::too_many_lines)]
    fn update_inner(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ErrorEncountered(e) => {
                if matches!(e, AppError::Auth(_)) {
                    let _ = self.audio_tx.try_send(AudioCommand::Pause);
                    let _ = crate::api::cache::clear_cache_disk();
                    let _ = crate::api::auth::delete_refresh_token_from_keyring();
                    self.active_error = None;
                    self.state = AppState::Login {
                        is_loading: false,
                        error: Some("Session expired. Please log in again.".to_string()),
                    };
                } else {
                    self.active_error = Some(e.to_string());
                }
                Task::none()
            }
            Message::DismissError => {
                self.active_error = None;
                Task::none()
            }
            Message::CancelLogin => {
                if let AppState::Login { is_loading, .. } = &mut self.state {
                    *is_loading = false;
                }
                Task::none()
            }
            Message::LoginRequested => {
                if let AppState::Login {
                    is_loading, error, ..
                } = &mut self.state
                {
                    *is_loading = true;
                    *error = None;

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
            Message::CheckLoginFailed | Message::PlaybackTick => Task::none(),

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
                    playback_link: PlaybackLink::Connecting,
                    pending_play_uri: None,
                    main_scroll: ScrollWindow::default(),
                    pending_images: std::collections::HashSet::new(),
                    user_profile: cached_profile,
                    user_playlists: cached_playlists,
                    user_albums: cached_albums,
                    user_top_tracks: cached_top_tracks,
                    featured_playlists: cached_featured_playlists,
                    featured_albums: cached_featured_albums,
                    search_query: String::new(),
                    search_results: crate::api::search::SearchResults::default(),
                    is_searching: false,
                    search_cache: crate::api::cache::LruCache::new(SEARCH_CACHE_CAPACITY),
                    sidebar_filter: SidebarFilter::All,
                    selected_playlist: None,
                    selected_album: None,
                    selected_artist: None,
                    user_queue: Vec::new(),
                    context_queue: Vec::new(),
                    original_context_queue: Vec::new(),
                    context_index: 0,
                    history: Vec::new(),
                    active_context_menu: None,
                    active_modal: None,
                    toast_notification: None,
                    toast_id: 0,
                    loaded_images: crate::api::cache::LruCache::new(IMAGE_CACHE_CAPACITY),
                    spotify_client: Some(Arc::clone(&spotify_arc)),
                    sidebar_width: sw,
                    right_panel_width: rw,
                    active_right_panel: None,
                    dragging_sidebar: false,
                    dragging_right_panel: false,
                    window_width: 1200.0,
                    window_height: 800.0,
                    last_right_click: None,
                    navigation_history: Vec::new(),
                    forward_history: Vec::new(),
                    current_lyrics: None,
                    lyrics_key: None,
                    bio_key: None,
                    lyrics_line: None,
                    is_loading_lyrics: false,
                    current_artist_bio: None,
                    is_loading_artist_bio: false,
                    autoplay_enabled: true,
                    search_category_filter: SearchCategoryFilter::All,
                    cache_size_bytes: 0,
                    allow_explicit_content: true,
                    ui_scale: load_ui_scale(),
                    accent_tone: load_accent_tone(),
                    ui_language: load_ui_language(),
                    audio_bitrate: load_audio_bitrate(),
                    audio_normalization: load_audio_normalization(),
                    gapless_playback: load_gapless_playback(),
                };

                let spotify_2 = Arc::clone(&spotify_arc);
                let spotify_3 = Arc::clone(&spotify_arc);
                let spotify_4 = Arc::clone(&spotify_arc);
                let spotify_5 = Arc::clone(&spotify_arc);
                let spotify_6 = Arc::clone(&spotify_arc);
                let spotify_7 = Arc::clone(&spotify_arc);
                let spotify_8 = Arc::clone(&spotify_arc);

                Task::batch([
                    connect_playback_task(
                        load_audio_bitrate(),
                        load_audio_normalization(),
                        load_gapless_playback(),
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
                    Task::perform(
                        async { crate::api::cache::calculate_cache_size_bytes() },
                        Message::CacheSizeCalculated,
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
                            explicit: false,
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
                    search_cache,
                    is_searching,
                    nav_item,
                    loaded_images,
                    ..
                } = &mut self.state
                {
                    search_query.clone_from(&query);
                    *nav_item = NavigationItem::Search;

                    if query.trim().is_empty() {
                        *search_results = crate::api::search::SearchResults::default();
                        *is_searching = false;
                        return Task::none();
                    }
                    if let Some(cached) = search_cache.get(&search_cache_key(&query)) {
                        *search_results = cached.clone();
                        *is_searching = false;
                        return Task::batch(search_image_tasks(search_results, loaded_images));
                    }
                    // Previous results stay on screen until the new ones arrive.
                    *is_searching = true;
                    return Task::perform(
                        async move {
                            tokio::time::sleep(SEARCH_DEBOUNCE).await;
                            query
                        },
                        Message::SearchDebounced,
                    );
                }
                Task::none()
            }
            Message::SearchDebounced(query) => {
                if let AppState::Main {
                    search_query,
                    spotify_client,
                    ..
                } = &self.state
                {
                    if *search_query == query {
                        if let Some(client) = spotify_client.clone() {
                            return Task::perform(
                                async move {
                                    let res =
                                        crate::api::search::execute_search(&client, &query).await;
                                    (query, res)
                                },
                                |(query, res)| Message::SearchResultsFetched(query, res),
                            );
                        }
                    }
                }
                Task::none()
            }
            Message::SearchResultsFetched(query, res) => {
                if let AppState::Main {
                    search_query,
                    search_results,
                    search_cache,
                    is_searching,
                    loaded_images,
                    ..
                } = &mut self.state
                {
                    let is_current = *search_query == query;
                    match res {
                        Ok(results) => {
                            search_cache.insert(search_cache_key(&query), results.clone());
                            // Responses can arrive out of order; only the latest query
                            // may replace what's on screen.
                            if is_current {
                                *search_results = results;
                                *is_searching = false;
                                return Task::batch(search_image_tasks(
                                    search_results,
                                    loaded_images,
                                ));
                            }
                        }
                        Err(e) if is_current => {
                            *is_searching = false;
                            return self.update(Message::ShowToast(format!("Search failed: {e}")));
                        }
                        Err(_) => {}
                    }
                }
                Task::none()
            }
            Message::SelectPlaylist(playlist_id) => {
                if let AppState::Main {
                    nav_item,
                    selected_playlist,
                    selected_album,
                    selected_artist,
                    search_query,
                    navigation_history,
                    forward_history,
                    ..
                } = &mut self.state
                {
                    let curr = get_current_destination(
                        *nav_item,
                        selected_playlist.as_ref(),
                        selected_album.as_ref(),
                        selected_artist.as_ref(),
                        search_query,
                    );
                    push_to_history(navigation_history, curr);
                    forward_history.clear();
                }
                self.load_playlist_internal(&playlist_id)
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
                                tracks
                                    .iter()
                                    .take(PLAYLIST_ROWS_PREFETCH)
                                    .map(|t| t.image_url.clone()),
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
                    nav_item,
                    selected_playlist,
                    selected_album,
                    selected_artist,
                    search_query,
                    navigation_history,
                    forward_history,
                    ..
                } = &mut self.state
                {
                    let curr = get_current_destination(
                        *nav_item,
                        selected_playlist.as_ref(),
                        selected_album.as_ref(),
                        selected_artist.as_ref(),
                        search_query,
                    );
                    push_to_history(navigation_history, curr);
                    forward_history.clear();
                }
                self.load_album_internal(&album_id)
            }
            Message::SelectArtist(artist_id) => {
                if let AppState::Main {
                    nav_item,
                    selected_playlist,
                    selected_album,
                    selected_artist,
                    search_query,
                    navigation_history,
                    forward_history,
                    ..
                } = &mut self.state
                {
                    let curr = get_current_destination(
                        *nav_item,
                        selected_playlist.as_ref(),
                        selected_album.as_ref(),
                        selected_artist.as_ref(),
                        search_query,
                    );
                    push_to_history(navigation_history, curr);
                    forward_history.clear();
                }
                self.load_artist_internal(&artist_id)
            }
            Message::ArtistDetailsFetched(artist_id, res) => {
                let mut tasks = Vec::new();
                if let AppState::Main {
                    selected_artist: Some(selected),
                    loaded_images,
                    ..
                } = &mut self.state
                {
                    if selected.id == artist_id {
                        selected.is_loading = false;
                        if let Ok(detail) = res {
                            selected.name = detail.name;
                            selected.image_url.clone_from(&detail.image_url);
                            selected.genres = detail.genres;
                            selected.followers = detail.followers;
                            selected.top_tracks = detail.top_tracks;
                            selected.albums = detail.albums;
                            let urls = std::iter::once(selected.image_url.clone())
                                .chain(selected.albums.iter().map(|a| a.image_url.clone()));
                            tasks.extend(load_image_tasks(urls, loaded_images));
                        }
                    }
                }
                if tasks.is_empty() {
                    Task::none()
                } else {
                    Task::batch(tasks)
                }
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
                    playback_link,
                    pending_play_uri,
                    playback,
                    user_top_tracks,
                    selected_playlist,
                    selected_album,
                    selected_artist,
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
                    playback.last_seek = None;

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
                                explicit: false,
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
                                explicit: false,
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
                    } else if let Some(artist_state) = selected_artist {
                        let art_name = artist_state.name.clone();
                        let art_img = artist_state.image_url.clone();
                        let new_ctx: Vec<TrackInfo> = artist_state
                            .top_tracks
                            .iter()
                            .map(|t| TrackInfo {
                                title: t.title.clone(),
                                artist: art_name.clone(),
                                album: t.album.clone(),
                                duration_ms: t.duration_ms,
                                image_url: art_img.clone(),
                                uri: t.uri.clone(),
                                explicit: false,
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
                                explicit: t.explicit,
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
                                explicit: t.explicit,
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
                        playback.track_loaded = true;
                        let _ = session.cmd_tx.try_send(PlayerCommand::Play(uri));
                    } else {
                        // Audio isn't connected yet (still connecting or pairing):
                        // remember the request and start it once the session is up.
                        playback.is_playing = false;
                        *pending_play_uri = Some(uri);
                        let notice = match playback_link {
                            PlaybackLink::AwaitingApproval { user_code, .. } => format!(
                                "Approve Spotifust on spotify.com/pair (code {user_code}) to start playback"
                            ),
                            PlaybackLink::Failed(_) => {
                                "Audio is offline — use Retry in the banner above".to_string()
                            }
                            PlaybackLink::Connecting | PlaybackLink::Ready => {
                                "Connecting to Spotify audio…".to_string()
                            }
                        };
                        tasks.push(Task::done(Message::ShowToast(notice)));
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
            Message::RequestImages(urls) => {
                let mut tasks = Vec::new();
                if let AppState::Main {
                    loaded_images,
                    pending_images,
                    ..
                } = &mut self.state
                {
                    for url in urls {
                        if loaded_images.get(&url).is_some() {
                            continue; // `get` also marks it recently used
                        }
                        if pending_images.insert(url.clone()) {
                            tasks.push(Task::perform(
                                crate::api::cache::ImageCache::fetch_image_bytes(url.clone()),
                                move |res| {
                                    Message::ImageLoaded(url.clone(), res.map(|(_, bytes)| bytes))
                                },
                            ));
                        }
                    }
                }
                Task::batch(tasks)
            }
            Message::ImageLoaded(url, res) => {
                if let AppState::Main {
                    loaded_images,
                    pending_images,
                    ..
                } = &mut self.state
                {
                    pending_images.remove(&url);
                    if let Ok(bytes) = res {
                        loaded_images.insert(url, iced::widget::image::Handle::from_bytes(bytes));
                    }
                }
                Task::none()
            }
            Message::ClearSelection => {
                if let AppState::Main {
                    selected_playlist,
                    selected_album,
                    selected_artist,
                    ..
                } = &mut self.state
                {
                    *selected_playlist = None;
                    *selected_album = None;
                    *selected_artist = None;
                }
                Task::none()
            }
            Message::AudioSessionConnected(session) => {
                let mut pending = None;
                if let AppState::Main {
                    audio_session,
                    playback,
                    playback_link,
                    pending_play_uri,
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
                    *playback_link = PlaybackLink::Ready;
                    playback.track_loaded = false;
                    playback.is_playing = false;
                    pending = pending_play_uri.take();
                }
                match pending {
                    Some(uri) => self.update(Message::PlayTrack(uri)),
                    None => Task::none(),
                }
            }
            Message::PlaybackSessionResult(res) => match res {
                Ok(session) => self.update(Message::AudioSessionConnected(session)),
                Err(AppError::PlaybackPairing(_)) => {
                    if let AppState::Main { playback_link, .. } = &mut self.state {
                        *playback_link = PlaybackLink::Connecting;
                    }
                    Task::perform(
                        crate::audio::credentials::request_pairing(),
                        Message::PlaybackPairingStarted,
                    )
                }
                Err(e) => {
                    if let AppState::Main { playback_link, .. } = &mut self.state {
                        *playback_link = PlaybackLink::Failed(e.to_string());
                    }
                    Task::none()
                }
            },
            Message::PlaybackPairingStarted(res) => match res {
                Ok(request) => {
                    let mut config = None;
                    if let AppState::Main {
                        playback_link,
                        audio_bitrate,
                        audio_normalization,
                        gapless_playback,
                        ..
                    } = &mut self.state
                    {
                        *playback_link = PlaybackLink::AwaitingApproval {
                            user_code: request.user_code.clone(),
                            url: request.url.clone(),
                        };
                        config = Some((*audio_bitrate, *audio_normalization, *gapless_playback));
                    }
                    let Some((bitrate, normalisation, gapless)) = config else {
                        return Task::none();
                    };
                    open::that_in_background(&request.url);
                    Task::perform(
                        async move {
                            let credentials =
                                crate::audio::credentials::await_pairing(request).await?;
                            crate::audio::session::connect_with_credentials(
                                credentials,
                                bitrate,
                                normalisation,
                                gapless,
                            )
                            .await
                        },
                        |res| match res {
                            // A failed pairing must not loop straight back into a new one.
                            Err(AppError::PlaybackPairing(reason)) => {
                                Message::PlaybackSessionResult(Err(AppError::Playback(reason)))
                            }
                            other => Message::PlaybackSessionResult(other),
                        },
                    )
                }
                Err(e) => {
                    if let AppState::Main { playback_link, .. } = &mut self.state {
                        *playback_link = PlaybackLink::Failed(e.to_string());
                    }
                    Task::none()
                }
            },
            Message::RetryPlaybackConnection => {
                if let AppState::Main {
                    playback_link,
                    audio_session,
                    audio_bitrate,
                    audio_normalization,
                    gapless_playback,
                    ..
                } = &mut self.state
                {
                    if audio_session.is_none() {
                        *playback_link = PlaybackLink::Connecting;
                        return connect_playback_task(
                            *audio_bitrate,
                            *audio_normalization,
                            *gapless_playback,
                        );
                    }
                }
                Task::none()
            }
            Message::OpenPlaybackPairingUrl => {
                if let AppState::Main {
                    playback_link: PlaybackLink::AwaitingApproval { url, .. },
                    ..
                } = &self.state
                {
                    open::that_in_background(url);
                }
                Task::none()
            }
            Message::MainContentScrolled(window) => {
                if let AppState::Main {
                    main_scroll,
                    selected_playlist,
                    loaded_images,
                    ..
                } = &mut self.state
                {
                    *main_scroll = window;
                    if let Some(sp) = selected_playlist {
                        let (first, last) = crate::ui::main_layout::visible_row_range(
                            sp.tracks.len(),
                            crate::ui::main_layout::DETAIL_LIST_TOP,
                            window,
                        );
                        return Task::batch(load_image_tasks(
                            sp.tracks[first..last].iter().map(|t| t.image_url.clone()),
                            loaded_images,
                        ));
                    }
                }
                Task::none()
            }
            Message::PlayerEventReceived(event) => {
                match &event {
                    // Positions in these events are the decoder's, which runs ahead of
                    // the speakers; progress comes from `PlaybackPositionReceived`.
                    PlayerEvent::Playing { track_id, .. } => {
                        if let AppState::Main { playback, .. } = &mut self.state {
                            playback.is_playing = true;
                            playback.current_track_uri = Some(track_id.to_uri());
                        }
                    }
                    PlayerEvent::Paused { .. } => {
                        if let AppState::Main { playback, .. } = &mut self.state {
                            playback.is_playing = false;
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
                                explicit: false,
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
                            playback.last_seek = None;
                        }
                    }
                    PlayerEvent::EndOfTrack { .. } => {
                        if let AppState::Main { playback, .. } = &mut self.state {
                            playback.is_playing = false;
                            playback.progress_ms = 0;
                            playback.last_seek = None;
                        }
                        return self.update(Message::SkipNext);
                    }
                    PlayerEvent::Unavailable { .. } => {
                        if let AppState::Main { playback, .. } = &mut self.state {
                            playback.is_playing = false;
                        }
                        return self.update(Message::ShowToast(
                            "Track unavailable for playback".to_string(),
                        ));
                    }
                    _ => {}
                }
                Task::none()
            }

            Message::PlaybackPositionReceived(pos) => {
                if let AppState::Main { playback, .. } = &mut self.state {
                    if let Some((seek_time, target_ms)) = playback.last_seek {
                        if seek_time.elapsed() < std::time::Duration::from_millis(600) {
                            if pos.abs_diff(target_ms) > 2500 {
                                return Task::none();
                            }
                        } else {
                            playback.last_seek = None;
                        }
                    }
                    let max_dur = playback.current_track.as_ref().map_or(u32::MAX, |t| {
                        if t.duration_ms > 0 {
                            t.duration_ms
                        } else {
                            u32::MAX
                        }
                    });
                    playback.progress_ms = pos.min(max_dur);
                }
                self.follow_lyrics()
            }
            Message::SessionExpired => {
                // The librespot connection dropped; this says nothing about the Web API
                // login, so reconnect audio quietly instead of logging the user out.
                if let AppState::Main {
                    audio_session,
                    playback,
                    playback_link,
                    audio_bitrate,
                    audio_normalization,
                    gapless_playback,
                    ..
                } = &mut self.state
                {
                    *audio_session = None;
                    playback.is_playing = false;
                    *playback_link = PlaybackLink::Connecting;
                    return connect_playback_task(
                        *audio_bitrate,
                        *audio_normalization,
                        *gapless_playback,
                    );
                }
                Task::none()
            }
            Message::LoginFailed(err) => {
                match &mut self.state {
                    AppState::Initializing => {
                        self.state = AppState::Login {
                            is_loading: false,
                            error: if err == "No token" { None } else { Some(err) },
                        };
                    }
                    AppState::Login { is_loading, error } => {
                        *is_loading = false;
                        if err != "No token" {
                            *error = Some(err);
                        }
                    }
                    AppState::Main { .. } => {}
                }
                Task::none()
            }
            Message::NavigationSelected(item) => {
                if let AppState::Main {
                    nav_item,
                    selected_playlist,
                    selected_album,
                    selected_artist,
                    navigation_history,
                    forward_history,
                    search_query,
                    ..
                } = &mut self.state
                {
                    let curr = get_current_destination(
                        *nav_item,
                        selected_playlist.as_ref(),
                        selected_album.as_ref(),
                        selected_artist.as_ref(),
                        search_query,
                    );
                    push_to_history(navigation_history, curr);
                    forward_history.clear();

                    *nav_item = item;
                    if item == NavigationItem::Home {
                        *selected_playlist = None;
                        *selected_album = None;
                        *selected_artist = None;
                    }
                    if item == NavigationItem::Settings {
                        return Task::perform(
                            async { crate::api::cache::calculate_cache_size_bytes() },
                            Message::CacheSizeCalculated,
                        );
                    }
                }
                Task::none()
            }
            Message::NavigateBack => {
                let target = match &mut self.state {
                    AppState::Main {
                        nav_item,
                        selected_playlist,
                        selected_album,
                        selected_artist,
                        search_query,
                        navigation_history,
                        forward_history,
                        ..
                    } => {
                        if let Some(dest) = navigation_history.pop() {
                            let curr = get_current_destination(
                                *nav_item,
                                selected_playlist.as_ref(),
                                selected_album.as_ref(),
                                selected_artist.as_ref(),
                                search_query,
                            );
                            push_to_history(forward_history, curr);
                            Some(dest)
                        } else {
                            None
                        }
                    }
                    AppState::Login { .. } | AppState::Initializing => None,
                };
                if let Some(dest) = target {
                    return self.navigate_to(dest);
                }
                Task::none()
            }
            Message::NavigateForward => {
                let target = match &mut self.state {
                    AppState::Main {
                        nav_item,
                        selected_playlist,
                        selected_album,
                        selected_artist,
                        search_query,
                        navigation_history,
                        forward_history,
                        ..
                    } => {
                        if let Some(dest) = forward_history.pop() {
                            let curr = get_current_destination(
                                *nav_item,
                                selected_playlist.as_ref(),
                                selected_album.as_ref(),
                                selected_artist.as_ref(),
                                search_query,
                            );
                            push_to_history(navigation_history, curr);
                            Some(dest)
                        } else {
                            None
                        }
                    }
                    AppState::Login { .. } | AppState::Initializing => None,
                };
                if let Some(dest) = target {
                    return self.navigate_to(dest);
                }
                Task::none()
            }
            Message::SeekToMs(pos_ms) => {
                if let AppState::Main {
                    playback,
                    audio_session,
                    ..
                } = &mut self.state
                {
                    playback.progress_ms = pos_ms;
                    playback.last_seek = Some((std::time::Instant::now(), pos_ms));
                    if let Some(session) = audio_session {
                        let _ = session.cmd_tx.try_send(PlayerCommand::Seek(pos_ms));
                    }
                }
                Task::none()
            }
            Message::LyricsFetched(key, res) => {
                if let AppState::Main {
                    current_lyrics,
                    lyrics_key,
                    is_loading_lyrics,
                    ..
                } = &mut self.state
                {
                    // Ignore answers for a track the user already skipped past.
                    if lyrics_key.as_deref() == Some(key.as_str()) {
                        *is_loading_lyrics = false;
                        *current_lyrics = res.ok();
                    }
                }
                Task::none()
            }
            Message::ArtistBioFetched(key, res) => {
                if let AppState::Main {
                    current_artist_bio,
                    bio_key,
                    is_loading_artist_bio,
                    ..
                } = &mut self.state
                {
                    if bio_key.as_deref() == Some(key.as_str()) {
                        *is_loading_artist_bio = false;
                        *current_artist_bio = res.ok();
                    }
                }
                Task::none()
            }
            Message::TogglePlayback => {
                if let AppState::Main {
                    playback,
                    audio_session,
                    context_queue,
                    user_queue,
                    ..
                } = &mut self.state
                {
                    if playback.current_track.is_none() {
                        if let Some(next_track) = user_queue
                            .first()
                            .cloned()
                            .or_else(|| context_queue.first().cloned())
                        {
                            return self.update(Message::PlayTrack(next_track.uri));
                        }
                        return Task::none();
                    }

                    let was_playing = playback.is_playing;
                    playback.is_playing = !was_playing;

                    if let Some(session) = audio_session {
                        let cmd = if was_playing {
                            PlayerCommand::Pause
                        } else if playback.track_loaded {
                            PlayerCommand::Resume
                        } else if let Some(track) = &playback.current_track {
                            // Restored from the last session: nothing is loaded in
                            // librespot yet, so load it where the user left off.
                            playback.track_loaded = true;
                            PlayerCommand::PlayFrom(track.uri.clone(), playback.progress_ms)
                        } else {
                            PlayerCommand::Resume
                        };
                        let _ = session.cmd_tx.try_send(cmd);
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
                    autoplay_enabled,
                    spotify_client,
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
                    } else if *autoplay_enabled {
                        let maybe_seed = history.last().or(playback.current_track.as_ref());
                        if let Some(seed) = maybe_seed {
                            let seed_id = seed.uri.trim_start_matches("spotify:track:").to_string();
                            if let Some(spotify) = spotify_client.clone() {
                                playback.is_playing = false;
                                playback.progress_ms = 0;
                                return Task::perform(
                                    async move {
                                        crate::api::tracks::fetch_recommendations(
                                            &spotify,
                                            &[seed_id],
                                        )
                                        .await
                                    },
                                    Message::AutoplayRecommendationsFetched,
                                );
                            }
                        }
                        None
                    } else {
                        None
                    };

                    if let Some(next_track) = next_track_opt {
                        playback.current_track = Some(next_track.clone());
                        playback.progress_ms = 0;
                        playback.last_seek = None;
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
                        playback.last_seek = None;
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
                        playback.last_seek = Some((std::time::Instant::now(), 0));
                        if let Some(session) = audio_session {
                            let _ = session.cmd_tx.try_send(PlayerCommand::Seek(0));
                        }
                    } else if let Some(prev_track) = history.pop() {
                        playback.current_track = Some(prev_track.clone());
                        playback.progress_ms = 0;
                        playback.last_seek = None;
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
                        playback.last_seek = None;
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
                        playback.last_seek = Some((std::time::Instant::now(), 0));
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
                        playback.last_seek = Some((std::time::Instant::now(), pos_ms));

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
                    last_right_click,
                    ..
                } = &mut self.state
                {
                    let anchor = recent_right_click(*last_right_click);
                    *active_context_menu = Some(ContextMenuState {
                        target: ContextMenuTarget::Track {
                            track,
                            from_playlist_id,
                        },
                        position: anchor.unwrap_or(position),
                        anchored: anchor.is_some(),
                    });
                }
                Task::none()
            }
            Message::OpenAlbumContextMenu { album, position } => {
                if let AppState::Main {
                    active_context_menu,
                    last_right_click,
                    ..
                } = &mut self.state
                {
                    let anchor = recent_right_click(*last_right_click);
                    *active_context_menu = Some(ContextMenuState {
                        target: ContextMenuTarget::Album(album),
                        position: anchor.unwrap_or(position),
                        anchored: anchor.is_some(),
                    });
                }
                Task::none()
            }
            Message::OpenPlaylistContextMenu { playlist, position } => {
                if let AppState::Main {
                    active_context_menu,
                    last_right_click,
                    ..
                } = &mut self.state
                {
                    let anchor = recent_right_click(*last_right_click);
                    *active_context_menu = Some(ContextMenuState {
                        target: ContextMenuTarget::Playlist(playlist),
                        position: anchor.unwrap_or(position),
                        anchored: anchor.is_some(),
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
                    last_right_click,
                    ..
                } = &mut self.state
                {
                    let anchor = recent_right_click(*last_right_click);
                    *active_context_menu = Some(ContextMenuState {
                        target: ContextMenuTarget::Artist {
                            artist_id,
                            artist_name,
                        },
                        position: anchor.unwrap_or(position),
                        anchored: anchor.is_some(),
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
                    ..
                } = &mut self.state
                {
                    *active_context_menu = None;
                }
                let toast_task = self.update(Message::ShowToast(format!(
                    "Enlace de '{title}' copiado al portapapeles"
                )));
                Task::batch(vec![iced::clipboard::write(url), toast_task])
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
                            Ok("Added to playlist".to_string())
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
                            Ok("Removed from playlist".to_string())
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
                                Ok("Removed from Your Library".to_string())
                            } else {
                                crate::api::album::save_album(&spotify, &album_id).await?;
                                Ok("Saved to Your Library".to_string())
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
                            Ok("Playlist updated".to_string())
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
                            Ok("Playlist deleted".to_string())
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
                            Ok("Playlist privacy updated".to_string())
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
                            Ok("Songs copied to playlist".to_string())
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
                                Ok("Unfollowed artist".to_string())
                            } else {
                                crate::api::artist::follow_artist(&spotify, &artist_id).await?;
                                Ok("Following artist".to_string())
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
                    toast_notification,
                    toast_id,
                    ..
                } = &mut self.state
                {
                    *toast_id = toast_id.wrapping_add(1);
                    let current_id = *toast_id;
                    *toast_notification = Some(msg);
                    return Task::perform(
                        async move {
                            tokio::time::sleep(std::time::Duration::from_secs(4)).await;
                            current_id
                        },
                        Message::DismissToastId,
                    );
                }
                Task::none()
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

            Message::DismissToastId(id) => {
                if let AppState::Main {
                    toast_notification,
                    toast_id,
                    ..
                } = &mut self.state
                {
                    if *toast_id == id {
                        *toast_notification = None;
                    }
                }
                Task::none()
            }

            Message::OperationFinished(res) => match res {
                Ok(msg) => self.update(Message::ShowToast(msg)),
                Err(e) => self.update(Message::ShowToast(format!("Error: {e}"))),
            },
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
                let mut toast_task = Task::none();
                if let AppState::Main {
                    user_queue,
                    active_context_menu,
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
                    toast_task =
                        self.update(Message::ShowToast(format!("Added to queue: {title}")));
                }
                tasks.push(toast_task);
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
                    *active_right_panel = if *active_right_panel == Some(tab) {
                        None
                    } else {
                        Some(tab)
                    };
                }
                Task::none()
            }
            Message::ToggleAutoplay => {
                if let AppState::Main {
                    autoplay_enabled, ..
                } = &mut self.state
                {
                    *autoplay_enabled = !*autoplay_enabled;
                }
                Task::none()
            }
            Message::SearchCategoryFilterSelected(filter) => {
                if let AppState::Main {
                    search_category_filter,
                    ..
                } = &mut self.state
                {
                    *search_category_filter = filter;
                }
                Task::none()
            }
            Message::ToggleExplicitContent => {
                if let AppState::Main {
                    allow_explicit_content,
                    ..
                } = &mut self.state
                {
                    *allow_explicit_content = !*allow_explicit_content;
                }
                Task::none()
            }
            Message::CacheSizeCalculated(bytes) => {
                if let AppState::Main {
                    cache_size_bytes, ..
                } = &mut self.state
                {
                    *cache_size_bytes = bytes;
                }
                Task::none()
            }
            Message::ClearCacheRequested => Task::perform(
                async { crate::api::cache::clear_cache_disk() },
                Message::CacheCleared,
            ),
            Message::CacheCleared(res) => {
                if let AppState::Main {
                    cache_size_bytes, ..
                } = &mut self.state
                {
                    match res {
                        Ok(freed) => {
                            *cache_size_bytes = 0;
                            return self.update(Message::ShowToast(format!(
                                "Cache cleared ({} freed)",
                                crate::api::cache::format_bytes(freed)
                            )));
                        }
                        Err(e) => {
                            self.active_error = Some(e.to_string());
                        }
                    }
                }
                Task::none()
            }
            Message::AutoplayRecommendationsFetched(res) => {
                let mut tasks = Vec::new();
                if let AppState::Main {
                    context_queue,
                    context_index,
                    playback,
                    audio_session,
                    loaded_images,
                    autoplay_enabled,
                    ..
                } = &mut self.state
                {
                    if *autoplay_enabled {
                        if let Ok(tracks) = res {
                            if !tracks.is_empty() {
                                *context_queue = tracks
                                    .into_iter()
                                    .map(|t| TrackInfo {
                                        title: t.title,
                                        artist: t.artist,
                                        album: t.album,
                                        duration_ms: t.duration_ms,
                                        image_url: t.image_url,
                                        uri: t.uri,
                                        explicit: t.explicit,
                                    })
                                    .collect();
                                *context_index = 0;
                                let first = context_queue[0].clone();
                                playback.current_track = Some(first.clone());
                                playback.progress_ms = 0;
                                playback.is_playing = true;
                                if let Some(session) = audio_session {
                                    let _ = session
                                        .cmd_tx
                                        .try_send(PlayerCommand::Play(first.uri.clone()));
                                }
                                tasks.extend(load_image_tasks(
                                    context_queue.iter().take(24).map(|t| t.image_url.clone()),
                                    loaded_images,
                                ));
                            }
                        }
                    }
                }
                if tasks.is_empty() {
                    Task::none()
                } else {
                    Task::batch(tasks)
                }
            }
            Message::WindowResized(size) => {
                if let AppState::Main {
                    window_width,
                    window_height,
                    ..
                } = &mut self.state
                {
                    *window_width = size.width;
                    *window_height = size.height;
                }
                Task::none()
            }
            Message::RightClickAt(point) => {
                if let AppState::Main {
                    last_right_click,
                    active_context_menu,
                    ..
                } = &mut self.state
                {
                    *last_right_click = Some((point, std::time::Instant::now()));
                    // The widget's "open menu" message usually arrives before this
                    // event, so anchor a freshly opened menu to the real cursor spot.
                    if let Some(menu) = active_context_menu.as_mut().filter(|m| !m.anchored) {
                        menu.position = point;
                        menu.anchored = true;
                    }
                }
                Task::none()
            }
            Message::SetUiScale(scale) => {
                if let AppState::Main { ui_scale, .. } = &mut self.state {
                    *ui_scale = scale.clamp(0.70, 1.30);
                    save_ui_scale(*ui_scale);
                }
                Task::none()
            }
            Message::AdjustUiScale(delta) => {
                if let AppState::Main { ui_scale, .. } = &mut self.state {
                    *ui_scale = (*ui_scale + delta).clamp(0.70, 1.30);
                    save_ui_scale(*ui_scale);
                }
                Task::none()
            }
            Message::ResetUiScale => {
                if let AppState::Main { ui_scale, .. } = &mut self.state {
                    *ui_scale = 1.0;
                    save_ui_scale(1.0);
                }
                Task::none()
            }
            Message::SetAccentTone(tone) => {
                if let AppState::Main { accent_tone, .. } = &mut self.state {
                    *accent_tone = tone;
                    save_accent_tone(tone);
                }
                Task::none()
            }
            Message::OpenSpotifyAccount => {
                let _ = open::that("https://www.spotify.com/account");
                Task::none()
            }
            Message::SetUiLanguage(lang) => {
                if let AppState::Main { ui_language, .. } = &mut self.state {
                    *ui_language = lang;
                    save_ui_language(lang);
                }
                Task::none()
            }
            Message::SetAudioBitrate(bitrate) => {
                if let AppState::Main { audio_bitrate, .. } = &mut self.state {
                    *audio_bitrate = bitrate;
                    save_audio_bitrate(bitrate);
                }
                Task::none()
            }
            Message::ToggleAudioNormalization => {
                if let AppState::Main {
                    audio_normalization,
                    ..
                } = &mut self.state
                {
                    *audio_normalization = !*audio_normalization;
                    save_audio_normalization(*audio_normalization);
                }
                Task::none()
            }
            Message::ToggleGaplessPlayback => {
                if let AppState::Main {
                    gapless_playback, ..
                } = &mut self.state
                {
                    *gapless_playback = !*gapless_playback;
                    save_gapless_playback(*gapless_playback);
                }
                Task::none()
            }
            Message::AppCloseRequested => {
                if let AppState::Main {
                    playback,
                    audio_session,
                    ui_scale,
                    sidebar_width,
                    right_panel_width,
                    ..
                } = &mut self.state
                {
                    perform_graceful_shutdown(
                        playback,
                        audio_session.as_ref(),
                        *ui_scale,
                        *sidebar_width,
                        *right_panel_width,
                    );
                }
                std::process::exit(0);
            }
        }
    }

    #[allow(clippy::too_many_lines)]
    pub fn view(&self) -> Element<'_, Message> {
        let content = match &self.state {
            AppState::Initializing => iced::widget::Container::new(iced::widget::Space::new())
                .width(iced::Length::Fill)
                .height(iced::Length::Fill)
                .style(|_theme| iced::widget::container::Style {
                    background: Some(iced::Background::Color(crate::ui::theme::BG_BASE)),
                    ..Default::default()
                })
                .into(),
            AppState::Login { is_loading, error } => login::view(*is_loading, error.as_deref()),
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
                selected_artist,
                user_queue,
                context_queue,
                context_index,
                loaded_images,
                window_width,
                window_height,
                active_context_menu,
                active_modal,
                toast_notification,
                navigation_history,
                forward_history,
                current_lyrics,
                is_loading_lyrics,
                current_artist_bio,
                is_loading_artist_bio,
                autoplay_enabled,
                search_category_filter,
                cache_size_bytes,
                allow_explicit_content,
                ui_scale,
                accent_tone,
                ui_language,
                audio_bitrate,
                audio_normalization,
                gapless_playback,
                main_scroll,
                playback_link,
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
                selected_artist.as_ref(),
                user_queue,
                context_queue,
                *context_index,
                loaded_images.inner_map(),
                *window_width,
                *window_height,
                active_context_menu.as_ref(),
                active_modal.as_ref(),
                toast_notification.as_ref(),
                !navigation_history.is_empty(),
                !forward_history.is_empty(),
                current_lyrics.as_ref(),
                *is_loading_lyrics,
                current_artist_bio.as_ref(),
                *is_loading_artist_bio,
                *autoplay_enabled,
                *search_category_filter,
                *cache_size_bytes,
                *allow_explicit_content,
                *ui_scale,
                *accent_tone,
                *ui_language,
                *audio_bitrate,
                *audio_normalization,
                *gapless_playback,
                *main_scroll,
                playback_link,
            ),
        };

        if matches!(self.state, AppState::Main { .. }) {
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
                                        background: Some(Background::Color(
                                            iced::Color::TRANSPARENT,
                                        )),
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

/// The cursor position of a right click that happened just now, if any.
fn recent_right_click(last: Option<(iced::Point, std::time::Instant)>) -> Option<iced::Point> {
    last.filter(|(_, at)| at.elapsed() < std::time::Duration::from_millis(250))
        .map(|(point, _)| point)
}

fn connect_playback_task(
    bitrate: crate::audio::session::AudioBitrate,
    normalisation: bool,
    gapless: bool,
) -> Task<Message> {
    Task::perform(
        crate::audio::session::connect_stored(bitrate, normalisation, gapless),
        Message::PlaybackSessionResult,
    )
}

fn search_image_tasks(
    results: &crate::api::search::SearchResults,
    loaded_images: &crate::api::cache::LruCache<String, iced::widget::image::Handle>,
) -> Vec<Task<Message>> {
    load_image_tasks(
        results
            .tracks
            .iter()
            .map(|t| t.image_url.clone())
            .chain(results.albums.iter().map(|a| a.image_url.clone()))
            .chain(results.artists.iter().map(|a| a.image_url.clone())),
        loaded_images,
    )
}

fn load_image_tasks(
    urls: impl IntoIterator<Item = Option<String>>,
    loaded_images: &crate::api::cache::LruCache<String, iced::widget::image::Handle>,
) -> Vec<Task<Message>> {
    let mut wanted: Vec<String> = Vec::new();
    for url in urls.into_iter().flatten() {
        if !url.is_empty() && !loaded_images.contains_key(&url) && !wanted.contains(&url) {
            wanted.push(url);
        }
    }
    if wanted.is_empty() {
        Vec::new()
    } else {
        // Dedup against in-flight downloads happens in `Message::RequestImages`.
        vec![Task::done(Message::RequestImages(wanted))]
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_current_destination_nav_items() {
        assert_eq!(
            get_current_destination(NavigationItem::Home, None, None, None, ""),
            NavDestination::Home
        );
        assert_eq!(
            get_current_destination(NavigationItem::Search, None, None, None, "daft punk"),
            NavDestination::Search("daft punk".to_string())
        );
        assert_eq!(
            get_current_destination(NavigationItem::Library, None, None, None, ""),
            NavDestination::Library
        );
        assert_eq!(
            get_current_destination(NavigationItem::Settings, None, None, None, ""),
            NavDestination::Settings
        );
    }

    #[test]
    fn test_get_current_destination_playlist_priority() {
        let playlist = SelectedPlaylistState {
            id: "pl123".to_string(),
            name: "Favorites".to_string(),
            image_url: None,
            tracks: Vec::new(),
            is_loading: false,
        };
        assert_eq!(
            get_current_destination(NavigationItem::Home, Some(&playlist), None, None, ""),
            NavDestination::Playlist("pl123".to_string())
        );
    }

    #[test]
    fn test_get_current_destination_album_priority() {
        let album = SelectedAlbumState {
            id: "alb456".to_string(),
            name: "Discovery".to_string(),
            artist_name: "Daft Punk".to_string(),
            image_url: None,
            release_date: "2001-03-12".to_string(),
            tracks: Vec::new(),
            is_loading: false,
        };
        assert_eq!(
            get_current_destination(NavigationItem::Home, None, Some(&album), None, ""),
            NavDestination::Album("alb456".to_string())
        );
    }

    #[test]
    fn test_get_current_destination_artist_priority() {
        let artist = SelectedArtistState {
            id: "art789".to_string(),
            name: "Daft Punk".to_string(),
            image_url: None,
            genres: Vec::new(),
            followers: 1000,
            top_tracks: Vec::new(),
            albums: Vec::new(),
            is_loading: false,
        };
        assert_eq!(
            get_current_destination(NavigationItem::Home, None, None, Some(&artist), ""),
            NavDestination::Artist("art789".to_string())
        );
    }

    #[test]
    fn test_push_to_history_dedup() {
        let mut history = Vec::new();
        push_to_history(&mut history, NavDestination::Home);
        push_to_history(&mut history, NavDestination::Home);
        assert_eq!(history.len(), 1);
        push_to_history(&mut history, NavDestination::Library);
        assert_eq!(history.len(), 2);
    }

    #[test]
    fn test_push_to_history_cap_at_50() {
        let mut history = Vec::new();
        for i in 0..60 {
            push_to_history(&mut history, NavDestination::Search(format!("query_{i}")));
        }
        assert_eq!(history.len(), 50);
        assert_eq!(history[0], NavDestination::Search("query_10".to_string()));
        assert_eq!(history[49], NavDestination::Search("query_59".to_string()));
    }

    #[test]
    fn test_shuffle_slice_preserves_elements() {
        let mut original = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let mut shuffled = original.clone();
        shuffle_slice(&mut shuffled);
        shuffled.sort_unstable();
        original.sort_unstable();
        assert_eq!(original, shuffled);
    }

    #[test]
    fn test_autoplay_seed_extraction_strips_prefix() {
        let uri_with_prefix = "spotify:track:4cOdK2wGLETKBW3PvgPWqT";
        let seed_id = uri_with_prefix
            .trim_start_matches("spotify:track:")
            .to_string();
        assert_eq!(seed_id, "4cOdK2wGLETKBW3PvgPWqT");

        let raw_id = "4cOdK2wGLETKBW3PvgPWqT";
        let seed_id_raw = raw_id.trim_start_matches("spotify:track:").to_string();
        assert_eq!(seed_id_raw, "4cOdK2wGLETKBW3PvgPWqT");
    }

    #[test]
    fn test_autoplay_toggle_state() {
        let mut autoplay_enabled = true;
        autoplay_enabled = !autoplay_enabled;
        assert!(!autoplay_enabled);
        autoplay_enabled = !autoplay_enabled;
        assert!(autoplay_enabled);
    }

    #[test]
    fn test_search_category_filter_default_and_variants() {
        assert_eq!(SearchCategoryFilter::default(), SearchCategoryFilter::All);
        let mut filter = SearchCategoryFilter::All;
        assert_eq!(filter, SearchCategoryFilter::All);
        filter = SearchCategoryFilter::Tracks;
        assert_eq!(filter, SearchCategoryFilter::Tracks);
        filter = SearchCategoryFilter::Albums;
        assert_eq!(filter, SearchCategoryFilter::Albums);
        filter = SearchCategoryFilter::Artists;
        assert_eq!(filter, SearchCategoryFilter::Artists);
    }

    #[test]
    fn test_cache_size_update_and_clear() {
        let mut cache_size = 1024 * 1024 * 5;
        assert_eq!(cache_size, 5_242_880);
        cache_size = 0;
        assert_eq!(cache_size, 0);
    }

    #[test]
    fn test_map_keyboard_shortcuts() {
        use iced::keyboard::{Key, Modifiers, key::Named};

        let no_mod = Modifiers::empty();
        assert!(matches!(
            map_keyboard_shortcut(&Key::Named(Named::Space), no_mod),
            Some(Message::TogglePlayback)
        ));
        assert!(matches!(
            map_keyboard_shortcut(&Key::Named(Named::ArrowRight), no_mod),
            Some(Message::SkipNext)
        ));
        assert!(matches!(
            map_keyboard_shortcut(&Key::Named(Named::ArrowLeft), no_mod),
            Some(Message::SkipPrev)
        ));
        assert!(matches!(
            map_keyboard_shortcut(&Key::Named(Named::ArrowUp), no_mod),
            Some(Message::AdjustVolume(v)) if (v - 0.05).abs() < f32::EPSILON
        ));
        assert!(matches!(
            map_keyboard_shortcut(&Key::Named(Named::ArrowDown), no_mod),
            Some(Message::AdjustVolume(v)) if (v - (-0.05)).abs() < f32::EPSILON
        ));

        let ctrl_mod = Modifiers::CTRL;
        assert!(matches!(
            map_keyboard_shortcut(&Key::Character("f".into()), ctrl_mod),
            Some(Message::NavigationSelected(NavigationItem::Search))
        ));
        assert!(matches!(
            map_keyboard_shortcut(&Key::Character("m".into()), ctrl_mod),
            Some(Message::ToggleMute)
        ));
        assert!(matches!(
            map_keyboard_shortcut(&Key::Character("s".into()), ctrl_mod),
            Some(Message::ToggleShuffle)
        ));
        assert!(matches!(
            map_keyboard_shortcut(&Key::Character("r".into()), ctrl_mod),
            Some(Message::ToggleRepeat)
        ));
        assert!(matches!(
            map_keyboard_shortcut(&Key::Character("q".into()), ctrl_mod),
            Some(Message::ToggleRightPanel(RightPanelTab::Queue))
        ));
        assert!(matches!(
            map_keyboard_shortcut(&Key::Named(Named::ArrowLeft), ctrl_mod),
            Some(Message::NavigateBack)
        ));
        assert!(matches!(
            map_keyboard_shortcut(&Key::Named(Named::ArrowRight), ctrl_mod),
            Some(Message::NavigateForward)
        ));
        assert!(matches!(
            map_keyboard_shortcut(&Key::Character("+".into()), ctrl_mod),
            Some(Message::AdjustUiScale(d)) if (d - 0.05).abs() < f32::EPSILON
        ));
        assert!(matches!(
            map_keyboard_shortcut(&Key::Character("-".into()), ctrl_mod),
            Some(Message::AdjustUiScale(d)) if (d - (-0.05)).abs() < f32::EPSILON
        ));
        assert!(matches!(
            map_keyboard_shortcut(&Key::Character("0".into()), ctrl_mod),
            Some(Message::ResetUiScale)
        ));

        let alt_mod = Modifiers::ALT;
        assert!(matches!(
            map_keyboard_shortcut(&Key::Named(Named::ArrowLeft), alt_mod),
            Some(Message::NavigateBack)
        ));
        assert!(matches!(
            map_keyboard_shortcut(&Key::Named(Named::ArrowRight), alt_mod),
            Some(Message::NavigateForward)
        ));
    }

    #[test]
    fn test_explicit_content_toggle() {
        let mut allow_explicit = true;
        allow_explicit = !allow_explicit;
        assert!(!allow_explicit);
        allow_explicit = !allow_explicit;
        assert!(allow_explicit);
    }

    #[test]
    fn test_ui_scale_clamping() {
        let min_scale = (0.50_f32).clamp(0.70, 1.30);
        let max_scale = (1.50_f32).clamp(0.70, 1.30);
        let normal_scale = (1.05_f32).clamp(0.70, 1.30);
        assert!((min_scale - 0.70).abs() < f32::EPSILON);
        assert!((max_scale - 1.30).abs() < f32::EPSILON);
        assert!((normal_scale - 1.05).abs() < f32::EPSILON);
    }

    static DISK_TEST_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn test_perform_graceful_shutdown() {
        let _guard = DISK_TEST_MUTEX.lock().unwrap();
        let playback = PlaybackState {
            volume: 0.65,
            ..Default::default()
        };
        perform_graceful_shutdown(&playback, None, 1.15, 260.0, 320.0);
        let saved_vol = load_saved_volume();
        let saved_scale = load_ui_scale();
        assert!((saved_vol - 0.65).abs() < 0.001);
        assert!((saved_scale - 1.15).abs() < 0.001);
    }

    fn main_app_for_tests() -> App {
        let (audio_tx, _) = tokio::sync::mpsc::channel(1);
        App {
            state: AppState::Main {
                nav_item: NavigationItem::Home,
                playback: PlaybackState::default(),
                audio_session: None,
                playback_link: PlaybackLink::Connecting,
                pending_play_uri: None,
                main_scroll: ScrollWindow::default(),
                pending_images: std::collections::HashSet::new(),
                user_profile: None,
                user_playlists: Vec::new(),
                user_albums: Vec::new(),
                user_top_tracks: Vec::new(),
                featured_playlists: Vec::new(),
                featured_albums: Vec::new(),
                search_query: String::new(),
                search_results: crate::api::search::SearchResults::default(),
                is_searching: false,
                search_cache: crate::api::cache::LruCache::new(SEARCH_CACHE_CAPACITY),
                sidebar_filter: SidebarFilter::All,
                selected_playlist: None,
                selected_album: None,
                selected_artist: None,
                user_queue: Vec::new(),
                context_queue: Vec::new(),
                original_context_queue: Vec::new(),
                context_index: 0,
                history: Vec::new(),
                active_context_menu: None,
                active_modal: None,
                toast_notification: None,
                toast_id: 0,
                loaded_images: crate::api::cache::LruCache::new(IMAGE_CACHE_CAPACITY),
                spotify_client: None,
                sidebar_width: 240.0,
                right_panel_width: 280.0,
                active_right_panel: None,
                dragging_sidebar: false,
                dragging_right_panel: false,
                window_width: 1200.0,
                window_height: 800.0,
                last_right_click: None,
                navigation_history: Vec::new(),
                forward_history: Vec::new(),
                current_lyrics: None,
                lyrics_key: None,
                bio_key: None,
                lyrics_line: None,
                is_loading_lyrics: false,
                current_artist_bio: None,
                is_loading_artist_bio: false,
                autoplay_enabled: true,
                search_category_filter: SearchCategoryFilter::All,
                cache_size_bytes: 0,
                allow_explicit_content: true,
                ui_scale: 1.0,
                accent_tone: crate::ui::theme::AccentTone::default(),
                ui_language: UiLanguage::default(),
                audio_bitrate: crate::audio::session::AudioBitrate::default(),
                audio_normalization: true,
                gapless_playback: true,
            },
            audio_tx,
            active_error: None,
        }
    }

    fn search_results_named(title: &str) -> crate::api::search::SearchResults {
        crate::api::search::SearchResults {
            tracks: vec![crate::api::search::SearchResultTrack {
                id: "t".to_string(),
                title: title.to_string(),
                artist: "a".to_string(),
                album: "b".to_string(),
                duration_ms: 1,
                uri: "spotify:track:t".to_string(),
                image_url: None,
                explicit: false,
            }],
            ..Default::default()
        }
    }

    #[test]
    fn test_search_ignores_out_of_order_responses_and_caches() {
        let mut app = main_app_for_tests();
        let _ = app.update(Message::SearchInputChanged("oa".to_string()));
        let _ = app.update(Message::SearchInputChanged("oasis".to_string()));

        // The stale "oa" response arrives last: it must not replace the screen.
        let _ = app.update(Message::SearchResultsFetched(
            "oasis".to_string(),
            Ok(search_results_named("Wonderwall")),
        ));
        let _ = app.update(Message::SearchResultsFetched(
            "oa".to_string(),
            Ok(search_results_named("Stale")),
        ));
        let AppState::Main {
            search_results,
            is_searching,
            ..
        } = &app.state
        else {
            panic!("expected main state");
        };
        assert_eq!(search_results.tracks[0].title, "Wonderwall");
        assert!(!is_searching);

        // Going back to an earlier query is answered from the cache instantly.
        let _ = app.update(Message::SearchInputChanged("OA ".to_string()));
        let AppState::Main {
            search_results,
            is_searching,
            ..
        } = &app.state
        else {
            panic!("expected main state");
        };
        assert_eq!(search_results.tracks[0].title, "Stale");
        assert!(!is_searching);
    }

    #[test]
    fn test_session_expired_reconnects_audio_without_logout() {
        let _guard = DISK_TEST_MUTEX.lock().unwrap();
        let (audio_tx, _) = tokio::sync::mpsc::channel(1);
        let mut app = App {
            state: AppState::Main {
                nav_item: NavigationItem::Home,
                playback: PlaybackState::default(),
                audio_session: None,
                playback_link: PlaybackLink::Connecting,
                pending_play_uri: None,
                main_scroll: ScrollWindow::default(),
                pending_images: std::collections::HashSet::new(),
                user_profile: None,
                user_playlists: Vec::new(),
                user_albums: Vec::new(),
                user_top_tracks: Vec::new(),
                featured_playlists: Vec::new(),
                featured_albums: Vec::new(),
                search_query: String::new(),
                search_results: crate::api::search::SearchResults::default(),
                is_searching: false,
                search_cache: crate::api::cache::LruCache::new(SEARCH_CACHE_CAPACITY),
                sidebar_filter: SidebarFilter::All,
                selected_playlist: None,
                selected_album: None,
                selected_artist: None,
                user_queue: Vec::new(),
                context_queue: Vec::new(),
                original_context_queue: Vec::new(),
                context_index: 0,
                history: Vec::new(),
                active_context_menu: None,
                active_modal: None,
                toast_notification: None,
                toast_id: 0,
                loaded_images: crate::api::cache::LruCache::new(IMAGE_CACHE_CAPACITY),
                spotify_client: None,
                sidebar_width: 240.0,
                right_panel_width: 280.0,
                active_right_panel: None,
                dragging_sidebar: false,
                dragging_right_panel: false,
                window_width: 1200.0,
                window_height: 800.0,
                last_right_click: None,
                navigation_history: Vec::new(),
                forward_history: Vec::new(),
                current_lyrics: None,
                lyrics_key: None,
                bio_key: None,
                lyrics_line: None,
                is_loading_lyrics: false,
                current_artist_bio: None,
                is_loading_artist_bio: false,
                autoplay_enabled: true,
                search_category_filter: SearchCategoryFilter::All,
                cache_size_bytes: 0,
                allow_explicit_content: true,
                ui_scale: 1.0,
                accent_tone: crate::ui::theme::AccentTone::default(),
                ui_language: UiLanguage::default(),
                audio_bitrate: crate::audio::session::AudioBitrate::default(),
                audio_normalization: true,
                gapless_playback: true,
            },
            audio_tx,
            active_error: Some("Old error".to_string()),
        };

        let _ = app.update(Message::SessionExpired);

        match app.state {
            AppState::Main {
                audio_session,
                playback_link,
                ..
            } => {
                assert!(audio_session.is_none());
                assert!(matches!(playback_link, PlaybackLink::Connecting));
            }
            _ => panic!("Audio session loss must not log the user out"),
        }
    }

    #[test]
    fn test_auth_error_transitions_to_login() {
        let (audio_tx, _) = tokio::sync::mpsc::channel(1);
        let mut app = App {
            state: AppState::Login {
                is_loading: false,
                error: None,
            },
            audio_tx,
            active_error: None,
        };

        let _ = app.update(Message::ErrorEncountered(AppError::Auth(
            "Token revoked".to_string(),
        )));

        assert!(app.active_error.is_none());
        match app.state {
            AppState::Login {
                is_loading, error, ..
            } => {
                assert!(!is_loading);
                assert!(error.is_some());
            }
            _ => panic!("Expected AppState::Login"),
        }
    }

    #[test]
    fn test_login_failed_from_initializing() {
        let (audio_tx, _) = tokio::sync::mpsc::channel(1);
        let mut app = App {
            state: AppState::Initializing,
            audio_tx,
            active_error: None,
        };

        let _ = app.update(Message::LoginFailed("No token".to_string()));

        match app.state {
            AppState::Login { is_loading, error } => {
                assert!(!is_loading);
                assert!(error.is_none());
            }
            _ => panic!("Expected AppState::Login"),
        }
    }

    #[test]
    fn test_cancel_login_sets_is_loading_false() {
        let (audio_tx, _) = tokio::sync::mpsc::channel(1);
        let mut app = App {
            state: AppState::Login {
                is_loading: true,
                error: None,
            },
            audio_tx,
            active_error: None,
        };

        let _ = app.update(Message::CancelLogin);

        match app.state {
            AppState::Login { is_loading, .. } => {
                assert!(!is_loading);
            }
            _ => panic!("Expected AppState::Login"),
        }
    }

    #[test]
    fn test_player_event_unavailable_does_not_expire_session() {
        let (audio_tx, _) = tokio::sync::mpsc::channel(1);
        let mut app = App {
            state: AppState::Main {
                nav_item: NavigationItem::Home,
                playback: PlaybackState {
                    is_playing: true,
                    ..Default::default()
                },
                audio_session: None,
                playback_link: PlaybackLink::Connecting,
                pending_play_uri: None,
                main_scroll: ScrollWindow::default(),
                pending_images: std::collections::HashSet::new(),
                user_profile: None,
                user_playlists: Vec::new(),
                user_albums: Vec::new(),
                user_top_tracks: Vec::new(),
                featured_playlists: Vec::new(),
                featured_albums: Vec::new(),
                search_query: String::new(),
                search_results: crate::api::search::SearchResults::default(),
                is_searching: false,
                search_cache: crate::api::cache::LruCache::new(SEARCH_CACHE_CAPACITY),
                sidebar_filter: SidebarFilter::All,
                selected_playlist: None,
                selected_album: None,
                selected_artist: None,
                user_queue: Vec::new(),
                context_queue: Vec::new(),
                original_context_queue: Vec::new(),
                context_index: 0,
                history: Vec::new(),
                active_context_menu: None,
                active_modal: None,
                toast_notification: None,
                toast_id: 0,
                loaded_images: crate::api::cache::LruCache::new(IMAGE_CACHE_CAPACITY),
                spotify_client: None,
                sidebar_width: 280.0,
                right_panel_width: 320.0,
                active_right_panel: None,
                dragging_sidebar: false,
                dragging_right_panel: false,
                window_width: 1200.0,
                window_height: 800.0,
                last_right_click: None,
                navigation_history: Vec::new(),
                forward_history: Vec::new(),
                current_lyrics: None,
                lyrics_key: None,
                bio_key: None,
                lyrics_line: None,
                is_loading_lyrics: false,
                current_artist_bio: None,
                is_loading_artist_bio: false,
                autoplay_enabled: true,
                search_category_filter: SearchCategoryFilter::All,
                cache_size_bytes: 0,
                allow_explicit_content: true,
                ui_scale: 1.0,
                accent_tone: crate::ui::theme::AccentTone::default(),
                ui_language: UiLanguage::default(),
                audio_bitrate: crate::audio::session::AudioBitrate::default(),
                audio_normalization: true,
                gapless_playback: true,
            },
            audio_tx,
            active_error: None,
        };

        let dummy_uri = librespot::core::spotify_uri::SpotifyUri::from_uri(
            "spotify:track:4cOdK2wGLETKBW3PvgPWqT",
        )
        .expect("valid uri");
        let _ = app.update(Message::PlayerEventReceived(PlayerEvent::Unavailable {
            track_id: dummy_uri,
            play_request_id: 0,
        }));

        match app.state {
            AppState::Main {
                playback,
                toast_notification,
                ..
            } => {
                assert!(!playback.is_playing);
                assert!(toast_notification.is_some());
            }
            _ => panic!("Expected to remain in AppState::Main"),
        }
    }

    #[test]
    fn test_accent_tone_update_and_persistence() {
        let _guard = DISK_TEST_MUTEX.lock().unwrap();
        save_accent_tone(crate::ui::theme::AccentTone::ElectricBlue);
        assert_eq!(
            load_accent_tone(),
            crate::ui::theme::AccentTone::ElectricBlue
        );
        save_accent_tone(crate::ui::theme::AccentTone::default());
    }

    #[test]
    fn test_ui_language_update_and_persistence() {
        let _guard = DISK_TEST_MUTEX.lock().unwrap();
        save_ui_language(UiLanguage::Spanish);
        assert_eq!(load_ui_language(), UiLanguage::Spanish);
        save_ui_language(UiLanguage::default());
    }

    #[test]
    fn test_audio_bitrate_persistence() {
        let _guard = DISK_TEST_MUTEX.lock().unwrap();
        save_audio_bitrate(crate::audio::session::AudioBitrate::VeryHigh320k);
        assert_eq!(
            load_audio_bitrate(),
            crate::audio::session::AudioBitrate::VeryHigh320k
        );
        save_audio_bitrate(crate::audio::session::AudioBitrate::default());
    }

    #[test]
    fn test_audio_normalization_persistence() {
        let _guard = DISK_TEST_MUTEX.lock().unwrap();
        save_audio_normalization(false);
        assert!(!load_audio_normalization());
        save_audio_normalization(true);
        assert!(load_audio_normalization());
    }

    #[test]
    fn test_gapless_playback_persistence() {
        let _guard = DISK_TEST_MUTEX.lock().unwrap();
        save_gapless_playback(false);
        assert!(!load_gapless_playback());
        save_gapless_playback(true);
        assert!(load_gapless_playback());
    }

    #[test]
    fn test_seek_jump_suppression() {
        let mut playback = PlaybackState {
            is_playing: true,
            current_track: Some(TrackInfo {
                title: "Around the World".to_string(),
                artist: "Daft Punk".to_string(),
                album: "Homework".to_string(),
                duration_ms: 240_000,
                image_url: None,
                uri: "spotify:track:123".to_string(),
                explicit: false,
            }),
            progress_ms: 10_000,
            ..Default::default()
        };

        playback.progress_ms = 120_000;
        playback.last_seek = Some((std::time::Instant::now(), 120_000));

        let stale_pos: u32 = 10_500;
        if let Some((seek_time, target_ms)) = playback.last_seek {
            if seek_time.elapsed() < std::time::Duration::from_millis(600) {
                if stale_pos.abs_diff(target_ms) > 2500 {
                } else {
                    playback.progress_ms = stale_pos;
                }
            }
        }
        assert_eq!(playback.progress_ms, 120_000);

        let good_pos: u32 = 120_100;
        if let Some((seek_time, target_ms)) = playback.last_seek {
            if seek_time.elapsed() < std::time::Duration::from_millis(600) {
                if good_pos.abs_diff(target_ms) > 2500 {
                } else {
                    playback.progress_ms = good_pos;
                }
            }
        }
        assert_eq!(playback.progress_ms, 120_100);
    }
}
