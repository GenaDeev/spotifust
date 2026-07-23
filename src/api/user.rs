use crate::api::auth::with_auto_reauth;
use crate::error::AppError;
use rspotify::{AuthCodePkceSpotify, clients::OAuthClient};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserProfile {
    pub id: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub product: Option<String>,
}

/// Fetches the current authenticated user's profile (`/me`) from Spotify.
#[allow(clippy::missing_errors_doc)]
pub async fn fetch_user_profile(spotify: &AuthCodePkceSpotify) -> Result<UserProfile, AppError> {
    with_auto_reauth(spotify, || async {
        let user = spotify
            .current_user()
            .await
            .map_err(|e| AppError::Network(format!("Failed to fetch user profile: {e}")))?;

        let display_name = user
            .display_name
            .clone()
            .unwrap_or_else(|| user.id.to_string());

        let avatar_url = user
            .images
            .as_ref()
            .and_then(|imgs| imgs.first())
            .map(|img| img.url.clone());

        #[allow(deprecated)]
        let product = user.product.map(|p| format!("{p:?}"));

        Ok(UserProfile {
            id: user.id.to_string(),
            display_name,
            avatar_url,
            product,
        })
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_profile_struct() {
        let profile = UserProfile {
            id: "test_user".to_string(),
            display_name: "Test User".to_string(),
            avatar_url: Some("https://example.com/avatar.jpg".to_string()),
            product: Some("premium".to_string()),
        };
        assert_eq!(profile.display_name, "Test User");
        assert_eq!(profile.id, "test_user");
    }
}
