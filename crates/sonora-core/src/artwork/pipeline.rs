use crate::artwork::models::CachedArtworkAsset;
use crate::metadata::error::ProviderError;
use image::imageops::FilterType;
use image::{DynamicImage, GenericImageView, ImageFormat};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::BufWriter;
use std::path::{Path, PathBuf};

/// Security limits defined in docs/25 and ADR-010.
pub const MAX_IMAGE_PAYLOAD_BYTES: usize = 64 * 1024 * 1024; // 64 MB
pub const MAX_IMAGE_DIMENSION: u32 = 8192; // 8192 x 8192 px max
pub const FULL_ARTWORK_DIMENSION: u32 = 1200; // 1200 x 1200 px target
pub const THUMBNAIL_DIMENSION: u32 = 300; // 300 x 300 px target

/// Zero-Trust Media Asset Sanitization and WebP Texture Pipeline.
#[derive(Debug, Clone)]
pub struct ArtworkPipeline {
    cache_root: PathBuf,
}

impl ArtworkPipeline {
    /// Create a new artwork pipeline with a specified cache root directory.
    pub fn new<P: AsRef<Path>>(cache_root: P) -> Self {
        Self {
            cache_root: cache_root.as_ref().to_path_buf(),
        }
    }

    /// Default cache directory under standard XDG / OS cache path: `$CACHE/sonora/covers`.
    pub fn default_cache_dir() -> PathBuf {
        let base = dirs_next().unwrap_or_else(|| PathBuf::from(".cache"));
        base.join("sonora").join("covers")
    }

    /// Full resolution cache directory (`$CACHE_ROOT/full`).
    pub fn full_dir(&self) -> PathBuf {
        self.cache_root.join("full")
    }

    /// Thumbnail cache directory (`$CACHE_ROOT/thumbnails`).
    pub fn thumbnail_dir(&self) -> PathBuf {
        self.cache_root.join("thumbnails")
    }

    /// Initialize cache subdirectories.
    pub fn init_cache_dirs(&self) -> Result<(), ProviderError> {
        fs::create_dir_all(self.full_dir()).map_err(|e| {
            ProviderError::Storage(format!("Failed to create full cover cache dir: {e}"))
        })?;
        fs::create_dir_all(self.thumbnail_dir()).map_err(|e| {
            ProviderError::Storage(format!("Failed to create thumbnail cover cache dir: {e}"))
        })?;
        Ok(())
    }

    /// Compute a deterministic SHA256 hex key for a URL or payload identifier.
    pub fn compute_cache_key(identifier: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(identifier.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Validate, sanitize, decode, normalize, and cache raw image bytes.
    /// Returns the cached asset metadata if successful.
    pub fn process_and_cache(
        &self,
        identifier: &str,
        raw_bytes: &[u8],
    ) -> Result<CachedArtworkAsset, ProviderError> {
        // 1. Payload Size Validation (ADR-010)
        if raw_bytes.is_empty() {
            return Err(ProviderError::Parse("Image payload is empty".to_string()));
        }
        if raw_bytes.len() > MAX_IMAGE_PAYLOAD_BYTES {
            return Err(ProviderError::Parse(format!(
                "Image payload exceeds maximum allocation limit of 64MB: {} bytes",
                raw_bytes.len()
            )));
        }

        // 2. Safe Pure-Rust Decoding
        let img = image::load_from_memory(raw_bytes).map_err(|e| {
            ProviderError::Parse(format!("Failed to decode image in pure-Rust decoder: {e}"))
        })?;

        // 3. Dimension Guard Check (Decompression bomb protection)
        let (width, height) = img.dimensions();
        if width == 0 || height == 0 {
            return Err(ProviderError::Parse(
                "Image has zero width or height".to_string(),
            ));
        }
        if width > MAX_IMAGE_DIMENSION || height > MAX_IMAGE_DIMENSION {
            return Err(ProviderError::Parse(format!(
                "Image dimensions ({width}x{height}) exceed maximum allowed dimension ({MAX_IMAGE_DIMENSION}x{MAX_IMAGE_DIMENSION})"
            )));
        }

        // 4. Ensure directories exist
        self.init_cache_dirs()?;

        let key = Self::compute_cache_key(identifier);
        let full_path = self.full_dir().join(format!("{key}.webp"));
        let thumb_path = self.thumbnail_dir().join(format!("{key}.webp"));

        // 5. Generate Full-Res Texture (Max 1200x1200px, aspect-ratio preserved)
        let full_img = if width > FULL_ARTWORK_DIMENSION || height > FULL_ARTWORK_DIMENSION {
            img.resize(
                FULL_ARTWORK_DIMENSION,
                FULL_ARTWORK_DIMENSION,
                FilterType::Lanczos3,
            )
        } else {
            img.clone()
        };

        // 6. Generate Thumbnail Texture (300x300px, aspect-ratio preserved)
        let thumb_img = img.resize(
            THUMBNAIL_DIMENSION,
            THUMBNAIL_DIMENSION,
            FilterType::Lanczos3,
        );

        // 7. Write to cache files safely
        Self::save_image_safe(&full_img, &full_path)?;
        Self::save_image_safe(&thumb_img, &thumb_path)?;

        let metadata = fs::metadata(&full_path)
            .map_err(|e| ProviderError::Storage(format!("Failed to stat cached artwork: {e}")))?;

        Ok(CachedArtworkAsset {
            key,
            full_path: full_path.to_string_lossy().to_string(),
            thumbnail_path: thumb_path.to_string_lossy().to_string(),
            width: full_img.width(),
            height: full_img.height(),
            mime_type: "image/webp".to_string(),
            file_size_bytes: metadata.len(),
        })
    }

    /// Checks whether an artwork asset for the given identifier already exists on disk.
    pub fn get_cached_asset(&self, identifier: &str) -> Option<CachedArtworkAsset> {
        let key = Self::compute_cache_key(identifier);
        let full_path = self.full_dir().join(format!("{key}.webp"));
        let thumb_path = self.thumbnail_dir().join(format!("{key}.webp"));

        if full_path.is_file() && thumb_path.is_file() {
            if let Ok(meta) = fs::metadata(&full_path) {
                if meta.len() > 0 {
                    return Some(CachedArtworkAsset {
                        key,
                        full_path: full_path.to_string_lossy().to_string(),
                        thumbnail_path: thumb_path.to_string_lossy().to_string(),
                        width: 0, // dynamic when read
                        height: 0,
                        mime_type: "image/webp".to_string(),
                        file_size_bytes: meta.len(),
                    });
                }
            }
        }
        None
    }

    fn save_image_safe(img: &DynamicImage, path: &Path) -> Result<(), ProviderError> {
        let file = File::create(path).map_err(|e| {
            ProviderError::Storage(format!(
                "Failed to create artwork cache file {:?}: {e}",
                path
            ))
        })?;
        let mut writer = BufWriter::new(file);

        // Attempt WebP save, fallback to PNG if WebP writer is not enabled
        if let Err(e) = img.write_to(&mut writer, ImageFormat::WebP) {
            tracing::debug!("WebP write fallback to PNG: {e}");
            let file = File::create(path).map_err(|e| {
                ProviderError::Storage(format!(
                    "Failed to recreate artwork cache file {:?}: {e}",
                    path
                ))
            })?;
            let mut writer = BufWriter::new(file);
            img.write_to(&mut writer, ImageFormat::Png).map_err(|e| {
                ProviderError::Storage(format!("Failed to encode image to {:?}: {e}", path))
            })?;
        }

        Ok(())
    }
}

fn dirs_next() -> Option<PathBuf> {
    std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))
}
