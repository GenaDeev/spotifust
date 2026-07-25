use crate::error::AppError;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct UpdateInfo {
    pub current_version: String,
    pub latest_version: String,
    pub has_update: bool,
    pub download_url: Option<String>,
}

#[derive(serde::Deserialize)]
struct GitHubRelease {
    tag_name: String,
    html_url: String,
}

/// Checks GitHub API for the latest release tag of Spotifust.
#[allow(clippy::missing_errors_doc)]
pub async fn check_for_updates() -> Result<UpdateInfo, AppError> {
    let current_version = env!("CARGO_PKG_VERSION").to_string();

    let client = reqwest::Client::builder()
        .user_agent("Spotifust-Desktop-Client")
        .build()
        .map_err(|e| AppError::Network(format!("Failed to build HTTP client for updater: {e}")))?;

    let url = "https://api.github.com/repos/elgena/spotifust/releases/latest";
    let res = client
        .get(url)
        .send()
        .await
        .map_err(|e| AppError::Network(format!("Update check HTTP request failed: {e}")))?;

    if !res.status().is_success() {
        return Ok(UpdateInfo {
            current_version: current_version.clone(),
            latest_version: current_version,
            has_update: false,
            download_url: None,
        });
    }

    if let Ok(release) = res.json::<GitHubRelease>().await {
        let latest_tag = release.tag_name.trim_start_matches('v').to_string();
        let has_update = latest_tag != current_version;
        return Ok(UpdateInfo {
            current_version,
            latest_version: latest_tag,
            has_update,
            download_url: Some(release.html_url),
        });
    }

    Ok(UpdateInfo {
        current_version: current_version.clone(),
        latest_version: current_version,
        has_update: false,
        download_url: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_update_info_struct() {
        let info = UpdateInfo {
            current_version: "0.1.0".to_string(),
            latest_version: "0.1.0".to_string(),
            has_update: false,
            download_url: None,
        };
        assert_eq!(info.current_version, "0.1.0");
        assert!(!info.has_update);
    }
}
