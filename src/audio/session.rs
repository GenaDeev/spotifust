use librespot::core::authentication::Credentials;
use librespot::core::config::SessionConfig;
use librespot::core::session::Session;
use crate::error::AppError;

#[allow(dead_code)]
pub async fn connect(username: &str, password: &str) -> Result<Session, AppError> {
    let credentials = Credentials::with_password(username, password);
    let session_config = SessionConfig::default();

    let session = Session::new(session_config, None);
    session.connect(credentials, false)
        .await
        .map_err(|e| AppError::Auth(format!("Librespot login failed: {e}")))?;

    Ok(session)
}
