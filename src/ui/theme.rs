//! Branding constants for Spotifust's visual identity.
//!
//! Replicating the modern 2025-2026 Spotify UI palette.

/// Brand accent color — Rust Orange
pub const ACCENT: iced::Color = iced::Color {
    r: 0.957,
    g: 0.647,
    b: 0.510,
    a: 1.0,
};

/// Brand accent color hover — Deeper Rust Orange
pub const ACCENT_HOVER: iced::Color = iced::Color {
    r: 0.957,
    g: 0.510,
    b: 0.392,
    a: 1.0,
};

// --- Surface colors (dark theme) ---

/// Main Content background (`#121212`)
pub const BG_BASE: iced::Color = iced::Color {
    r: 0.07,
    g: 0.07,
    b: 0.07,
    a: 1.0,
};

/// Shell background (Top Bar, Sidebars, Player Bar) (`#000000`)
pub const SURFACE_0: iced::Color = iced::Color {
    r: 0.0,
    g: 0.0,
    b: 0.0,
    a: 1.0,
};

/// Cards normal / Elevated surfaces (`#181818`)
pub const SURFACE_1: iced::Color = iced::Color {
    r: 0.094,
    g: 0.094,
    b: 0.094,
    a: 1.0,
};

/// Cards hover / Popups (`#282828`)
pub const SURFACE_2: iced::Color = iced::Color {
    r: 0.157,
    g: 0.157,
    b: 0.157,
    a: 1.0,
};

// --- Text colors ---

/// Primary text — White (`#FFFFFF`)
pub const TEXT_PRIMARY: iced::Color = iced::Color {
    r: 1.0,
    g: 1.0,
    b: 1.0,
    a: 1.0,
};

/// Secondary text — Muted Grey (`#B3B3B3`)
pub const TEXT_SECONDARY: iced::Color = iced::Color {
    r: 0.702,
    g: 0.702,
    b: 0.702,
    a: 1.0,
};

/// Tertiary text / Disabled (`#6A6A6A`)
#[allow(dead_code)]
pub const TEXT_TERTIARY: iced::Color = iced::Color {
    r: 0.416,
    g: 0.416,
    b: 0.416,
    a: 1.0,
};

// --- Border / Separator ---

/// Subtle border color for containers (`#2A2A2A`)
#[allow(dead_code)]
pub const BORDER_SUBTLE: iced::Color = iced::Color {
    r: 0.165,
    g: 0.165,
    b: 0.165,
    a: 1.0,
};
