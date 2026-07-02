mod app;
mod error;
mod ui;

fn main() -> iced::Result {
    iced::application(app::App::new, app::App::update, app::App::view)
        .title("Spotifust")
        .run()
}
