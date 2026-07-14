// Suppress console window on Windows — Spotifust is a GUI application.
#![windows_subsystem = "windows"]

pub mod api;
mod app;
mod audio;
mod error;
mod ui;

fn load_icon() -> Option<iced::window::Icon> {
    let bytes = include_bytes!("../assets/spotifust_icon.png");
    let img = image::load_from_memory(bytes).ok()?;
    let rgba = img.into_rgba8();
    let (width, height) = rgba.dimensions();
    iced::window::icon::from_rgba(rgba.into_raw(), width, height).ok()
}

#[cfg(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "openbsd",
    target_os = "netbsd"
))]
fn get_platform_settings() -> iced::window::settings::PlatformSpecific {
    iced::window::settings::PlatformSpecific {
        application_id: String::from("spotifust"),
        ..Default::default()
    }
}

#[cfg(not(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "openbsd",
    target_os = "netbsd"
)))]
fn get_platform_settings() -> iced::window::settings::PlatformSpecific {
    iced::window::settings::PlatformSpecific::default()
}

fn main() -> iced::Result {
    // Intercept custom protocol launch (OAuth callback via spotifust:// scheme).
    // This instance writes the callback URL to a temp file for the main instance
    // to pick up, then exits silently — no console window is shown.
    if let Some(arg) = std::env::args().nth(1) {
        if arg.starts_with("spotifust://callback") {
            let temp_dir = std::env::temp_dir();
            let auth_file = temp_dir.join("spotifust_auth.txt");
            if let Err(e) = std::fs::write(&auth_file, arg) {
                eprintln!("Failed to pass token to main instance: {e}");
                std::process::exit(1);
            }
            std::process::exit(0);
        }
    }

    iced::application(app::App::new, app::App::update, app::App::view)
        .title("Spotifust")
        .window(iced::window::Settings {
            icon: load_icon(),
            platform_specific: get_platform_settings(),
            ..Default::default()
        })
        .subscription(app::App::subscription)
        .run()
}

