//! Branding constants and Design System for Spotifust.
//!
//! Visual identity centered around Spotifust's signature Rust Orange palette.

use iced::Color;

// --- Brand Accent Colors (Spotifust Rust Orange) ---

/// Core Brand Accent — Rust Orange (`#F4A261`)
pub const ACCENT: Color = Color {
    r: 0.957,
    g: 0.635,
    b: 0.380,
    a: 1.0,
};

/// Brand Accent Hover — Bright Rust Orange (`#F58A4B`)
pub const ACCENT_HOVER: Color = Color {
    r: 0.961,
    g: 0.541,
    b: 0.294,
    a: 1.0,
};

/// Brand Accent Pressed — Deep Rust Orange (`#D96B27`)
pub const ACCENT_PRESSED: Color = Color {
    r: 0.851,
    g: 0.420,
    b: 0.153,
    a: 1.0,
};

// Aliases for compatibility
#[allow(dead_code)]
pub const GREEN: Color = ACCENT;
#[allow(dead_code)]
pub const GREEN_HOVER: Color = ACCENT_HOVER;
#[allow(dead_code)]
pub const GREEN_PRESSED: Color = ACCENT_PRESSED;

// --- Surface & Background Elevation (Dark Mode) ---

/// Main Shell / Window Background (`#000000`)
pub const BG_BASE: Color = Color {
    r: 0.0,
    g: 0.0,
    b: 0.0,
    a: 1.0,
};

/// Main View Area Background (`#121212`)
pub const SURFACE_MAIN: Color = Color {
    r: 0.071,
    g: 0.071,
    b: 0.071,
    a: 1.0,
};

/// Card / Container Normal (`#181818`)
pub const SURFACE_CARD: Color = Color {
    r: 0.094,
    g: 0.094,
    b: 0.094,
    a: 1.0,
};

/// Card / Button Hover (`#282828`)
pub const SURFACE_HOVER: Color = Color {
    r: 0.157,
    g: 0.157,
    b: 0.157,
    a: 1.0,
};

/// Active / Selected Surface (`#2A2A2A`)
pub const SURFACE_ACTIVE: Color = Color {
    r: 0.165,
    g: 0.165,
    b: 0.165,
    a: 1.0,
};

/// Popups & Floating Panels (`#1F1F1F`)
#[allow(dead_code)]
pub const SURFACE_ELEVATED: Color = Color {
    r: 0.122,
    g: 0.122,
    b: 0.122,
    a: 1.0,
};

// --- Text Colors ---

/// Primary Text — Crisp White (`#FFFFFF`)
pub const TEXT_PRIMARY: Color = Color {
    r: 1.0,
    g: 1.0,
    b: 1.0,
    a: 1.0,
};

/// Secondary Text — Muted Silver (`#B3B3B3`)
pub const TEXT_SECONDARY: Color = Color {
    r: 0.702,
    g: 0.702,
    b: 0.702,
    a: 1.0,
};

/// Tertiary / Disabled Text — Charcoal (`#6A6A6A`)
#[allow(dead_code)]
pub const TEXT_TERTIARY: Color = Color {
    r: 0.416,
    g: 0.416,
    b: 0.416,
    a: 1.0,
};

#[allow(dead_code)]
pub const TEXT_MUTED: Color = TEXT_TERTIARY;

// --- Border & Dividers ---

/// Subtle border for cards & panels
pub const BORDER_SUBTLE: Color = Color {
    r: 1.0,
    g: 1.0,
    b: 1.0,
    a: 0.08,
};

/// Strong border for focused / highlighted elements
#[allow(dead_code)]
pub const BORDER_STRONG: Color = Color {
    r: 1.0,
    g: 1.0,
    b: 1.0,
    a: 0.18,
};

// --- Legacy Surface Aliases ---
#[allow(dead_code)]
pub const SURFACE_0: Color = BG_BASE;
#[allow(dead_code)]
pub const SURFACE_1: Color = SURFACE_CARD;
#[allow(dead_code)]
pub const SURFACE_2: Color = SURFACE_HOVER;

// --- Corner Radius Tokens ---

pub const RADIUS_SM: f32 = 4.0;
pub const RADIUS_MD: f32 = 8.0;
pub const RADIUS_LG: f32 = 12.0;
pub const RADIUS_XL: f32 = 16.0;
pub const RADIUS_PILL: f32 = 9999.0;

// --- Spacing Scale Tokens ---

#[allow(dead_code)]
pub const SPACING_XS: u16 = 4;
#[allow(dead_code)]
pub const SPACING_SM: u16 = 8;
#[allow(dead_code)]
pub const SPACING_MD: u16 = 12;
#[allow(dead_code)]
pub const SPACING_LG: u16 = 16;
#[allow(dead_code)]
pub const SPACING_XL: u16 = 24;

// --- Typography Scale Tokens ---

#[allow(dead_code)]
pub const FONT_SIZE_XS: u16 = 10;
#[allow(dead_code)]
pub const FONT_SIZE_SM: u16 = 12;
#[allow(dead_code)]
pub const FONT_SIZE_MD: u16 = 14;
#[allow(dead_code)]
pub const FONT_SIZE_LG: u16 = 18;
#[allow(dead_code)]
pub const FONT_SIZE_XL: u16 = 22;
#[allow(dead_code)]
pub const FONT_SIZE_HEADER: u16 = 30;

// --- Status & Feedback Colors ---

#[allow(dead_code)]
pub const COLOR_ERROR: Color = Color {
    r: 0.95,
    g: 0.35,
    b: 0.35,
    a: 1.0,
};

#[allow(dead_code)]
pub const COLOR_SUCCESS: Color = Color {
    r: 0.35,
    g: 0.85,
    b: 0.45,
    a: 1.0,
};
