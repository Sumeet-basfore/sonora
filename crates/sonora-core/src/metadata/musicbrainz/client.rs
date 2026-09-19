//! MusicBrainz Web Service v2 HTTP Client.
//!
//! Complies with MusicBrainz API policies:
//! - Hard 1.0 req/sec rate limit (`RateLimiter`)
//! - Explicit `User-Agent: Sonora/0.2.0 ( https://github.com/sonora-audio/sonora )`
//! - JSON responses (`fmt=json`)
//! - HTTP 429 and `Retry-After` header backoff
//! - Non-blocking asynchronous design

use crate::metadata::error::ProviderError;
use crate::metadata::models::{OnlineArtist, OnlineRelease, OnlineReleaseGroup, OnlineTrack};
use crate::metadata::musicbrainz::dto::{
    MbArtist, MbArtistSearchResponse, MbRecording, MbRecordingSearchResponse, MbRelease,
    MbReleaseGroup, MbReleaseGroupSearchResponse, MbReleaseSearchResponse,
};
use crate::metadata::throttle::RateLimiter;
use crate::metadata::traits::MetadataProvider;
use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderValue, RETRY_AFTER, USER_AGENT};
use std::time::Duration;
use tracing::{debug, warn};

pub const DEFAULT_MUSICBRAINZ_BASE_URL: &str = "https://musicbrainz.org/ws/2";
pub const DEFAULT_USER_AGENT: &str = "Sonora/0.2.0 ( https://github.com/sonora-audio/sonora )";
pub const DEFAULT_TIMEOUT_SECS: u64 = 10;

/// MusicBrainz Web Service v2 client.
#[derive(Debug, Clone)]
pub struct MusicBrainzClient {
    base_url: String,
    http_client: reqwest::Client,
    rate_limiter: RateLimiter,
}

impl MusicBrainzClient {
    /// Create a standard production MusicBrainz client with 1.0 req/sec rate limiting.
    pub fn new() -> Self {
        Self::with_config(
            DEFAULT_MUSICBRAINZ_BASE_URL.to_string(),
            DEFAULT_USER_AGENT.to_string(),
            Duration::from_secs(DEFAULT_TIMEOUT_SECS),
            RateLimiter::musicbrainz(),
        )
    }

    /// Create a custom configured client (useful for unit tests and private mirrors).
    pub fn with_config(
        base_url: String,
        user_agent: String,
        timeout: Duration,
        rate_limiter: RateLimiter,
    ) -> Self {
        let mut headers = HeaderMap::new();
        if let Ok(val) = HeaderValue::from_str(&user_agent) {
            headers.insert(USER_AGENT, val);
        }

        let http_client = reqwest::Client::builder()
            .default_headers(headers)
            .timeout(timeout)
            .build()
            .unwrap_or_default();

        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            http_client,
            rate_limiter,
        }
    }

    /// Parse `Retry-After` header into seconds.
    fn parse_retry_after(headers: &HeaderMap) -> Option<u64> {
        let header = headers.get(RETRY_AFTER)?;
        let s = header.to_str().ok()?.trim();
        if let Ok(secs) = s.parse::<u64>() {
            Some(secs)
        } else {
            // If date format, default to 5 seconds
            Some(5)
        }
    }

    /// Perform a rate-limited GET request with retry on transient 503 errors.
    async fn get_json<T: serde::de::DeserializeOwned>(
        &self,
        endpoint: &str,
        query_params: &[(&str, &str)],
    ) -> Result<T, ProviderError> {
        let url = format!("{}/{}", self.base_url, endpoint.trim_start_matches('/'));

        let mut attempts = 0;
        let max_attempts = 3;

        loop {
            attempts += 1;
            self.rate_limiter.acquire().await;

            debug!(url = %url, query = ?query_params, attempt = attempts, "MusicBrainz API request");

            let res = self
                .http_client
                .get(&url)
                .query(query_params)
                .query(&[("fmt", "json")])
                .send()
                .await;

            match res {
                Ok(response) => {
                    let status = response.status();
                    if status.is_success() {
                        let bytes = response.bytes().await.map_err(|e| {
                            ProviderError::Network(format!("Failed to read response body: {e}"))
                        })?;
                        let data = serde_json::from_slice::<T>(&bytes).map_err(|e| {
                            ProviderError::Parse(format!(
                                "Failed to parse MusicBrainz JSON: {e}; raw: {}",
                                String::from_utf8_lossy(&bytes)
                            ))
                        })?;
                        return Ok(data);
                    }

                    if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                        let retry_after = Self::parse_retry_after(response.headers());
                        let delay = Duration::from_secs(retry_after.unwrap_or(2));
                        warn!(
                            delay_secs = delay.as_secs(),
                            "MusicBrainz HTTP 429 rate limit received"
                        );
                        self.rate_limiter.delay_until(delay).await;
                        return Err(ProviderError::rate_limited(Some(delay)));
                    }

                    if status == reqwest::StatusCode::NOT_FOUND {
                        return Err(ProviderError::NotFound);
                    }

                    if status == reqwest::StatusCode::SERVICE_UNAVAILABLE && attempts < max_attempts
                    {
                        warn!(
                            attempt = attempts,
                            "MusicBrainz 503 service unavailable; retrying..."
                        );
                        tokio::time::sleep(Duration::from_millis(1000 * attempts as u64)).await;
                        continue;
                    }

                    let err_body = response.text().await.unwrap_or_default();
                    return Err(ProviderError::Http {
                        status: status.as_u16(),
                        message: err_body,
                    });
                }
                Err(err) => {
                    if err.is_timeout() {
                        return Err(ProviderError::Timeout);
                    }
                    if err.is_connect() {
                        return Err(ProviderError::Offline);
                    }
                    if attempts < max_attempts {
                        tokio::time::sleep(Duration::from_millis(500 * attempts as u64)).await;
                        continue;
                    }
                    return Err(ProviderError::Network(err.to_string()));
                }
            }
        }
    }
}

impl Default for MusicBrainzClient {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MetadataProvider for MusicBrainzClient {
    fn provider_id(&self) -> &'static str {
        "musicbrainz"
    }

    async fn search_artists(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<OnlineArtist>, ProviderError> {
        let limit_str = limit.min(100).to_string();
        let query_params = [("query", query), ("limit", &limit_str)];
        let resp: MbArtistSearchResponse = self.get_json("artist", &query_params).await?;
        Ok(resp.artists.into_iter().map(Into::into).collect())
    }

    async fn search_release_groups(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<OnlineReleaseGroup>, ProviderError> {
        let limit_str = limit.min(100).to_string();
        let query_params = [("query", query), ("limit", &limit_str)];
        let resp: MbReleaseGroupSearchResponse =
            self.get_json("release-group", &query_params).await?;
        Ok(resp.release_groups.into_iter().map(Into::into).collect())
    }

    async fn search_releases(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<OnlineRelease>, ProviderError> {
        let limit_str = limit.min(100).to_string();
        let query_params = [("query", query), ("limit", &limit_str)];
        let resp: MbReleaseSearchResponse = self.get_json("release", &query_params).await?;
        Ok(resp.releases.into_iter().map(Into::into).collect())
    }

    async fn search_recordings(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<OnlineTrack>, ProviderError> {
        let limit_str = limit.min(100).to_string();
        let query_params = [("query", query), ("limit", &limit_str)];
        let resp: MbRecordingSearchResponse = self.get_json("recording", &query_params).await?;
        Ok(resp.recordings.into_iter().map(Into::into).collect())
    }

    async fn get_artist_by_mbid(&self, mbid: &str) -> Result<OnlineArtist, ProviderError> {
        let endpoint = format!("artist/{mbid}");
        let query_params = [("inc", "url-rels+release-groups")];
        let resp: MbArtist = self.get_json(&endpoint, &query_params).await?;
        Ok(resp.into())
    }

    async fn get_release_group_by_mbid(
        &self,
        mbid: &str,
    ) -> Result<OnlineReleaseGroup, ProviderError> {
        let endpoint = format!("release-group/{mbid}");
        let query_params = [("inc", "artists+releases")];
        let resp: MbReleaseGroup = self.get_json(&endpoint, &query_params).await?;
        Ok(resp.into())
    }

    async fn get_release_by_mbid(&self, mbid: &str) -> Result<OnlineRelease, ProviderError> {
        let endpoint = format!("release/{mbid}");
        let query_params = [("inc", "artists+recordings+release-groups+media+labels")];
        let resp: MbRelease = self.get_json(&endpoint, &query_params).await?;
        Ok(resp.into())
    }

    async fn get_recording_by_mbid(&self, mbid: &str) -> Result<OnlineTrack, ProviderError> {
        let endpoint = format!("recording/{mbid}");
        let query_params = [("inc", "artists+releases+isrcs")];
        let resp: MbRecording = self.get_json(&endpoint, &query_params).await?;
        Ok(resp.into())
    }
}
