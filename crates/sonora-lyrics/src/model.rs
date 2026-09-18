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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LyricsDocument {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub offset_ms: i64,
    pub format: LyricsFormat,
    pub lines: Vec<LyricLine>,
}

impl Default for LyricsDocument {
    fn default() -> Self {
        Self {
            title: None,
            artist: None,
            album: None,
            offset_ms: 0,
            format: LyricsFormat::Plain,
            lines: Vec::new(),
        }
    }
}

impl LyricsDocument {
    pub fn is_synced(&self) -> bool {
        self.format != LyricsFormat::Plain
            && self
                .lines
                .iter()
                .any(|l| l.start_time_ms > 0 || l.end_time_ms.is_some())
    }
}
