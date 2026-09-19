use crate::artwork::models::{ArtworkCandidate, ArtworkQuery, CachedArtworkAsset};
use crate::artwork::pipeline::ArtworkPipeline;
use crate::artwork::traits::ArtworkProvider;
use crate::artwork::{CoverArtArchiveClient, FanartTvArtworkProvider, WikidataArtworkProvider};
use crate::cache::OnlineCacheManager;
use crate::lyrics_online::models::{LyricsCandidate, LyricsCandidateQuery};
use crate::lyrics_online::traits::OnlineLyricsProvider;
use crate::lyrics_online::LrclibClient;
use crate::metadata::error::ProviderError;
use crate::metadata::matching::DeterministicMatcher;
use crate::metadata::models::{LocalTrackMetadata, RankedCandidateMatch};
use crate::metadata::musicbrainz::MusicBrainzClient;
use crate::metadata::traits::MetadataProvider;
use reqwest::Client;
use sonora_library::Database;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Orchestration and routing layer for online enrichment (Metadata, Artwork, Lyrics).
#[derive(Clone)]
pub struct EnrichmentRouter {
    cache: OnlineCacheManager,
    metadata_provider: Arc<dyn MetadataProvider>,
    artwork_providers: Vec<Arc<dyn ArtworkProvider>>,
    lyrics_provider: Arc<dyn OnlineLyricsProvider>,
    artwork_pipeline: ArtworkPipeline,
    in_flight_queries: Arc<Mutex<HashSet<String>>>,
    http_client: Client,
}

impl EnrichmentRouter {
    /// Create a standard router backed by SQLite database.
    pub fn new(db: Arc<Database>, fanart_api_key: Option<String>) -> Self {
        let cache = OnlineCacheManager::new(db);
        let metadata_provider = Arc::new(MusicBrainzClient::new());
        let artwork_providers: Vec<Arc<dyn ArtworkProvider>> = vec![
            Arc::new(CoverArtArchiveClient::new()),
            Arc::new(WikidataArtworkProvider::new()),
            Arc::new(FanartTvArtworkProvider::new(fanart_api_key)),
        ];
        let lyrics_provider = Arc::new(LrclibClient::new());
        let artwork_pipeline = ArtworkPipeline::new(ArtworkPipeline::default_cache_dir());
        let in_flight_queries = Arc::new(Mutex::new(HashSet::new()));
        let http_client = Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            cache,
            metadata_provider,
            artwork_providers,
            lyrics_provider,
            artwork_pipeline,
            in_flight_queries,
            http_client,
        }
    }

    /// Create router with custom providers (primarily for mock testing and custom configs).
    pub fn with_providers(
        db: Arc<Database>,
        metadata_provider: Arc<dyn MetadataProvider>,
        artwork_providers: Vec<Arc<dyn ArtworkProvider>>,
        lyrics_provider: Arc<dyn OnlineLyricsProvider>,
        artwork_pipeline: ArtworkPipeline,
    ) -> Self {
        let cache = OnlineCacheManager::new(db);
        let in_flight_queries = Arc::new(Mutex::new(HashSet::new()));
        let http_client = Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            cache,
            metadata_provider,
            artwork_providers,
            lyrics_provider,
            artwork_pipeline,
            in_flight_queries,
            http_client,
        }
    }

    pub fn cache(&self) -> &OnlineCacheManager {
        &self.cache
    }

    pub fn artwork_pipeline(&self) -> &ArtworkPipeline {
        &self.artwork_pipeline
    }

    // -----------------------------------------------------------------------
    // 1. Metadata Candidates Resolution
    // -----------------------------------------------------------------------

    /// Find and rank metadata candidates for a local track, utilizing cache and MusicBrainz.
    pub async fn find_metadata_candidates(
        &self,
        local: &LocalTrackMetadata,
    ) -> Result<Vec<RankedCandidateMatch>, ProviderError> {
        let fingerprint = OnlineCacheManager::metadata_query_fingerprint(
            &local.title,
            local.artist.as_deref(),
            local.album.as_deref(),
            Some(local.duration_ms),
        );

        // 1. Check fresh cache
        if let Some(cached) = self.cache.get_metadata_candidates(&fingerprint, false) {
            return Ok(cached);
        }

        // 2. In-flight de-duplication
        {
            let mut in_flight = self.in_flight_queries.lock().await;
            if in_flight.contains(&fingerprint) {
                // If query is already in flight, wait briefly and check cache
                drop(in_flight);
                tokio::time::sleep(std::time::Duration::from_millis(300)).await;
                if let Some(cached) = self.cache.get_metadata_candidates(&fingerprint, true) {
                    return Ok(cached);
                }
            } else {
                in_flight.insert(fingerprint.clone());
            }
        }

        // 3. Remote search
        let remote_result = self.search_and_rank_metadata(local).await;

        // Clean in-flight marker
        {
            let mut in_flight = self.in_flight_queries.lock().await;
            in_flight.remove(&fingerprint);
        }

        match remote_result {
            Ok(candidates) => {
                self.cache
                    .set_metadata_candidates(&fingerprint, &candidates, None);
                Ok(candidates)
            }
            Err(e) => {
                // Stale fallback on network error
                if let Some(stale) = self.cache.get_metadata_candidates(&fingerprint, true) {
                    tracing::warn!("Provider failed ({e}); returning stale metadata cache");
                    return Ok(stale);
                }
                Err(e)
            }
        }
    }

    async fn search_and_rank_metadata(
        &self,
        local: &LocalTrackMetadata,
    ) -> Result<Vec<RankedCandidateMatch>, ProviderError> {
        let query = if let Some(artist) = &local.artist {
            format!("{} {}", local.title, artist)
        } else {
            local.title.clone()
        };

        let tracks = self.metadata_provider.search_recordings(&query, 15).await?;

        let candidates = tracks.into_iter().map(|t| (t, None, None)).collect();
        let ranked = DeterministicMatcher::rank_candidates(local, candidates);
        Ok(ranked)
    }

    // -----------------------------------------------------------------------
    // 2. Artwork Candidates Resolution
    // -----------------------------------------------------------------------

    /// Find artwork candidates across Cover Art Archive, Wikidata, and Fanart.tv.
    pub async fn find_artwork_candidates(
        &self,
        query: &ArtworkQuery,
    ) -> Result<Vec<ArtworkCandidate>, ProviderError> {
        let cache_key = format!(
            "art:{}:{}:{}",
            query.release_mbid.as_deref().unwrap_or(""),
            query.release_group_mbid.as_deref().unwrap_or(""),
            query.artist_mbid.as_deref().unwrap_or("")
        );

        // 1. Check fresh cache
        if let Some(cached) = self.cache.get_artwork_candidates(&cache_key, false) {
            return Ok(cached);
        }

        let mut all_candidates = Vec::new();
        let mut any_provider_succeeded = false;
        let mut last_error = None;

        // 2. Query all configured artwork providers
        for provider in &self.artwork_providers {
            // A. Release artwork
            if let Some(rel_mbid) = &query.release_mbid {
                match provider.fetch_release_artwork(rel_mbid).await {
                    Ok(candidates) => {
                        any_provider_succeeded = true;
                        all_candidates.extend(candidates);
                    }
                    Err(e) => last_error = Some(e),
                }
            }

            // B. Release-group artwork
            if let Some(rg_mbid) = &query.release_group_mbid {
                match provider.fetch_release_group_artwork(rg_mbid).await {
                    Ok(candidates) => {
                        any_provider_succeeded = true;
                        all_candidates.extend(candidates);
                    }
                    Err(e) => last_error = Some(e),
                }
            }

            // C. Artist imagery
            if let Some(art_mbid) = &query.artist_mbid {
                let name = query.artist_name.as_deref().unwrap_or("");
                match provider.fetch_artist_artwork(art_mbid, name).await {
                    Ok(candidates) => {
                        any_provider_succeeded = true;
                        all_candidates.extend(candidates);
                    }
                    Err(e) => last_error = Some(e),
                }
            }
        }

        if any_provider_succeeded {
            // Cache successful candidates
            self.cache
                .set_artwork_candidates(&cache_key, "multi", &all_candidates, "router", None);
            Ok(all_candidates)
        } else if let Some(stale) = self.cache.get_artwork_candidates(&cache_key, true) {
            // Stale fallback
            tracing::warn!("Artwork providers failed; returning stale artwork cache");
            Ok(stale)
        } else if let Some(err) = last_error {
            Err(err)
        } else {
            Ok(Vec::new())
        }
    }

    // -----------------------------------------------------------------------
    // 3. Lyrics Candidates Resolution
    // -----------------------------------------------------------------------

    /// Find lyrics candidates from LRCLIB.
    pub async fn find_lyrics_candidates(
        &self,
        query: &LyricsCandidateQuery,
    ) -> Result<Vec<LyricsCandidate>, ProviderError> {
        let fingerprint = OnlineCacheManager::lyrics_query_fingerprint(
            &query.track_name,
            query.artist_name.as_deref(),
            query.album_name.as_deref(),
            query.duration_seconds,
        );

        // 1. Check fresh cache
        if let Some(cached) = self.cache.get_lyrics_candidates(&fingerprint, false) {
            return Ok(cached);
        }

        // 2. Query remote provider
        let remote_result = self.lyrics_provider.search_lyrics(query).await;

        match remote_result {
            Ok(candidates) => {
                self.cache
                    .set_lyrics_candidates(&fingerprint, &candidates, None);
                Ok(candidates)
            }
            Err(e) => {
                if let Some(stale) = self.cache.get_lyrics_candidates(&fingerprint, true) {
                    tracing::warn!("Lyrics provider failed ({e}); returning stale lyrics cache");
                    return Ok(stale);
                }
                Err(e)
            }
        }
    }

    // -----------------------------------------------------------------------
    // 4. Safe Artwork Download & WebP Caching Pipeline
    // -----------------------------------------------------------------------

    /// Downloads a remote candidate image, validates payload and dimensions,
    /// normalizes to WebP textures, and caches locally.
    pub async fn download_and_cache_artwork(
        &self,
        image_url: &str,
    ) -> Result<CachedArtworkAsset, ProviderError> {
        // 1. Check if asset already exists in local disk cache
        if let Some(existing) = self.artwork_pipeline.get_cached_asset(image_url) {
            return Ok(existing);
        }

        // 2. Fetch raw image bytes from remote CDN safely
        let resp = self
            .http_client
            .get(image_url)
            .send()
            .await
            .map_err(|e| ProviderError::Network(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            return Err(ProviderError::Http {
                status: status.as_u16(),
                message: format!("Image download failed with status {status}"),
            });
        }

        let bytes = resp
            .bytes()
            .await
            .map_err(|e| ProviderError::Network(e.to_string()))?;

        // 3. Process through zero-trust sanitization pipeline
        self.artwork_pipeline.process_and_cache(image_url, &bytes)
    }
}
