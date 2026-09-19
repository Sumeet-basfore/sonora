pub mod lrclib;
pub mod models;
pub mod traits;

pub use lrclib::LrclibClient;
pub use models::{LyricsCandidate, LyricsCandidateQuery, LyricsSourceKind, LyricsSyncType};
pub use traits::OnlineLyricsProvider;

#[cfg(test)]
mod tests;
