//! Provider interfaces for metadata resolution.

use crate::metadata::error::ProviderError;
use crate::metadata::models::{OnlineArtist, OnlineRelease, OnlineReleaseGroup, OnlineTrack};
use async_trait::async_trait;

/// Asynchronous metadata provider interface.
///
/// Providers isolate HTTP/network transport logic and deserialize vendor APIs
/// into Sonora's normalized `OnlineArtist`, `OnlineReleaseGroup`, `OnlineRelease`,
/// and `OnlineTrack` domain structures.
#[async_trait]
pub trait MetadataProvider: Send + Sync {
    /// Unique identifier for this provider (e.g. "musicbrainz").
    fn provider_id(&self) -> &'static str;

    /// Search for artists matching the given text query.
    async fn search_artists(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<OnlineArtist>, ProviderError>;

    /// Search for release groups (albums) matching the given text query.
    async fn search_release_groups(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<OnlineReleaseGroup>, ProviderError>;

    /// Search for releases (specific pressings/editions) matching the given text query.
    async fn search_releases(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<OnlineRelease>, ProviderError>;

    /// Search for tracks / recordings matching the given text query.
    async fn search_recordings(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<OnlineTrack>, ProviderError>;

    /// Lookup a single artist by their MusicBrainz Identifier (MBID).
    async fn get_artist_by_mbid(&self, mbid: &str) -> Result<OnlineArtist, ProviderError>;

    /// Lookup a release group by its MBID.
    async fn get_release_group_by_mbid(
        &self,
        mbid: &str,
    ) -> Result<OnlineReleaseGroup, ProviderError>;

    /// Lookup a specific release by its MBID with tracklist and media.
    async fn get_release_by_mbid(&self, mbid: &str) -> Result<OnlineRelease, ProviderError>;

    /// Lookup a recording by its MBID.
    async fn get_recording_by_mbid(&self, mbid: &str) -> Result<OnlineTrack, ProviderError>;
}
