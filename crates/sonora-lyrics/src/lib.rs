pub mod model;
pub mod parser;
pub mod provider;

pub use model::{LyricLine, LyricSyllable, LyricsDocument, LyricsFormat};
pub use parser::{LrcLyricsParser, LyricsParser, PlainTextLyricsParser};
pub use provider::{
    CascadingLyricsResolver, EmbeddedLyricsProvider, LocalSidecarLyricsProvider, LrclibProvider,
    LyricsProvider, TrackLyricsQuery,
};
