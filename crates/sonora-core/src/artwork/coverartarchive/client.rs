use crate::artwork::coverartarchive::dto::CaaResponse;
use crate::artwork::models::{ArtworkCandidate, ArtworkKind, ArtworkSourceType};
use crate::artwork::traits::ArtworkProvider;
use crate::metadata::error::ProviderError;
use crate::metadata::throttle::RateLimiter;
use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, USER_AGENT};
use reqwest::Client;
use std::time::Duration;

pub const DEFAULT_CAA_BASE_URL: &str = "https://coverartarchive.org";
pub const DEFAULT_CAA_USER_AGENT: &str = "Sonora/0.2.0 ( https://github.com/sonora-audio/sonora )";

#[derive(Clone)]
pub struct CoverArtArchiveClient {
    client: Client,
    base_url: String,
    rate_limiter: RateLimiter,
}

impl CoverArtArchiveClient {
    pub fn new() -> Self {
        Self::with_options(
            DEFAULT_CAA_BASE_URL,
            DEFAULT_CAA_USER_AGENT,
            Duration::from_millis(500),
        )
    }

    pub fn with_options(base_url: &str, user_agent: &str, min_request_interval: Duration) -> Self {
        let mut headers = HeaderMap::new();
        headers.insert(
            USER_AGENT,
            HeaderValue::from_str(user_agent)
                .unwrap_or_else(|_| HeaderValue::from_static("Sonora/0.2.0")),
        );
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));

        let client = Client::builder()
            .default_headers(headers)
            .redirect(reqwest::redirect::Policy::limited(10))
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            rate_limiter: RateLimiter::new(min_request_interval),
        }
    }

    async fn get_json<T: serde::de::DeserializeOwned>(
        &self,
        url: &str,
    ) -> Result<Option<T>, ProviderError> {
        self.rate_limiter.acquire().await;

        let resp = match self.client.get(url).send().await {
            Ok(r) => r,
            Err(e) => {
                if e.is_timeout() {
                    return Err(ProviderError::Timeout);
                }
                return Err(ProviderError::Network(e.to_string()));
            }
        };

        let status = resp.status();
        if status.as_u16() == 404 {
            return Ok(None);
        }

        if status.as_u16() == 429 {
            let retry_after = resp
                .headers()
                .get("Retry-After")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok());
            let delay = retry_after.unwrap_or(2);
            self.rate_limiter
                .delay_until(Duration::from_secs(delay))
                .await;
            return Err(ProviderError::RateLimited {
                retry_after_secs: retry_after,
            });
        }

        if !status.is_success() {
            return Err(ProviderError::Http {
                status: status.as_u16(),
                message: format!("Cover Art Archive request failed with status {status}"),
            });
        }

        let body = resp
            .text()
            .await
            .map_err(|e| ProviderError::Network(e.to_string()))?;

        let parsed: T = serde_json::from_str(&body).map_err(|e| {
            ProviderError::Parse(format!("Failed to parse CAA JSON from {url}: {e}"))
        })?;

        Ok(Some(parsed))
    }
}

impl Default for CoverArtArchiveClient {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ArtworkProvider for CoverArtArchiveClient {
    fn name(&self) -> &'static str {
        "coverartarchive"
    }

    async fn fetch_release_artwork(
        &self,
        release_mbid: &str,
    ) -> Result<Vec<ArtworkCandidate>, ProviderError> {
        let url = format!("{}/release/{}", self.base_url, release_mbid);
        let resp: Option<CaaResponse> = self.get_json(&url).await?;

        let mut candidates = Vec::new();
        if let Some(caa) = resp {
            for img in caa.images {
                let kind = parse_caa_kind(&img.types, img.front, img.back);
                let thumb_url = img
                    .thumbnails
                    .get("500")
                    .or_else(|| img.thumbnails.get("large"))
                    .or_else(|| img.thumbnails.get("250"))
                    .or_else(|| img.thumbnails.get("small"))
                    .cloned()
                    .unwrap_or_else(|| img.image.clone());

                let id = match img.id {
                    serde_json::Value::String(s) => s,
                    serde_json::Value::Number(n) => n.to_string(),
                    _ => format!("img_{}", candidates.len()),
                };

                candidates.push(ArtworkCandidate {
                    id,
                    provider_name: "Cover Art Archive".to_string(),
                    source_type: ArtworkSourceType::CoverArtArchive {
                        release_mbid: Some(release_mbid.to_string()),
                        release_group_mbid: None,
                    },
                    kind,
                    original_url: img.image,
                    preview_thumbnail_url: thumb_url,
                    width: 1200,
                    height: 1200,
                    format: "JPEG".to_string(),
                    size_bytes: None,
                    match_confidence: if img.front { 1.0 } else { 0.85 },
                    is_canonical: img.front,
                });
            }
        }

        Ok(candidates)
    }

    async fn fetch_release_group_artwork(
        &self,
        release_group_mbid: &str,
    ) -> Result<Vec<ArtworkCandidate>, ProviderError> {
        let url = format!("{}/release-group/{}", self.base_url, release_group_mbid);
        let resp: Option<CaaResponse> = self.get_json(&url).await?;

        let mut candidates = Vec::new();
        if let Some(caa) = resp {
            for img in caa.images {
                let kind = parse_caa_kind(&img.types, img.front, img.back);
                let thumb_url = img
                    .thumbnails
                    .get("500")
                    .or_else(|| img.thumbnails.get("large"))
                    .or_else(|| img.thumbnails.get("250"))
                    .or_else(|| img.thumbnails.get("small"))
                    .cloned()
                    .unwrap_or_else(|| img.image.clone());

                let id = match img.id {
                    serde_json::Value::String(s) => s,
                    serde_json::Value::Number(n) => n.to_string(),
                    _ => format!("rg_img_{}", candidates.len()),
                };

                candidates.push(ArtworkCandidate {
                    id,
                    provider_name: "Cover Art Archive".to_string(),
                    source_type: ArtworkSourceType::CoverArtArchive {
                        release_mbid: None,
                        release_group_mbid: Some(release_group_mbid.to_string()),
                    },
                    kind,
                    original_url: img.image,
                    preview_thumbnail_url: thumb_url,
                    width: 1200,
                    height: 1200,
                    format: "JPEG".to_string(),
                    size_bytes: None,
                    match_confidence: if img.front { 0.95 } else { 0.80 },
                    is_canonical: img.front,
                });
            }
        }

        Ok(candidates)
    }

    async fn fetch_artist_artwork(
        &self,
        _artist_mbid: &str,
        _artist_name: &str,
    ) -> Result<Vec<ArtworkCandidate>, ProviderError> {
        // Cover Art Archive is release & release-group artwork authority only
        Ok(Vec::new())
    }
}

fn parse_caa_kind(types: &[String], front: bool, back: bool) -> ArtworkKind {
    if front {
        return ArtworkKind::FrontCover;
    }
    if back {
        return ArtworkKind::BackCover;
    }
    for t in types {
        let lower = t.to_lowercase();
        if lower.contains("front") {
            return ArtworkKind::FrontCover;
        } else if lower.contains("back") {
            return ArtworkKind::BackCover;
        } else if lower.contains("booklet") {
            return ArtworkKind::Booklet;
        } else if lower.contains("medium") || lower.contains("cd") || lower.contains("vinyl") {
            return ArtworkKind::Medium;
        }
    }
    ArtworkKind::Other
}
