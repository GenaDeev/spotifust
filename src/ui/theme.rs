//! Branding constants for Spotifust's visual identity.
//!
//! The accent color is the warm salmon/peach orange from the Spotifust logo,
//! NOT Spotify green. Spotify green should only appear where it explicitly
//! represents the Spotify brand (e.g., the "Login with Spotify" button).
/// Brand accent color — warm salmon/peach orange.
/// Derived from the Spotifust logo: `#F4A582`.
pub const ACCENT: iced::Color = iced::Color {
    r: 0.957,
    g: 0.647,
    b: 0.510,
    a: 1.0,
};

/// Deeper/saturated variant of the accent — `#F48264`.
#[allow(dead_code)]
pub const ACCENT_DEEP: iced::Color = iced::Color {
    r: 0.957,
    g: 0.510,
    b: 0.392,
    a: 1.0,
};

/// Accent at reduced opacity for subtle highlights.
#[allow(dead_code)]
pub const ACCENT_DIM: iced::Color = iced::Color {
    r: 0.957,
    g: 0.647,
    b: 0.510,
    a: 0.6,
};

// --- Surface colors (dark theme) ---

/// Deepest background — near black. `#090909`
pub const BG_BASE: iced::Color = iced::Color {
    r: 0.035,
    g: 0.035,
    b: 0.035,
    a: 1.0,
};

/// Primary surface — very dark grey. `#0F0F0F`
pub const SURFACE_0: iced::Color = iced::Color {
    r: 0.059,
    g: 0.059,
    b: 0.059,
    a: 1.0,
};

/// Elevated surface — cards, sidebar. `#161616`
pub const SURFACE_1: iced::Color = iced::Color {
    r: 0.086,
    g: 0.086,
    b: 0.086,
    a: 1.0,
};

/// Higher elevation surface — hover states, popups. `#1E1E1E`
pub const SURFACE_2: iced::Color = iced::Color {
    r: 0.118,
    g: 0.118,
    b: 0.118,
    a: 1.0,
};

/// Highest elevation surface — active states. `#282828`
#[allow(dead_code)]
pub const SURFACE_3: iced::Color = iced::Color {
    r: 0.157,
    g: 0.157,
    b: 0.157,
    a: 1.0,
};

// --- Text colors ---

/// Primary text — bright white.
pub const TEXT_PRIMARY: iced::Color = iced::Color {
    r: 0.937,
    g: 0.937,
    b: 0.937,
    a: 1.0,
};

/// Secondary text — muted grey. `#B3B3B3`
pub const TEXT_SECONDARY: iced::Color = iced::Color {
    r: 0.702,
    g: 0.702,
    b: 0.702,
    a: 1.0,
};

/// Tertiary text — very muted. `#727272`
#[allow(dead_code)]
pub const TEXT_TERTIARY: iced::Color = iced::Color {
    r: 0.447,
    g: 0.447,
    b: 0.447,
    a: 1.0,
};

// --- Border / Separator ---

/// Subtle border color for containers.
pub const BORDER_SUBTLE: iced::Color = iced::Color {
    r: 1.0,
    g: 1.0,
    b: 1.0,
    a: 0.06,
};

// --- Spotify brand color (ONLY for Spotify-branded elements) ---

/// Spotify green — use ONLY for elements that represent the Spotify brand,
/// such as the "Login with Spotify" button. NOT for general UI accent.
#[allow(dead_code)]
pub const SPOTIFY_GREEN: iced::Color = iced::Color {
    r: 0.114,
    g: 0.725,
    b: 0.329,
    a: 1.0,
};

// --- Semantic colors ---

/// Error/danger color — warm red. `#FF5555`
#[allow(dead_code)]
pub const ERROR: iced::Color = iced::Color {
    r: 1.0,
    g: 0.333,
    b: 0.333,
    a: 1.0,
};
