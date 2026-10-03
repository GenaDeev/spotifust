// pub mod canvas_view;
pub mod context_menu;
pub mod icons;
pub mod login;
pub mod main_layout;
pub mod systray;
pub mod theme;

const LOGO_BYTES: &[u8] = include_bytes!("../../assets/spotifust.png");

/// Pre-scaled application logo.
///
/// `image::Handle::from_bytes` mints a fresh id on every call, so building it inside
/// `view()` made the renderer re-decode the 1080px PNG on every single frame. The
/// handle is created once, already downscaled for the sizes the UI draws.
#[must_use]
pub fn logo_handle() -> iced::widget::image::Handle {
    static LOGO: std::sync::OnceLock<iced::widget::image::Handle> = std::sync::OnceLock::new();
    LOGO.get_or_init(|| match image::load_from_memory(LOGO_BYTES) {
        Ok(img) => {
            let small = img
                .resize(168, 168, image::imageops::FilterType::Lanczos3)
                .to_rgba8();
            iced::widget::image::Handle::from_rgba(small.width(), small.height(), small.into_raw())
        }
        Err(_) => iced::widget::image::Handle::from_bytes(LOGO_BYTES),
    })
    .clone()
}
