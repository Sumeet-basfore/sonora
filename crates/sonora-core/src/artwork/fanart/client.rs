use crate::artwork::models::{ArtworkCandidate, ArtworkKind, ArtworkSourceType};
use crate::artwork::traits::ArtworkProvider;
use crate::metadata::error::ProviderError;
use crate::metadata::throttle::RateLimiter;
use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, USER_AGENT};
use reqwest::Client;
use serde::Deserialize;
use std::time::Duration;

pub const DEFAULT_FANART_BASE_URL: &str = "https://webservice.fanart.tv/v3/music";

#[derive(Debug, Deserialize)]
struct FanartMusicResponse {
    #[serde(default)]
    artistthumb: Vec<FanartImageItem>,
    #[serde(default)]
    artistbackground: Vec<FanartImageItem>,
    #[serde(default)]
    hdmusiclogo: Vec<FanartImageItem>,
    #[serde(default)]
    musicbanner: Vec<FanartImageItem>,
}

#[derive(Debug, Deserialize)]
struct FanartImageItem {
    pub id: String,
    pub url: String,
    pub _likes: Option<String>,
}

#[derive(Clone)]
pub struct FanartTvArtworkProvider {
    client: Client,
    base_url: String,
    api_key: Option<String>,
    rate_limiter: RateLimiter,
}

impl FanartTvArtworkProvider {
    pub fn new(api_key: Option<String>) -> Self {
        Self::with_options(DEFAULT_FANART_BASE_URL, api_key, Duration::from_millis(500))
    }

    pub fn with_options(
        base_url: &str,
        api_key: Option<String>,
        min_request_interval: Duration,
    ) -> Self {
        let mut headers = HeaderMap::new();
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static("Sonora/0.2.0 ( https://github.com/sonora-audio/sonora )"),
        );
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));

        let client = Client::builder()
            .default_headers(headers)
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key,
            rate_limiter: RateLimiter::new(min_request_interval),
        }
    }
}

#[async_trait]
impl ArtworkProvider for FanartTvArtworkProvider {
    fn name(&self) -> &'static str {
        "fanart_tv"
    }

    async fn fetch_release_artwork(
        &self,
        _release_mbid: &str,
    ) -> Result<Vec<ArtworkCandidate>, ProviderError> {
        Ok(Vec::new())
    }

    async fn fetch_release_group_artwork(
        &self,
        _release_group_mbid: &str,
    ) -> Result<Vec<ArtworkCandidate>, ProviderError> {
        Ok(Vec::new())
    }

    async fn fetch_artist_artwork(
        &self,
        artist_mbid: &str,
        _artist_name: &str,
    ) -> Result<Vec<ArtworkCandidate>, ProviderError> {
        let key = match &self.api_key {
            Some(k) if !k.trim().is_empty() => k,
            _ => {
                // Fanart.tv requires an API key. Graceful no-op when unconfigured.
                return Ok(Vec::new());
            }
        };

        self.rate_limiter.acquire().await;

        let url = format!("{}/{}?api_key={}", self.base_url, artist_mbid, key);
        let resp = match self.client.get(&url).send().await {
            Ok(r) => r,
            Err(e) => {
                if e.is_timeout() {
                    return Err(ProviderError::Timeout);
                }
                return Err(ProviderError::Network(e.to_string()));
            }
        };

        let status = resp.status();
        if status.as_u16() == 404 || status.as_u16() == 401 || status.as_u16() == 403 {
            // Not found or invalid key -> degrade gracefully to empty candidates
            return Ok(Vec::new());
        }

        if status.as_u16() == 429 {
            return Err(ProviderError::RateLimited {
                retry_after_secs: Some(5),
            });
        }

        if !status.is_success() {
            return Err(ProviderError::Http {
                status: status.as_u16(),
                message: format!("Fanart.tv request failed with status {status}"),
            });
        }

        let body = resp
            .text()
            .await
            .map_err(|e| ProviderError::Network(e.to_string()))?;

        let parsed: FanartMusicResponse = serde_json::from_str(&body)
            .map_err(|e| ProviderError::Parse(format!("Failed to parse Fanart.tv JSON: {e}")))?;

        let mut candidates = Vec::new();

        // 1. Artist Thumbnails (Portraits)
        for (i, item) in parsed.artistthumb.into_iter().enumerate() {
            candidates.push(ArtworkCandidate {
                id: format!("fanart_thumb_{}", item.id),
                provider_name: "Fanart.tv".to_string(),
                source_type: ArtworkSourceType::FanartTv {
                    artist_mbid: artist_mbid.to_string(),
                },
                kind: ArtworkKind::ArtistPortrait,
                original_url: item.url.clone(),
                preview_thumbnail_url: item.url,
                width: 1000,
                height: 1000,
                format: "JPEG".to_string(),
                size_bytes: None,
                match_confidence: if i == 0 { 0.95 } else { 0.85 },
                is_canonical: i == 0,
            });
        }

        // 2. Artist Backgrounds
        for (i, item) in parsed.artistbackground.into_iter().enumerate() {
            candidates.push(ArtworkCandidate {
                id: format!("fanart_bg_{}", item.id),
                provider_name: "Fanart.tv".to_string(),
                source_type: ArtworkSourceType::FanartTv {
                    artist_mbid: artist_mbid.to_string(),
                },
                kind: ArtworkKind::ArtistBackground,
                original_url: item.url.clone(),
                preview_thumbnail_url: item.url,
                width: 1920,
                height: 1080,
                format: "JPEG".to_string(),
                size_bytes: None,
                match_confidence: if i == 0 { 0.90 } else { 0.80 },
                is_canonical: i == 0,
            });
        }

        // 3. HD Logos
        for (i, item) in parsed.hdmusiclogo.into_iter().enumerate() {
            candidates.push(ArtworkCandidate {
                id: format!("fanart_logo_{}", item.id),
                provider_name: "Fanart.tv".to_string(),
                source_type: ArtworkSourceType::FanartTv {
                    artist_mbid: artist_mbid.to_string(),
                },
                kind: ArtworkKind::ArtistLogo,
                original_url: item.url.clone(),
                preview_thumbnail_url: item.url,
                width: 800,
                height: 310,
                format: "PNG".to_string(),
                size_bytes: None,
                match_confidence: if i == 0 { 0.85 } else { 0.75 },
                is_canonical: i == 0,
            });
        }

        // 4. Banners
        for (i, item) in parsed.musicbanner.into_iter().enumerate() {
            candidates.push(ArtworkCandidate {
                id: format!("fanart_banner_{}", item.id),
                provider_name: "Fanart.tv".to_string(),
                source_type: ArtworkSourceType::FanartTv {
                    artist_mbid: artist_mbid.to_string(),
                },
                kind: ArtworkKind::ArtistBanner,
                original_url: item.url.clone(),
                preview_thumbnail_url: item.url,
                width: 1000,
                height: 185,
                format: "JPEG".to_string(),
                size_bytes: None,
                match_confidence: if i == 0 { 0.80 } else { 0.70 },
                is_canonical: i == 0,
            });
        }

        Ok(candidates)
    }
}
