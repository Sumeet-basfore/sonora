use serde::{Deserialize, Serialize};

/// Type of synchronization present in lyrics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LyricsSyncType {
    SyllableSynced, // Word or syllable level (TTML / Enhanced LRC)
    LineSynced,     // Standard LRC line timestamps
    PlainText,      // Unsynchronized text block
}

/// Provenance of the lyrics candidate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "details")]
pub enum LyricsSourceKind {
    EmbeddedTag,
    LocalSidecar { path: String },
    SQLiteCache,
    LrclibPublicApi { id: u64 },
    PluginProvider { plugin_id: String },
    UserManualEdit,
}

/// A candidate lyric result from local extraction or remote providers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LyricsCandidate {
    pub candidate_id: String,
    pub source_kind: LyricsSourceKind,
    pub provider_name: String,
    pub sync_type: LyricsSyncType,
    pub track_name: String,
    pub artist_name: String,
    pub album_name: Option<String>,
    pub duration_seconds: f64,
    pub duration_delta_seconds: f64, // |track_duration - candidate_duration|
    pub match_confidence: f32,       // 0.0 to 1.0
    pub language_code: Option<String>,
    pub is_instrumental: bool,
    pub raw_content: String,
}

/// Query parameters for searching or fetching lyrics candidates.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LyricsCandidateQuery {
    pub track_name: String,
    pub artist_name: Option<String>,
    pub album_name: Option<String>,
    pub duration_seconds: Option<f64>,
}
