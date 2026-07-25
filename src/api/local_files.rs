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
    pub cover_image_bytes: Option<Vec<u8>>,
}

/// Recursively scans a directory for supported local audio files (.mp3, .flac, .wav, .ogg, .m4a).
#[allow(clippy::missing_errors_doc)]
pub fn scan_local_directory(dir_path: &Path) -> Result<Vec<LocalAudioTrack>, AppError> {
    if !dir_path.exists() {
        return Ok(Vec::new());
    }

    let mut tracks = Vec::new();
    scan_directory_recursive(dir_path, &mut tracks);
    Ok(tracks)
}

fn scan_directory_recursive(dir_path: &Path, tracks: &mut Vec<LocalAudioTrack>) {
    let Ok(entries) = fs::read_dir(dir_path) else {
        return;
    };
    let supported_exts = ["mp3", "flac", "wav", "ogg", "m4a"];

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            scan_directory_recursive(&path, tracks);
        } else if path.is_file() {
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

                    let (title, artist) = parse_title_artist_from_filename(stem);
                    let cover_image_bytes = extract_cover_image(&path);

                    tracks.push(LocalAudioTrack {
                        file_name,
                        path: path.clone(),
                        title,
                        artist,
                        duration_ms: 180_000,
                        extension: ext_lower,
                        cover_image_bytes,
                    });
                }
            }
        }
    }
}

fn parse_title_artist_from_filename(stem: &str) -> (String, String) {
    if let Some((artist, title)) = stem.split_once(" - ") {
        (title.trim().to_string(), artist.trim().to_string())
    } else {
        (stem.to_string(), "Local Artist".to_string())
    }
}

#[must_use]
pub fn extract_cover_image(path: &Path) -> Option<Vec<u8>> {
    // First, check directory for cover.jpg/png, folder.jpg/png, album.jpg/png
    if let Some(parent) = path.parent() {
        for cover_name in &[
            "cover.jpg",
            "cover.png",
            "folder.jpg",
            "folder.png",
            "album.jpg",
            "album.png",
        ] {
            let cover_path = parent.join(cover_name);
            if cover_path.is_file() {
                if let Ok(bytes) = fs::read(&cover_path) {
                    if !bytes.is_empty() {
                        return Some(bytes);
                    }
                }
            }
        }
    }

    // Try reading embedded cover bytes from audio file header / tags
    let Ok(data) = fs::read(path) else {
        return None;
    };
    if data.len() < 100 {
        return None;
    }

    let search_limit = data.len().min(512 * 1024);
    let header_slice = &data[..search_limit];

    // Check PNG
    let png_magic = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
    if let Some(png_idx) = header_slice.windows(8).position(|w| w == png_magic) {
        let png_end_magic = [0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82];
        if let Some(end_idx) = header_slice[png_idx..]
            .windows(8)
            .position(|w| w == png_end_magic)
        {
            let img_bytes = header_slice[png_idx..png_idx + end_idx + 8].to_vec();
            if img_bytes.len() > 100 {
                return Some(img_bytes);
            }
        }
    }

    // Check JPEG
    let jpeg_start = [0xFF, 0xD8, 0xFF];
    let jpeg_end = [0xFF, 0xD9];
    if let Some(jpg_idx) = header_slice.windows(3).position(|w| w == jpeg_start) {
        if let Some(end_idx) = header_slice[jpg_idx..]
            .windows(2)
            .position(|w| w == jpeg_end)
        {
            let img_bytes = header_slice[jpg_idx..jpg_idx + end_idx + 2].to_vec();
            if img_bytes.len() > 100 {
                return Some(img_bytes);
            }
        }
    }

    None
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

    #[test]
    fn test_parse_title_artist() {
        let (title, artist) = parse_title_artist_from_filename("Daft Punk - One More Time");
        assert_eq!(title, "One More Time");
        assert_eq!(artist, "Daft Punk");
    }
}
