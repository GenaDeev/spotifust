//! Playback credentials for the librespot session.
//!
//! Spotify only lets login5 mint streaming tokens for sessions whose stored
//! credentials were issued to the official desktop (keymaster) client ID. A Web API
//! token from our own client ID connects to the access point but every track then
//! reports `Unavailable`. We therefore pair once through the OAuth device
//! authorization flow (no redirect URI, no local port), let the access point turn
//! that token into reusable credentials, and keep those in the OS keychain.

use crate::error::AppError;
use librespot::core::authentication::Credentials;
use librespot::core::config::SessionConfig;
use librespot::core::session::Session;
use librespot::oauth::{DeviceAuthClient, DeviceAuthClientBuilder, DeviceAuthorization};
use librespot::protocol::authentication::AuthenticationType;

const KEYRING_USER: &str = "librespot_credentials";

/// Scopes for the playback-only token. `streaming` is all the access point needs.
const PLAYBACK_SCOPES: [&str; 1] = ["streaming"];

/// A pending device pairing the user still has to approve in a browser.
#[derive(Debug, Clone)]
pub struct PairingRequest {
    pub user_code: String,
    pub url: String,
    authorization: DeviceAuthorization,
}

fn keyring_entry() -> Result<keyring::Entry, AppError> {
    keyring::Entry::new(crate::api::auth::keyring_service(), KEYRING_USER)
        .map_err(|e| AppError::Auth(format!("Keyring error: {e}")))
}

fn device_client() -> Result<DeviceAuthClient, AppError> {
    let client_id = SessionConfig::default().client_id;
    DeviceAuthClientBuilder::new(&client_id, PLAYBACK_SCOPES.to_vec())
        .build()
        .map_err(|e| AppError::Playback(format!("Failed to build pairing client: {e}")))
}

/// Loads reusable librespot credentials from the OS keychain, if any.
#[must_use]
pub fn load_stored_credentials() -> Option<Credentials> {
    let json = keyring_entry().ok()?.get_password().ok()?;
    serde_json::from_str(&json).ok()
}

/// Persists the session's reusable credentials in the OS keychain.
#[allow(clippy::missing_errors_doc)]
pub fn store_session_credentials(session: &Session) -> Result<(), AppError> {
    let auth_data = session.auth_data();
    if auth_data.is_empty() {
        return Err(AppError::Playback(
            "Spotify did not return reusable credentials".to_string(),
        ));
    }
    let credentials = Credentials {
        username: Some(session.username()),
        auth_type: AuthenticationType::AUTHENTICATION_STORED_SPOTIFY_CREDENTIALS,
        auth_data,
    };
    let json = serde_json::to_string(&credentials)
        .map_err(|e| AppError::Playback(format!("Failed to encode credentials: {e}")))?;
    keyring_entry()?
        .set_password(&json)
        .map_err(|e| AppError::Auth(format!("Failed to save credentials to keyring: {e}")))
}

/// Removes stored playback credentials (e.g. after Spotify revoked them).
pub fn delete_stored_credentials() {
    if let Ok(entry) = keyring_entry() {
        let _ = entry.delete_credential();
    }
}

/// Starts a device pairing and returns the code the user must approve.
#[allow(clippy::missing_errors_doc)]
pub async fn request_pairing() -> Result<PairingRequest, AppError> {
    let client = device_client()?;
    let authorization = client
        .request_device_code_async()
        .await
        .map_err(|e| AppError::Playback(format!("Failed to start pairing: {e}")))?;
    Ok(PairingRequest {
        user_code: authorization.user_code().to_string(),
        url: authorization.url().to_string(),
        authorization,
    })
}

/// Waits until the user approves `request`, returning access-token credentials.
#[allow(clippy::missing_errors_doc)]
pub async fn await_pairing(request: PairingRequest) -> Result<Credentials, AppError> {
    let client = device_client()?;
    let token = client
        .poll_for_token_async(&request.authorization)
        .await
        .map_err(|e| AppError::Playback(format!("Pairing was not completed: {e}")))?;
    Ok(Credentials::with_access_token(token.access_token))
}
