use crate::error::AppError;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LocalAudioTrack {
    pub file_name: String,
    pub path: PathBuf,
    pub title: String,
    pub artist: String,
    pub duration_ms: u32,
    pub extension: String,
}

/// Scans a directory for supported local audio files (.mp3, .flac, .wav, .ogg, .m4a).
#[allow(clippy::missing_errors_doc)]
pub fn scan_local_directory(dir_path: &Path) -> Result<Vec<LocalAudioTrack>, AppError> {
    if !dir_path.exists() {
        return Ok(Vec::new());
    }

    let entries = fs::read_dir(dir_path)
        .map_err(|e| AppError::Cache(format!("Failed to read local music directory: {e}")))?;

    let mut tracks = Vec::new();
    let supported_exts = ["mp3", "flac", "wav", "ogg", "m4a"];

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                let ext_lower = ext.to_lowercase();
                if supported_exts.contains(&ext_lower.as_str()) {
                    let file_name = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("Unknown Track")
                        .to_string();

                    let stem = path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or(&file_name);

                    let title = stem.to_string();

                    tracks.push(LocalAudioTrack {
                        file_name,
                        path: path.clone(),
                        title,
                        artist: "Local File".to_string(),
                        duration_ms: 180_000,
                        extension: ext_lower,
                    });
                }
            }
        }
    }

    Ok(tracks)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_local_directory_non_existent() {
        let path = Path::new("/non_existent_spotifust_dir_12345");
        let result = scan_local_directory(path);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }
}
