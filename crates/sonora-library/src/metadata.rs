use lofty::file::{AudioFile, TaggedFileExt};
use lofty::probe::Probe;
use lofty::tag::Accessor;
use sonora_common::{Result, SonoraError};
use std::path::Path;

/// Extracted audio metadata and file stream properties.
#[derive(Debug, Clone, Default)]
pub struct ExtractedMetadata {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub album_artist: Option<String>,
    pub track_number: Option<u32>,
    pub total_tracks: Option<u32>,
    pub disc_number: Option<u32>,
    pub year: Option<u32>,
    pub genre: Option<String>,
    pub duration_ms: u64,
    pub sample_rate: Option<u32>,
    pub channels: Option<u8>,
    pub bit_depth: Option<u8>,
    pub bitrate_kbps: Option<u32>,
}

/// Metadata extraction engine wrapping Lofty.
pub struct MetadataExtractor;

impl MetadataExtractor {
    pub fn extract<P: AsRef<Path>>(path: P) -> Result<ExtractedMetadata> {
        let tagged_file = Probe::open(path.as_ref())
            .map_err(|e| SonoraError::Library(format!("Probe error: {e}")))?
            .read()
            .map_err(|e| SonoraError::Library(format!("Metadata read error: {e}")))?;

        let properties = tagged_file.properties();
        let duration_ms = properties.duration().as_millis() as u64;
        let sample_rate = properties.sample_rate();
        let channels = properties.channels();
        let bit_depth = properties.bit_depth();
        let bitrate_kbps = properties.audio_bitrate();

        let tag = tagged_file
            .primary_tag()
            .or_else(|| tagged_file.first_tag());

        let mut meta = ExtractedMetadata {
            duration_ms,
            sample_rate,
            channels,
            bit_depth,
            bitrate_kbps,
            ..Default::default()
        };

        if let Some(tag) = tag {
            meta.title = tag.title().as_deref().map(ToString::to_string);
            meta.artist = tag.artist().as_deref().map(ToString::to_string);
            meta.album = tag.album().as_deref().map(ToString::to_string);
            meta.genre = tag.genre().as_deref().map(ToString::to_string);
            meta.track_number = tag.track();
            meta.total_tracks = tag.track_total();
            meta.disc_number = tag.disk();
            meta.year = tag.date().map(|d| d.year as u32);
        }

        Ok(meta)
    }
}
