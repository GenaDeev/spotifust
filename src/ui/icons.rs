/// SVG icon rendering module for Spotifust.
///
/// Provides a centralized way to load and display SVG icons throughout the UI.
/// All icons are loaded from `assets/icons/` at compile time via `include_bytes!`.
use iced::widget::Svg;
use iced::{Element, Length};

/// Available icons in the application.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    Home,
    Search,
    Library,
    Play,
    Pause,
    SkipNext,
    SkipPrev,
    Volume,
    VolumeMute,
    Queue,
    Devices,
    Heart,
    Shuffle,
    Repeat,
    Settings,
    User,
    MusicNote,
    Album,
    Clock,
    Pin,
    ChevronDown,
    ChevronRight,
    Plus,
    X,
}

impl Icon {
    /// Returns the raw SVG bytes for this icon.
    fn svg_bytes(self) -> &'static [u8] {
        match self {
            Self::Home => include_bytes!("../../assets/icons/home.svg"),
            Self::Search => include_bytes!("../../assets/icons/search.svg"),
            Self::Library => include_bytes!("../../assets/icons/library.svg"),
            Self::Play => include_bytes!("../../assets/icons/play.svg"),
            Self::Pause => include_bytes!("../../assets/icons/pause.svg"),
            Self::SkipNext => include_bytes!("../../assets/icons/skip_next.svg"),
            Self::SkipPrev => include_bytes!("../../assets/icons/skip_prev.svg"),
            Self::Volume => include_bytes!("../../assets/icons/volume.svg"),
            Self::VolumeMute => include_bytes!("../../assets/icons/volume_mute.svg"),
            Self::Queue => include_bytes!("../../assets/icons/queue.svg"),
            Self::Devices => include_bytes!("../../assets/icons/devices.svg"),
            Self::Heart => include_bytes!("../../assets/icons/heart.svg"),
            Self::Shuffle => include_bytes!("../../assets/icons/shuffle.svg"),
            Self::Repeat => include_bytes!("../../assets/icons/repeat.svg"),
            Self::Settings => include_bytes!("../../assets/icons/settings.svg"),
            Self::User => include_bytes!("../../assets/icons/user.svg"),
            Self::MusicNote => include_bytes!("../../assets/icons/music_note.svg"),
            Self::Album => include_bytes!("../../assets/icons/album.svg"),
            Self::Clock => include_bytes!("../../assets/icons/clock.svg"),
            Self::Pin => include_bytes!("../../assets/icons/pin.svg"),
            Self::ChevronDown => include_bytes!("../../assets/icons/chevron_down.svg"),
            Self::ChevronRight => include_bytes!("../../assets/icons/chevron_right.svg"),
            Self::Plus => include_bytes!("../../assets/icons/plus.svg"),
            Self::X => include_bytes!("../../assets/icons/x.svg"),
        }
    }

    /// Creates an iced SVG handle for this icon.
    fn handle(self) -> iced::widget::svg::Handle {
        iced::widget::svg::Handle::from_memory(self.svg_bytes())
    }

    /// Renders this icon as an `Svg` widget with the given size.
    pub fn view<M: 'static>(self, size: f32) -> Element<'static, M> {
        Svg::new(self.handle())
            .width(Length::Fixed(size))
            .height(Length::Fixed(size))
            .into()
    }

    /// Renders this icon as a raw `Svg` widget (for further customization).
    #[allow(dead_code)]
    pub fn svg(self, size: f32) -> Svg<'static> {
        Svg::new(self.handle())
            .width(Length::Fixed(size))
            .height(Length::Fixed(size))
    }
}
