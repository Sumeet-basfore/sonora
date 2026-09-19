use serde::{Deserialize, Serialize};

/// Identifies the provenance source of an artwork asset.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "details")]
pub enum ArtworkSourceType {
    EmbeddedTag {
        file_path: String,
    },
    LocalSidecar {
        file_path: String,
    },
    UserCustom {
        file_path: String,
    },
    CoverArtArchive {
        release_mbid: Option<String>,
        release_group_mbid: Option<String>,
    },
    FanartTv {
        artist_mbid: String,
    },
    Wikidata {
        image_url: String,
    },
}

/// The visual kind / role of the artwork asset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArtworkKind {
    FrontCover,
    BackCover,
    Booklet,
    Medium, // Vinyl disc, CD label scan, cassette face
    ArtistPortrait,
    ArtistBackground,
    ArtistBanner,
    ArtistLogo,
    Other,
}

/// An artwork candidate discovered from a local or remote provider.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArtworkCandidate {
    pub id: String,
    pub provider_name: String,
    pub source_type: ArtworkSourceType,
    pub kind: ArtworkKind,
    pub original_url: String,
    pub preview_thumbnail_url: String,
    pub width: u32,
    pub height: u32,
    pub format: String, // e.g. "JPEG", "PNG", "WebP"
    pub size_bytes: Option<u64>,
    pub match_confidence: f32, // 0.0 to 1.0
    pub is_canonical: bool,
}

/// Query parameters for artwork discovery.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ArtworkQuery {
    pub release_mbid: Option<String>,
    pub release_group_mbid: Option<String>,
    pub artist_mbid: Option<String>,
    pub artist_name: Option<String>,
    pub album_title: Option<String>,
}

/// Information about a locally cached artwork asset (full-res and thumbnail).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CachedArtworkAsset {
    pub key: String,
    pub full_path: String,
    pub thumbnail_path: String,
    pub width: u32,
    pub height: u32,
    pub mime_type: String,
    pub file_size_bytes: u64,
}
