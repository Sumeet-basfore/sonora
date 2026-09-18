use crate::model::{LyricsDocument, LyricsFormat};
use sonora_common::Result;

/// Trait implemented by lyrics format parsers.
pub trait LyricsParser {
    fn format(&self) -> LyricsFormat;
    fn parse(&self, raw: &str) -> Result<LyricsDocument>;
}

/// Fallback plain text lyrics parser.
pub struct PlainTextLyricsParser;

impl LyricsParser for PlainTextLyricsParser {
    fn format(&self) -> LyricsFormat {
        LyricsFormat::Plain
    }

    fn parse(&self, raw: &str) -> Result<LyricsDocument> {
        let lines = raw
            .lines()
            .map(|line| crate::model::LyricLine {
                start_time_ms: 0,
                end_time_ms: None,
                text: line.to_string(),
                syllables: Vec::new(),
            })
            .collect();

        Ok(LyricsDocument {
            lines,
            ..Default::default()
        })
    }
}
