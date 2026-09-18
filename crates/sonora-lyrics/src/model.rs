use serde::{Deserialize, Serialize};

/// Supported format types for incoming lyric payloads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LyricsFormat {
    Plain,
    Lrc,
    EnhancedLrc,
    Ttml,
}

/// A syllable-level or word-level timing segment for kinetic typography.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LyricSyllable {
    pub text: String,
    pub start_time_ms: u64,
    pub end_time_ms: u64,
}

/// A single line of lyrics with start timestamp and optional word/syllable subdivisions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LyricLine {
    pub start_time_ms: u64,
    pub end_time_ms: Option<u64>,
    pub text: String,
    pub syllables: Vec<LyricSyllable>,
}

/// Universal AST representation of parsed lyrics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct LyricsDocument {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub offset_ms: i64,
    pub lines: Vec<LyricLine>,
}
