use crate::model::LyricsDocument;
use sonora_common::Result;

/// Trait implemented by lyrics providers in the cascading resolver hierarchy.
pub trait LyricsProvider: Send + Sync {
    fn name(&self) -> &'static str;
    fn fetch(&self, track_title: &str, artist: &str) -> Result<Option<LyricsDocument>>;
}
