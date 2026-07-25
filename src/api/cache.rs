use crate::error::AppError;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Returns the local cache directory for Spotifust image and metadata storage.
#[must_use]
pub fn get_cache_dir() -> PathBuf {
    std::env::temp_dir().join("spotifust_cache")
}

/// Helper function to generate a safe filename from a URL.
#[must_use]
pub fn url_to_filename(url: &str) -> String {
    use std::hash::{Hash, Hasher};

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    url.hash(&mut hasher);
    format!("{:x}.img", hasher.finish())
}

/// Image cache manager for downloading and storing album artwork locally.
pub struct ImageCache;

impl ImageCache {
    /// Retrieves a cached image path or downloads it if missing.
    #[allow(clippy::missing_errors_doc)]
    pub async fn get_or_fetch_image(url: &str) -> Result<PathBuf, AppError> {
        let dir = get_cache_dir().join("images");
        fs::create_dir_all(&dir)
            .map_err(|e| AppError::Cache(format!("Failed to create image cache directory: {e}")))?;

        let filename = url_to_filename(url);
        let file_path = dir.join(filename);

        if file_path.exists() {
            return Ok(file_path);
        }

        let bytes = reqwest::get(url)
            .await
            .map_err(|e| AppError::Network(format!("Failed to download image from {url}: {e}")))?
            .bytes()
            .await
            .map_err(|e| AppError::Network(format!("Failed to read image bytes: {e}")))?;

        fs::write(&file_path, bytes)
            .map_err(|e| AppError::Cache(format!("Failed to save image to disk: {e}")))?;

        Ok(file_path)
    }

    /// Fetches image bytes by URL, returning a tuple of (url, bytes).
    #[allow(clippy::missing_errors_doc)]
    pub async fn fetch_image_bytes(url: String) -> Result<(String, Vec<u8>), AppError> {
        let file_path = Self::get_or_fetch_image(&url).await?;
        let bytes = fs::read(&file_path)
            .map_err(|e| AppError::Cache(format!("Failed to read cached image file: {e}")))?;
        Ok((url, bytes))
    }
}

/// TTL-based in-memory metadata cache entry.
struct CacheEntry<T> {
    data: T,
    expires_at: Instant,
}

/// Simple thread-safe in-memory metadata cache with configurable TTL.
pub struct MetadataCache<K, V> {
    store: Mutex<HashMap<K, CacheEntry<V>>>,
    ttl: Duration,
}

impl<K: Eq + std::hash::Hash + Clone, V: Clone> MetadataCache<K, V> {
    #[must_use]
    pub fn new(ttl_secs: u64) -> Self {
        Self {
            store: Mutex::new(HashMap::new()),
            ttl: Duration::from_secs(ttl_secs),
        }
    }

    pub fn get(&self, key: &K) -> Option<V> {
        let mut guard = self.store.lock().ok()?;
        if let Some(entry) = guard.get(key) {
            if Instant::now() < entry.expires_at {
                return Some(entry.data.clone());
            }
        }
        guard.remove(key);
        None
    }

    pub fn insert(&self, key: K, value: V) {
        if let Ok(mut guard) = self.store.lock() {
            guard.insert(
                key,
                CacheEntry {
                    data: value,
                    expires_at: Instant::now() + self.ttl,
                },
            );
        }
    }
}

/// Disk-persistent JSON metadata cache manager.
pub struct DiskMetadataCache;

impl DiskMetadataCache {
    /// Saves a serializable data structure to disk cache under the given key name.
    pub fn save<T: serde::Serialize>(key: &str, data: &T) -> Result<(), AppError> {
        let dir = get_cache_dir().join("metadata");
        fs::create_dir_all(&dir)
            .map_err(|e| AppError::Cache(format!("Failed to create metadata cache dir: {e}")))?;

        let file_path = dir.join(format!("{key}.json"));
        let json_str = serde_json::to_string(data)
            .map_err(|e| AppError::Cache(format!("Failed to serialize cache key {key}: {e}")))?;

        fs::write(file_path, json_str)
            .map_err(|e| AppError::Cache(format!("Failed to write metadata cache key {key}: {e}")))?;

        Ok(())
    }

    /// Loads a deserializable data structure from disk cache under the given key name.
    pub fn load<T: serde::de::DeserializeOwned>(key: &str) -> Option<T> {
        let file_path = get_cache_dir().join("metadata").join(format!("{key}.json"));
        if !file_path.exists() {
            return None;
        }
        let json_str = fs::read_to_string(file_path).ok()?;
        serde_json::from_str(&json_str).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_to_filename() {
        let fn1 = url_to_filename("https://example.com/cover1.jpg");
        let fn2 = url_to_filename("https://example.com/cover2.jpg");
        assert_ne!(fn1, fn2);
        assert!(
            std::path::Path::new(&fn1)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("img"))
        );
    }

    #[test]
    fn test_metadata_cache_ttl() {
        let cache = MetadataCache::<String, String>::new(60);
        cache.insert("artist_1".to_string(), "The Midnight".to_string());
        assert_eq!(
            cache.get(&"artist_1".to_string()),
            Some("The Midnight".to_string())
        );
        assert_eq!(cache.get(&"unknown".to_string()), None);
    }

    #[test]
    fn test_disk_metadata_cache() {
        let key = "test_key_spotifust";
        let data = vec!["item1".to_string(), "item2".to_string()];
        assert!(DiskMetadataCache::save(key, &data).is_ok());

        let loaded: Option<Vec<String>> = DiskMetadataCache::load(key);
        assert_eq!(loaded, Some(data));
    }
}
