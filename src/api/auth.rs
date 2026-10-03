use crate::error::AppError;
use rspotify::{
    AuthCodePkceSpotify, Credentials, OAuth,
    clients::{BaseClient, OAuthClient},
    scopes,
};

const CLIENT_ID: &str = "6b2bd6e25f5e49e1853788e7b705522f"; // Needs to be a valid client id or from env
const REDIRECT_URI: &str = "spotifust://callback";

fn get_spotify_client() -> AuthCodePkceSpotify {
    let client_id = std::env::var("SPOTIFY_CLIENT_ID").unwrap_or_else(|_| CLIENT_ID.to_string());

    let creds = Credentials::new_pkce(&client_id);
    let oauth = OAuth {
        redirect_uri: REDIRECT_URI.to_string(),
        scopes: scopes!(
            "user-read-playback-state",
            "user-modify-playback-state",
            "user-read-currently-playing",
            "streaming",
            "app-remote-control",
            "playlist-read-private",
            "playlist-read-collaborative",
            "user-library-read",
            "user-top-read",
            "playlist-modify-public",
            "playlist-modify-private",
            "user-library-modify",
            "user-follow-read",
            "user-follow-modify"
        ),
        ..Default::default()
    };

    AuthCodePkceSpotify::new(creds, oauth)
}

/// Keychain service all Spotifust secrets live under.
///
/// Unit tests get their own service so flows like "session expired" can't wipe the
/// developer's real login. Live (`#[ignore]`d) tests that need the real login opt
/// back in with `SPOTIFUST_LIVE_KEYRING=1`.
#[must_use]
pub fn keyring_service() -> &'static str {
    if cfg!(test) && std::env::var_os("SPOTIFUST_LIVE_KEYRING").is_none() {
        "spotifust-test"
    } else {
        "spotifust"
    }
}

/// Saves the refresh token to the OS keychain via `keyring`.
#[allow(clippy::missing_errors_doc)]
pub fn save_refresh_token_to_keyring(refresh_token: &str) -> Result<(), AppError> {
    let entry = keyring::Entry::new(keyring_service(), "spotify_refresh_token")
        .map_err(|e| AppError::Auth(format!("Keyring error: {e}")))?;
    entry
        .set_password(refresh_token)
        .map_err(|e| AppError::Auth(format!("Failed to save token to keyring: {e}")))?;
    Ok(())
}

/// Retrieves the refresh token from the OS keychain via `keyring`.
#[allow(clippy::missing_errors_doc)]
pub fn get_refresh_token_from_keyring() -> Result<String, AppError> {
    let entry = keyring::Entry::new(keyring_service(), "spotify_refresh_token")
        .map_err(|e| AppError::Auth(format!("Keyring error: {e}")))?;
    entry
        .get_password()
        .map_err(|_| AppError::Auth("No token in keyring".to_string()))
}

/// Deletes the refresh token from the OS keychain via `keyring`.
#[allow(clippy::missing_errors_doc)]
pub fn delete_refresh_token_from_keyring() -> Result<(), AppError> {
    let entry = keyring::Entry::new(keyring_service(), "spotify_refresh_token")
        .map_err(|e| AppError::Auth(format!("Keyring error: {e}")))?;
    let _ = entry.delete_credential();
    delete_session_token();
    Ok(())
}

const TOKEN_KEYRING_USER: &str = "spotify_session_token";

/// Minimum lifetime left on a cached access token for it to be reused at startup.
const TOKEN_REUSE_MARGIN_SECS: i64 = 300;

/// Stores the whole OAuth token (access + refresh) in the OS keychain so the next
/// launch can skip the refresh round trip while the access token is still valid.
pub fn save_session_token(token: &rspotify::Token) {
    if let Some(refresh_token) = &token.refresh_token {
        let _ = save_refresh_token_to_keyring(refresh_token);
    }
    if let (Ok(entry), Ok(json)) = (
        keyring::Entry::new(keyring_service(), TOKEN_KEYRING_USER),
        serde_json::to_string(token),
    ) {
        let _ = entry.set_password(&json);
    }
}

fn load_session_token() -> Option<rspotify::Token> {
    let json = keyring::Entry::new(keyring_service(), TOKEN_KEYRING_USER)
        .ok()?
        .get_password()
        .ok()?;
    serde_json::from_str(&json).ok()
}

fn delete_session_token() {
    if let Ok(entry) = keyring::Entry::new(keyring_service(), TOKEN_KEYRING_USER) {
        let _ = entry.delete_credential();
    }
}

/// True when `token` stays valid for at least [`TOKEN_REUSE_MARGIN_SECS`].
fn token_is_fresh(token: &rspotify::Token) -> bool {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX));
    !token.is_expired()
        && token
            .expires_at
            .is_some_and(|at| at.timestamp() > now + TOKEN_REUSE_MARGIN_SECS)
}

async fn snapshot_token(spotify: &AuthCodePkceSpotify) -> Option<rspotify::Token> {
    spotify.get_token().lock().await.ok()?.clone()
}

#[allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]
pub async fn do_login_flow() -> Result<AuthCodePkceSpotify, AppError> {
    let mut spotify = get_spotify_client();
    let url = spotify
        .get_authorize_url(None)
        .map_err(|e| AppError::Auth(format!("Failed to generate auth url: {e}")))?;

    open::that(&url).map_err(|e| AppError::Auth(format!("Failed to open browser: {e}")))?;

    // Wait for the temp file to be created by the interceptor instance
    let temp_dir = std::env::temp_dir();
    let auth_file = temp_dir.join("spotifust_auth.txt");

    // Ensure it doesn't exist from a previous run
    let _ = std::fs::remove_file(&auth_file);

    let mut attempts = 0;
    let url_with_code = loop {
        if attempts > 60 {
            // 2 minutes timeout (assuming 2s intervals)
            return Err(AppError::Auth("Timeout waiting for login".into()));
        }
        if let Ok(content) = std::fs::read_to_string(&auth_file) {
            let _ = std::fs::remove_file(&auth_file);
            break content;
        }
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        attempts += 1;
    };

    let code = spotify
        .parse_response_code(&url_with_code)
        .ok_or_else(|| AppError::Auth("Could not parse auth code from URL".to_string()))?;

    spotify
        .request_token(&code)
        .await
        .map_err(|e| AppError::Auth(format!("Failed to request token: {e}")))?;

    let token_mutex = spotify.get_token();
    let token_guard = token_mutex
        .lock()
        .await
        .map_err(|e| AppError::Auth(format!("Failed to lock token mutex: {e:?}")))?;
    let token = token_guard
        .clone()
        .ok_or_else(|| AppError::Auth("No token obtained".to_string()))?;

    drop(token_guard);
    if token.refresh_token.is_none() {
        return Err(AppError::Auth("Spotify returned no refresh token".to_string()));
    }
    save_session_token(&token);

    Ok(spotify)
}

#[allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]
pub async fn check_existing_login() -> Result<AuthCodePkceSpotify, AppError> {
    let refresh_token = get_refresh_token_from_keyring()?;

    let spotify = get_spotify_client();

    // Fast path: reuse the cached access token while it's still valid, saving a
    // full round trip to accounts.spotify.com before the UI can load anything.
    if let Some(cached) = load_session_token()
        .filter(token_is_fresh)
        .filter(|t| t.refresh_token.as_deref() == Some(refresh_token.as_str()))
    {
        *spotify
            .get_token()
            .lock()
            .await
            .map_err(|e| AppError::Auth(format!("Failed to lock token mutex: {e:?}")))? =
            Some(cached);
        return Ok(spotify);
    }

    // We construct a mock token just with the refresh token so rspotify can refresh it
    let token = rspotify::model::Token {
        refresh_token: Some(refresh_token),
        ..Default::default()
    };

    let token_mutex = spotify.get_token();
    *token_mutex
        .lock()
        .await
        .map_err(|e| AppError::Auth(format!("Failed to lock token mutex: {e:?}")))? = Some(token);

    // Force a refresh to verify it works and get an access token
    spotify
        .refresh_token()
        .await
        .map_err(|e| classify_refresh_error(&e))?;
    if let Some(token) = snapshot_token(&spotify).await {
        save_session_token(&token);
    }

    Ok(spotify)
}

/// Refreshes the Spotify client's OAuth token silently if expired or missing.
#[allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]
pub async fn refresh_token_if_expired(spotify: &AuthCodePkceSpotify) -> Result<(), AppError> {
    let token_mutex = spotify.get_token();
    let token_guard = token_mutex
        .lock()
        .await
        .map_err(|e| AppError::Auth(format!("Failed to lock token mutex: {e:?}")))?;

    let is_expired = if let Some(token) = token_guard.as_ref() {
        token.is_expired()
    } else {
        true
    };

    drop(token_guard);

    if is_expired {
        // If token has refresh_token, refresh directly. Otherwise load from keyring.
        let has_refresh = {
            let mutex = spotify.get_token();
            let guard = mutex
                .lock()
                .await
                .map_err(|e| AppError::Auth(format!("Lock error: {e:?}")))?;
            guard.as_ref().and_then(|t| t.refresh_token.clone())
        };

        if has_refresh.is_none() {
            let refresh_token = get_refresh_token_from_keyring()?;
            let mock_token = rspotify::model::Token {
                refresh_token: Some(refresh_token),
                ..Default::default()
            };
            let mutex = spotify.get_token();
            *mutex
                .lock()
                .await
                .map_err(|e| AppError::Auth(format!("Lock error: {e:?}")))? = Some(mock_token);
        }

        spotify
            .refresh_token()
            .await
            .map_err(|e| classify_refresh_error(&e))?;
        if let Some(token) = snapshot_token(spotify).await {
            save_session_token(&token);
        }
    }

    Ok(())
}

/// Only a rejected refresh token (HTTP 400/401) means the login is gone; anything
/// else (offline, DNS, 5xx) is transient and must not log the user out.
fn classify_refresh_error(e: &rspotify::ClientError) -> AppError {
    if let rspotify::ClientError::Http(http_err) = e {
        if let rspotify::http::HttpError::StatusCode(resp) = http_err.as_ref() {
            if matches!(
                resp.status(),
                reqwest::StatusCode::BAD_REQUEST | reqwest::StatusCode::UNAUTHORIZED
            ) {
                return AppError::Auth(format!("Failed to refresh token: {e}"));
            }
        }
    }
    AppError::Network(format!("Failed to refresh token: {e}"))
}

#[must_use]
pub fn map_rspotify_error(e: rspotify::ClientError) -> AppError {
    match e {
        rspotify::ClientError::Http(http_err) => match *http_err {
            rspotify::http::HttpError::StatusCode(ref resp)
                if resp.status() == reqwest::StatusCode::TOO_MANY_REQUESTS =>
            {
                let secs = resp
                    .headers()
                    .get(reqwest::header::RETRY_AFTER)
                    .and_then(|h| h.to_str().ok())
                    .and_then(|s| s.parse::<u64>().ok())
                    .unwrap_or(2);
                AppError::RateLimited(secs)
            }
            rspotify::http::HttpError::StatusCode(ref resp)
                if resp.status() == reqwest::StatusCode::UNAUTHORIZED =>
            {
                AppError::Auth("Unauthorized 401".to_string())
            }
            other => AppError::Network(other.to_string()),
        },
        rspotify::ClientError::InvalidToken => AppError::Auth("Invalid token".to_string()),
        other => {
            let err_str = other.to_string();
            if err_str.contains("429") {
                AppError::RateLimited(2)
            } else {
                AppError::Network(err_str)
            }
        }
    }
}

#[allow(clippy::missing_errors_doc)]
pub async fn with_auto_reauth<F, Fut, T>(spotify: &AuthCodePkceSpotify, f: F) -> Result<T, AppError>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T, AppError>>,
{
    let mut attempts = 0;
    loop {
        match f().await {
            Ok(val) => return Ok(val),
            Err(AppError::RateLimited(secs)) if attempts < 3 => {
                attempts += 1;
                tokio::time::sleep(std::time::Duration::from_secs(secs)).await;
            }
            Err(AppError::Auth(_)) if attempts < 1 => {
                attempts += 1;
                refresh_token_if_expired(spotify).await?;
            }
            Err(e) => return Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keyring_service_and_account_name() {
        let res = keyring::Entry::new(keyring_service(), "spotify_refresh_token");
        let _ = res;
    }

    #[test]
    fn test_keyring_helper_functions_exist() {
        let _ = get_refresh_token_from_keyring();
    }

    #[tokio::test]
    async fn test_with_auto_reauth_success_path() {
        let spotify = get_spotify_client();
        let res = with_auto_reauth(&spotify, || async { Ok::<i32, AppError>(42) }).await;
        assert_eq!(res.unwrap(), 42);
    }

    #[test]
    fn test_map_rspotify_error_invalid_token() {
        let err = rspotify::ClientError::InvalidToken;
        let mapped = map_rspotify_error(err);
        assert!(matches!(mapped, AppError::Auth(_)));
    }

    #[tokio::test]
    async fn test_with_auto_reauth_rate_limit_retry() {
        let spotify = get_spotify_client();
        let counter = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let c = counter.clone();
        let res = with_auto_reauth(&spotify, || {
            let count = c.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            async move {
                if count == 0 {
                    Err(AppError::RateLimited(0))
                } else {
                    Ok(99)
                }
            }
        })
        .await;
        assert_eq!(res.unwrap(), 99);
        assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 2);
    }
}
