use crate::lyrics_online::models::{LyricsCandidate, LyricsCandidateQuery};
use crate::metadata::error::ProviderError;
use async_trait::async_trait;

/// Asynchronous lyrics provider interface for multi-candidate search and resolution.
#[async_trait]
pub trait OnlineLyricsProvider: Send + Sync {
    /// Human-readable provider name.
    fn name(&self) -> &'static str;

    /// Direct track lookup by exact track metadata.
    async fn get_lyrics(
        &self,
        query: &LyricsCandidateQuery,
    ) -> Result<Option<LyricsCandidate>, ProviderError>;

    /// Multi-candidate search across query terms.
    async fn search_lyrics(
        &self,
        query: &LyricsCandidateQuery,
    ) -> Result<Vec<LyricsCandidate>, ProviderError>;
}
