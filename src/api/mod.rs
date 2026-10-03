pub mod album;
pub mod artist;
pub mod auth;
pub mod cache;
pub mod local_files;
pub mod lyrics;
pub mod playlist;
pub mod search;
pub mod tracks;
pub mod updater;
pub mod user;

/// Picks the smallest Spotify image that is still at least 300px wide.
///
/// Spotify lists images largest-first (usually 640/300/64); the UI never draws
/// covers bigger than ~240px, so the 640px variant only wastes bandwidth and decode time.
#[must_use]
pub fn best_image_url(images: &[rspotify::model::Image]) -> Option<String> {
    images
        .iter()
        .filter(|img| img.width.is_some_and(|w| w >= 300))
        .min_by_key(|img| img.width)
        .or_else(|| images.first())
        .map(|img| img.url.clone())
}
