use crate::artwork::models::ArtworkCandidate;
use crate::metadata::error::ProviderError;
use async_trait::async_trait;

/// Trait for online artwork providers (Cover Art Archive, Wikidata, Fanart.tv, etc.).
#[async_trait]
pub trait ArtworkProvider: Send + Sync {
    /// Human-readable provider name.
    fn name(&self) -> &'static str;

    /// Fetch cover artwork candidates for a specific MusicBrainz Release MBID.
    async fn fetch_release_artwork(
        &self,
        release_mbid: &str,
    ) -> Result<Vec<ArtworkCandidate>, ProviderError>;

    /// Fetch cover artwork candidates for a MusicBrainz Release Group MBID.
    async fn fetch_release_group_artwork(
        &self,
        release_group_mbid: &str,
    ) -> Result<Vec<ArtworkCandidate>, ProviderError>;

    /// Fetch artist imagery (portraits, backgrounds, logos) for a MusicBrainz Artist MBID.
    async fn fetch_artist_artwork(
        &self,
        artist_mbid: &str,
        artist_name: &str,
    ) -> Result<Vec<ArtworkCandidate>, ProviderError>;
}
