use thiserror::Error;

#[allow(dead_code)]
#[derive(Error, Debug, Clone)]
pub enum AppError {
    #[error("Authentication error: {0}")]
    Auth(String),

    #[error("Playback error: {0}")]
    Playback(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Cache error: {0}")]
    Cache(String),
}
