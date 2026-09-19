use crate::artwork::models::ArtworkCandidate;
use crate::lyrics_online::models::LyricsCandidate;
use crate::metadata::matching::normalizer::StringNormalizer;
use crate::metadata::models::{
    OnlineArtist, OnlineRelease, OnlineReleaseGroup, OnlineTrack, RankedCandidateMatch,
};
use sha2::{Digest, Sha256};
use sonora_library::{Database, LibraryRepository};
use std::sync::Arc;

pub const DEFAULT_METADATA_TTL_SECS: u64 = 14 * 24 * 3600; // 14 days
pub const DEFAULT_ARTWORK_TTL_SECS: u64 = 30 * 24 * 3600; // 30 days
pub const DEFAULT_LYRICS_TTL_SECS: u64 = 14 * 24 * 3600; // 14 days

/// Thread-safe SQLite Persistent Online Enrichment Cache Manager.
#[derive(Clone)]
pub struct OnlineCacheManager {
    db: Arc<Database>,
}

impl OnlineCacheManager {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// Helper to generate deterministic SHA256 hex string.
    pub fn hash_key(input: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(input.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Compute deterministic metadata query fingerprint.
    pub fn metadata_query_fingerprint(
        title: &str,
        artist: Option<&str>,
        album: Option<&str>,
        duration_ms: Option<u64>,
    ) -> String {
        let norm_title = StringNormalizer::normalize(title);
        let norm_artist = artist.map(StringNormalizer::normalize).unwrap_or_default();
        let norm_album = album.map(StringNormalizer::normalize).unwrap_or_default();
        let dur_bucket = duration_ms.map(|d| d / 1000).unwrap_or(0); // 1-sec bucket
        let raw = format!("{norm_title}|{norm_artist}|{norm_album}|{dur_bucket}");
        format!("meta_{}", Self::hash_key(&raw))
    }

    /// Compute deterministic lyrics query fingerprint.
    pub fn lyrics_query_fingerprint(
        title: &str,
        artist: Option<&str>,
        album: Option<&str>,
        duration_seconds: Option<f64>,
    ) -> String {
        let norm_title = StringNormalizer::normalize(title);
        let norm_artist = artist.map(StringNormalizer::normalize).unwrap_or_default();
        let norm_album = album.map(StringNormalizer::normalize).unwrap_or_default();
        let dur_bucket = duration_seconds.map(|d| d.round() as u64).unwrap_or(0);
        let raw = format!("{norm_title}|{norm_artist}|{norm_album}|{dur_bucket}");
        format!("lyrics_{}", Self::hash_key(&raw))
    }

    // -----------------------------------------------------------------------
    // Metadata Entity Cache
    // -----------------------------------------------------------------------

    pub fn get_artist_by_mbid(&self, mbid: &str, allow_stale: bool) -> Option<OnlineArtist> {
        let repo = LibraryRepository::new(&self.db);
        let key = format!("mb:artist:{mbid}");
        let raw = repo.get_online_metadata_cache(&key, allow_stale).ok()??;
        serde_json::from_str(&raw).ok()
    }

    pub fn set_artist(&self, artist: &OnlineArtist, ttl_secs: Option<u64>) {
        let repo = LibraryRepository::new(&self.db);
        let key = format!("mb:artist:{}", artist.mbid);
        if let Ok(payload) = serde_json::to_string(artist) {
            let _ = repo.set_online_metadata_cache(
                &key,
                "artist",
                &payload,
                "musicbrainz",
                ttl_secs.unwrap_or(DEFAULT_METADATA_TTL_SECS),
            );
        }
    }

    pub fn get_release_group_by_mbid(
        &self,
        mbid: &str,
        allow_stale: bool,
    ) -> Option<OnlineReleaseGroup> {
        let repo = LibraryRepository::new(&self.db);
        let key = format!("mb:release_group:{mbid}");
        let raw = repo.get_online_metadata_cache(&key, allow_stale).ok()??;
        serde_json::from_str(&raw).ok()
    }

    pub fn set_release_group(&self, rg: &OnlineReleaseGroup, ttl_secs: Option<u64>) {
        let repo = LibraryRepository::new(&self.db);
        let key = format!("mb:release_group:{}", rg.mbid);
        if let Ok(payload) = serde_json::to_string(rg) {
            let _ = repo.set_online_metadata_cache(
                &key,
                "release_group",
                &payload,
                "musicbrainz",
                ttl_secs.unwrap_or(DEFAULT_METADATA_TTL_SECS),
            );
        }
    }

    pub fn get_release_by_mbid(&self, mbid: &str, allow_stale: bool) -> Option<OnlineRelease> {
        let repo = LibraryRepository::new(&self.db);
        let key = format!("mb:release:{mbid}");
        let raw = repo.get_online_metadata_cache(&key, allow_stale).ok()??;
        serde_json::from_str(&raw).ok()
    }

    pub fn set_release(&self, release: &OnlineRelease, ttl_secs: Option<u64>) {
        let repo = LibraryRepository::new(&self.db);
        let key = format!("mb:release:{}", release.mbid);
        if let Ok(payload) = serde_json::to_string(release) {
            let _ = repo.set_online_metadata_cache(
                &key,
                "release",
                &payload,
                "musicbrainz",
                ttl_secs.unwrap_or(DEFAULT_METADATA_TTL_SECS),
            );
        }
    }

    pub fn get_recording_by_mbid(&self, mbid: &str, allow_stale: bool) -> Option<OnlineTrack> {
        let repo = LibraryRepository::new(&self.db);
        let key = format!("mb:recording:{mbid}");
        let raw = repo.get_online_metadata_cache(&key, allow_stale).ok()??;
        serde_json::from_str(&raw).ok()
    }

    pub fn set_recording(&self, track: &OnlineTrack, ttl_secs: Option<u64>) {
        let repo = LibraryRepository::new(&self.db);
        let key = format!("mb:recording:{}", track.recording_mbid);
        if let Ok(payload) = serde_json::to_string(track) {
            let _ = repo.set_online_metadata_cache(
                &key,
                "recording",
                &payload,
                "musicbrainz",
                ttl_secs.unwrap_or(DEFAULT_METADATA_TTL_SECS),
            );
        }
    }

    // -----------------------------------------------------------------------
    // Ranked Metadata Candidates Cache
    // -----------------------------------------------------------------------

    pub fn get_metadata_candidates(
        &self,
        fingerprint: &str,
        allow_stale: bool,
    ) -> Option<Vec<RankedCandidateMatch>> {
        let repo = LibraryRepository::new(&self.db);
        let raw = repo
            .get_metadata_candidates_cache(fingerprint, allow_stale)
            .ok()??;
        serde_json::from_str(&raw).ok()
    }

    pub fn set_metadata_candidates(
        &self,
        fingerprint: &str,
        candidates: &[RankedCandidateMatch],
        ttl_secs: Option<u64>,
    ) {
        let repo = LibraryRepository::new(&self.db);
        if let Ok(payload) = serde_json::to_string(candidates) {
            let _ = repo.set_metadata_candidates_cache(
                fingerprint,
                &payload,
                ttl_secs.unwrap_or(DEFAULT_METADATA_TTL_SECS),
            );
        }
    }

    // -----------------------------------------------------------------------
    // Artwork Candidates Cache
    // -----------------------------------------------------------------------

    pub fn get_artwork_candidates(
        &self,
        cache_key: &str,
        allow_stale: bool,
    ) -> Option<Vec<ArtworkCandidate>> {
        let repo = LibraryRepository::new(&self.db);
        let raw = repo
            .get_online_artwork_cache(cache_key, allow_stale)
            .ok()??;
        serde_json::from_str(&raw).ok()
    }

    pub fn set_artwork_candidates(
        &self,
        cache_key: &str,
        entity_type: &str,
        candidates: &[ArtworkCandidate],
        provider: &str,
        ttl_secs: Option<u64>,
    ) {
        let repo = LibraryRepository::new(&self.db);
        if let Ok(payload) = serde_json::to_string(candidates) {
            let _ = repo.set_online_artwork_cache(
                cache_key,
                entity_type,
                &payload,
                provider,
                ttl_secs.unwrap_or(DEFAULT_ARTWORK_TTL_SECS),
            );
        }
    }

    // -----------------------------------------------------------------------
    // Lyrics Candidates Cache
    // -----------------------------------------------------------------------

    pub fn get_lyrics_candidates(
        &self,
        fingerprint: &str,
        allow_stale: bool,
    ) -> Option<Vec<LyricsCandidate>> {
        let repo = LibraryRepository::new(&self.db);
        let raw = repo
            .get_lyrics_candidates_cache(fingerprint, allow_stale)
            .ok()??;
        serde_json::from_str(&raw).ok()
    }

    pub fn set_lyrics_candidates(
        &self,
        fingerprint: &str,
        candidates: &[LyricsCandidate],
        ttl_secs: Option<u64>,
    ) {
        let repo = LibraryRepository::new(&self.db);
        if let Ok(payload) = serde_json::to_string(candidates) {
            let _ = repo.set_lyrics_candidates_cache(
                fingerprint,
                &payload,
                ttl_secs.unwrap_or(DEFAULT_LYRICS_TTL_SECS),
            );
        }
    }

    // -----------------------------------------------------------------------
    // Expiration Management
    // -----------------------------------------------------------------------

    pub fn purge_expired(&self) -> usize {
        let repo = LibraryRepository::new(&self.db);
        repo.purge_expired_online_cache().unwrap_or(0)
    }
}
