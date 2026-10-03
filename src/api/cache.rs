use crate::error::AppError;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};

#[must_use]
pub fn get_cache_dir() -> PathBuf {
    // Unit tests exercise "session expired" flows that clear the cache; keep them
    // away from the real user cache and settings.
    if cfg!(test) {
        return std::env::temp_dir().join("spotifust-test-cache");
    }
    if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".cache").join("spotifust")
    } else {
        std::env::temp_dir().join("spotifust_cache")
    }
}

/// Helper function to generate a safe filename from a URL.
#[must_use]
pub fn url_to_filename(url: &str) -> String {
    use std::hash::{Hash, Hasher};

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    url.hash(&mut hasher);
    format!("{:x}.img", hasher.finish())
}

/// Longest edge, in pixels, of cached artwork. Covers are drawn at most ~240px wide.
const THUMBNAIL_EDGE: u32 = 256;

fn image_http_client() -> &'static reqwest::Client {
    static CLIENT: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();
    // One pooled client: reusing TLS connections to i.scdn.co makes a grid of
    // covers load an order of magnitude faster than a handshake per image.
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(Duration::from_secs(20))
            .pool_max_idle_per_host(8)
            .build()
            .unwrap_or_default()
    })
}

/// Image cache manager for downloading and storing album artwork locally.
pub struct ImageCache;

impl ImageCache {
    /// Returns thumbnail bytes for `url`, from disk when cached or downloaded otherwise.
    ///
    /// Decoding and resizing happen on the blocking pool so a burst of covers never
    /// stalls the async workers that also drive the audio session.
    #[allow(clippy::missing_errors_doc)]
    pub async fn fetch_image_bytes(url: String) -> Result<(String, Vec<u8>), AppError> {
        let dir = get_cache_dir().join("images");
        let file_path = dir.join(url_to_filename(&url));

        let cached_path = file_path.clone();
        if let Ok(Ok(bytes)) = tokio::task::spawn_blocking(move || fs::read(cached_path)).await {
            if !bytes.is_empty() {
                return Ok((url, bytes));
            }
        }

        let resp = image_http_client()
            .get(&url)
            .send()
            .await
            .map_err(|e| AppError::Network(format!("Failed to download image from {url}: {e}")))?;
        if !resp.status().is_success() {
            return Err(AppError::Network(format!(
                "Image download returned status {}",
                resp.status()
            )));
        }
        let bytes = resp
            .bytes()
            .await
            .map_err(|e| AppError::Network(format!("Failed to read image bytes: {e}")))?;
        if bytes.is_empty() {
            return Err(AppError::Network("Downloaded empty image bytes".into()));
        }

        let processed = tokio::task::spawn_blocking(move || {
            let processed = optimize_image_bytes(&bytes);
            let _ = fs::create_dir_all(&dir);
            let _ = fs::write(&file_path, &processed);
            processed
        })
        .await
        .map_err(|e| AppError::Cache(format!("Image processing task failed: {e}")))?;
        Ok((url, processed))
    }
}

/// Downscales artwork to [`THUMBNAIL_EDGE`] so the renderer decodes and scales
/// small images instead of 640px originals on every frame they're drawn.
fn optimize_image_bytes(bytes: &[u8]) -> Vec<u8> {
    if let Ok(img) = image::load_from_memory(bytes) {
        if img.width() > THUMBNAIL_EDGE || img.height() > THUMBNAIL_EDGE {
            let thumb = img.thumbnail(THUMBNAIL_EDGE, THUMBNAIL_EDGE);
            let mut cursor = std::io::Cursor::new(Vec::new());
            let format = if img.color().has_alpha() {
                image::ImageFormat::Png
            } else {
                image::ImageFormat::Jpeg
            };
            if thumb.write_to(&mut cursor, format).is_ok() {
                return cursor.into_inner();
            }
        }
    }
    bytes.to_vec()
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
    #[allow(clippy::missing_errors_doc)]
    pub fn save<T: serde::Serialize>(key: &str, data: &T) -> Result<(), AppError> {
        let dir = get_cache_dir().join("metadata");
        fs::create_dir_all(&dir)
            .map_err(|e| AppError::Cache(format!("Failed to create metadata cache dir: {e}")))?;

        let file_path = dir.join(format!("{key}.json"));
        let json_str = serde_json::to_string(data)
            .map_err(|e| AppError::Cache(format!("Failed to serialize cache key {key}: {e}")))?;

        fs::write(file_path, json_str).map_err(|e| {
            AppError::Cache(format!("Failed to write metadata cache key {key}: {e}"))
        })?;

        Ok(())
    }

    #[must_use]
    pub fn load<T: serde::de::DeserializeOwned>(key: &str) -> Option<T> {
        let file_path = get_cache_dir().join("metadata").join(format!("{key}.json"));
        if !file_path.exists() {
            return None;
        }
        let json_str = fs::read_to_string(file_path).ok()?;
        serde_json::from_str(&json_str).ok()
    }
}

#[must_use]
pub fn calculate_cache_size_bytes() -> u64 {
    let dir = get_cache_dir();
    calculate_dir_size(&dir)
}

fn calculate_dir_size(path: &std::path::Path) -> u64 {
    let mut total_size = 0;
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_dir() {
                    total_size += calculate_dir_size(&entry.path());
                } else {
                    total_size += metadata.len();
                }
            }
        }
    }
    total_size
}

#[allow(clippy::missing_errors_doc)]
pub fn clear_cache_disk() -> Result<u64, AppError> {
    let dir = get_cache_dir();
    let freed_bytes = calculate_dir_size(&dir);
    let images_dir = dir.join("images");
    if images_dir.exists() {
        let _ = fs::remove_dir_all(&images_dir);
        let _ = fs::create_dir_all(&images_dir);
    }
    let metadata_dir = dir.join("metadata");
    if metadata_dir.exists() {
        if let Ok(entries) = fs::read_dir(&metadata_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                let is_setting = matches!(
                    name,
                    "saved_volume.json"
                        | "ui_scale.json"
                        | "accent_tone.json"
                        | "ui_language.json"
                        | "audio_bitrate.json"
                        | "audio_normalization.json"
                        | "gapless_playback.json"
                        | "last_playback_state.json"
                );
                if !is_setting && path.is_file() {
                    let _ = fs::remove_file(path);
                }
            }
        }
    }
    Ok(freed_bytes)
}

#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;

    let bytes_f = bytes as f64;
    if bytes_f >= GB {
        format!("{:.1} GB", bytes_f / GB)
    } else if bytes_f >= MB {
        format!("{:.1} MB", bytes_f / MB)
    } else if bytes_f >= KB {
        format!("{:.1} KB", bytes_f / KB)
    } else {
        format!("{bytes} B")
    }
}

#[derive(Debug, Clone)]
pub struct LruCache<K, V> {
    entries: HashMap<K, V>,
    order: std::collections::VecDeque<K>,
    capacity: usize,
}

impl<K: Eq + std::hash::Hash + Clone, V> LruCache<K, V> {
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        let cap = capacity.max(1);
        Self {
            entries: HashMap::with_capacity(cap),
            order: std::collections::VecDeque::with_capacity(cap),
            capacity: cap,
        }
    }

    pub fn get(&mut self, key: &K) -> Option<&V> {
        if self.entries.contains_key(key) {
            self.promote(key);
            self.entries.get(key)
        } else {
            None
        }
    }

    #[must_use]
    pub fn peek(&self, key: &K) -> Option<&V> {
        self.entries.get(key)
    }

    #[must_use]
    pub fn contains_key(&self, key: &K) -> bool {
        self.entries.contains_key(key)
    }

    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        if self.entries.contains_key(&key) {
            self.promote(&key);
            self.entries.insert(key, value)
        } else {
            if self.entries.len() >= self.capacity {
                if let Some(oldest) = self.order.pop_front() {
                    self.entries.remove(&oldest);
                }
            }
            self.order.push_back(key.clone());
            self.entries.insert(key, value)
        }
    }

    pub fn remove(&mut self, key: &K) -> Option<V> {
        if let Some(val) = self.entries.remove(key) {
            self.order.retain(|k| k != key);
            Some(val)
        } else {
            None
        }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    #[must_use]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.order.clear();
    }

    #[must_use]
    pub fn inner_map(&self) -> &HashMap<K, V> {
        &self.entries
    }

    fn promote(&mut self, key: &K) {
        self.order.retain(|k| k != key);
        self.order.push_back(key.clone());
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

    #[test]
    fn test_get_cache_dir() {
        let dir = get_cache_dir();
        assert!(dir.to_string_lossy().contains("spotifust"));
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(500), "500 B");
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.0 MB");
        assert_eq!(format_bytes(1024 * 1024 * 1024 * 2), "2.0 GB");
    }

    #[test]
    fn test_calculate_dir_size() {
        let temp_dir = std::env::temp_dir().join("spotifust_test_size");
        let _ = fs::remove_dir_all(&temp_dir);
        let _ = fs::create_dir_all(&temp_dir);
        let test_file = temp_dir.join("test.bin");
        let _ = fs::write(&test_file, [0u8; 100]);
        let size = calculate_dir_size(&temp_dir);
        assert_eq!(size, 100);
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_lru_cache_bounded_capacity_and_eviction() {
        let mut lru = LruCache::<String, i32>::new(3);
        assert_eq!(lru.capacity(), 3);
        assert!(lru.is_empty());

        lru.insert("a".to_string(), 1);
        lru.insert("b".to_string(), 2);
        lru.insert("c".to_string(), 3);
        assert_eq!(lru.len(), 3);

        lru.insert("d".to_string(), 4);
        assert_eq!(lru.len(), 3);
        assert!(!lru.contains_key(&"a".to_string()));
        assert!(lru.contains_key(&"b".to_string()));
        assert!(lru.contains_key(&"c".to_string()));
        assert!(lru.contains_key(&"d".to_string()));
    }

    #[test]
    fn test_lru_cache_promotion_on_access() {
        let mut lru = LruCache::<String, i32>::new(3);
        lru.insert("a".to_string(), 1);
        lru.insert("b".to_string(), 2);
        lru.insert("c".to_string(), 3);

        assert_eq!(lru.get(&"a".to_string()), Some(&1));

        lru.insert("d".to_string(), 4);
        assert!(lru.contains_key(&"a".to_string()));
        assert!(!lru.contains_key(&"b".to_string()));
        assert!(lru.contains_key(&"c".to_string()));
        assert!(lru.contains_key(&"d".to_string()));

        lru.remove(&"c".to_string());
        assert_eq!(lru.len(), 2);
        assert!(!lru.contains_key(&"c".to_string()));

        lru.clear();
        assert!(lru.is_empty());
    }
}
