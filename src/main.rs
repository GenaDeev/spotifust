pub mod api;
mod app;
mod audio;
mod error;
mod ui;

fn load_icon() -> Option<iced::window::Icon> {
    let bytes = include_bytes!("../assets/spotifust.png");
    let img = image::load_from_memory(bytes).ok()?;
    let rgba = img.into_rgba8();
    let (width, height) = rgba.dimensions();
    iced::window::icon::from_rgba(rgba.into_raw(), width, height).ok()
}

fn main() -> iced::Result {
    // Intercept custom protocol launch
    if let Some(arg) = std::env::args().nth(1) {
        if arg.starts_with("spotifust://callback") {
            let temp_dir = std::env::temp_dir();
            let auth_file = temp_dir.join("spotifust_auth.txt");
            if let Err(e) = std::fs::write(&auth_file, arg) {
                eprintln!("Failed to pass token to main instance: {e}");
                std::process::exit(1);
            }
            println!("Authentication passed! You can close this window.");
            std::process::exit(0);
        }
    }

    iced::application(app::App::new, app::App::update, app::App::view)
        .title("Spotifust")
        .window(iced::window::Settings {
            icon: load_icon(),
            ..Default::default()
        })
        .subscription(app::App::subscription)
        .run()
}
