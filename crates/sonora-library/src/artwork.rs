use image::ImageFormat;
use std::collections::HashMap;
use std::io::Cursor;
use std::path::Path;
use std::sync::RwLock;

/// Base64 RFC 4648 encoder helper.
fn to_base64(data: &[u8]) -> String {
    const CHARSET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity(data.len().div_ceil(3) * 4);

    for chunk in data.chunks(3) {
        let b0 = chunk[0];
        let b1 = if chunk.len() > 1 { chunk[1] } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] } else { 0 };

        result.push(CHARSET[(b0 >> 2) as usize] as char);
        result.push(CHARSET[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize] as char);

        if chunk.len() > 1 {
            result.push(CHARSET[(((b1 & 0x0f) << 2) | (b2 >> 6)) as usize] as char);
        } else {
            result.push('=');
        }

        if chunk.len() > 2 {
            result.push(CHARSET[(b2 & 0x3f) as usize] as char);
        } else {
            result.push('=');
        }
    }
    result
}

/// Cached artwork data URL strings for a track or album.
#[derive(Clone, Debug, Default)]
pub struct CachedArtwork {
    pub thumbnail: Option<String>,
    pub full: Option<String>,
}

/// High-performance thread-safe artwork cache.
/// Avoids repeated disk I/O, file tag parsing, and base64 conversions.
/// Generates 256x256 thumbnails for library views while preserving original artwork.
pub struct ArtworkCache {
    memory_cache: RwLock<HashMap<i64, CachedArtwork>>,
}

impl Default for ArtworkCache {
    fn default() -> Self {
        Self::new()
    }
}

impl ArtworkCache {
    pub fn new() -> Self {
        Self {
            memory_cache: RwLock::new(HashMap::new()),
        }
    }

    /// Retrieve artwork for a track from cache or decode from file on disk.
    pub fn get_or_load_track_artwork<P: AsRef<Path>>(
        &self,
        track_id: i64,
        file_path: P,
        thumbnail: bool,
    ) -> Option<String> {
        // 1. Check memory cache
        {
            let cache = self.memory_cache.read().ok()?;
            if let Some(entry) = cache.get(&track_id) {
                if thumbnail {
                    if entry.thumbnail.is_some() {
                        return entry.thumbnail.clone();
                    }
                } else if entry.full.is_some() {
                    return entry.full.clone();
                }
            }
        }

        // 2. Extract raw image bytes from audio file or local directory
        let (raw_data, mime_type) = Self::extract_raw_artwork(file_path.as_ref())?;

        // 3. Generate full data URL
        let full_data_url = format!("data:{mime_type};base64,{}", to_base64(&raw_data));

        // 4. Generate 256x256 thumbnail data URL
        let thumb_data_url = Self::create_thumbnail(&raw_data)
            .map(|thumb_bytes| format!("data:image/jpeg;base64,{}", to_base64(&thumb_bytes)))
            .unwrap_or_else(|| full_data_url.clone());

        // 5. Store in cache
        if let Ok(mut cache) = self.memory_cache.write() {
            cache.insert(
                track_id,
                CachedArtwork {
                    thumbnail: Some(thumb_data_url.clone()),
                    full: Some(full_data_url.clone()),
                },
            );
        }

        if thumbnail {
            Some(thumb_data_url)
        } else {
            Some(full_data_url)
        }
    }

    /// Resize an image buffer into a 256x256 JPEG thumbnail.
    fn create_thumbnail(raw_bytes: &[u8]) -> Option<Vec<u8>> {
        let img = image::load_from_memory(raw_bytes).ok()?;
        let thumb = img.thumbnail(256, 256);
        let mut out_buf = Cursor::new(Vec::with_capacity(16 * 1024));
        thumb.write_to(&mut out_buf, ImageFormat::Jpeg).ok()?;
        Some(out_buf.into_inner())
    }

    /// Extract raw image bytes from audio metadata tags or directory cover images.
    fn extract_raw_artwork(path: &Path) -> Option<(Vec<u8>, String)> {
        // 1. Check embedded tags via Lofty
        use lofty::file::TaggedFileExt;
        if let Ok(tagged_file) = lofty::probe::Probe::open(path).and_then(|pr| pr.read()) {
            let tag = tagged_file
                .primary_tag()
                .or_else(|| tagged_file.first_tag());
            if let Some(tag) = tag {
                if let Some(picture) = tag.pictures().first() {
                    let mime = picture
                        .mime_type()
                        .map(|m| m.as_str().to_string())
                        .unwrap_or_else(|| "image/jpeg".to_string());
                    return Some((picture.data().to_vec(), mime));
                }
            }
        }

        // 2. Check directory files
        if let Some(parent) = path.parent() {
            let candidates = [
                "cover.jpg",
                "cover.jpeg",
                "cover.png",
                "folder.jpg",
                "folder.jpeg",
                "folder.png",
                "front.jpg",
                "album.jpg",
            ];
            for c in &candidates {
                let candidate_path = parent.join(c);
                if candidate_path.is_file() {
                    if let Ok(data) = std::fs::read(&candidate_path) {
                        let mime = if c.ends_with(".png") {
                            "image/png".to_string()
                        } else {
                            "image/jpeg".to_string()
                        };
                        return Some((data, mime));
                    }
                }
            }
        }

        None
    }

    /// Clear all cached artwork.
    pub fn clear(&self) {
        if let Ok(mut cache) = self.memory_cache.write() {
            cache.clear();
        }
    }
}
