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
    iced::application(app::App::new, app::App::update, app::App::view)
        .title("Spotifust")
        .window(iced::window::Settings {
            icon: load_icon(),
            ..Default::default()
        })
        .run()
}
