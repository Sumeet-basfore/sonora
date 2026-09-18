pub mod model;
pub mod parser;
pub mod provider;

pub use model::{LyricLine, LyricSyllable, LyricsDocument, LyricsFormat};
pub use parser::{LyricsParser, PlainTextLyricsParser};
pub use provider::LyricsProvider;
