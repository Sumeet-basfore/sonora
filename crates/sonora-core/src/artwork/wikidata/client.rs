use crate::artwork::models::{ArtworkCandidate, ArtworkKind, ArtworkSourceType};
use crate::artwork::traits::ArtworkProvider;
use crate::metadata::error::ProviderError;
use crate::metadata::throttle::RateLimiter;
use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, USER_AGENT};
use reqwest::Client;
use serde::Deserialize;
use std::time::Duration;

pub const DEFAULT_WIKIDATA_SPARQL_URL: &str = "https://query.wikidata.org/sparql";

#[derive(Debug, Deserialize)]
struct WikidataSparqlResponse {
    results: WikidataResults,
}

#[derive(Debug, Deserialize)]
struct WikidataResults {
    #[serde(default)]
    bindings: Vec<WikidataBinding>,
}

#[derive(Debug, Deserialize)]
struct WikidataBinding {
    image: Option<WikidataValue>,
}

#[derive(Debug, Deserialize)]
struct WikidataValue {
    value: String,
}

#[derive(Clone)]
pub struct WikidataArtworkProvider {
    client: Client,
    endpoint_url: String,
    rate_limiter: RateLimiter,
}

impl WikidataArtworkProvider {
    pub fn new() -> Self {
        Self::with_endpoint(DEFAULT_WIKIDATA_SPARQL_URL)
    }

    pub fn with_endpoint(endpoint: &str) -> Self {
        let mut headers = HeaderMap::new();
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static("Sonora/0.2.0 ( https://github.com/sonora-audio/sonora )"),
        );
        headers.insert(
            ACCEPT,
            HeaderValue::from_static("application/sparql-results+json"),
        );

        let client = Client::builder()
            .default_headers(headers)
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            client,
            endpoint_url: endpoint.to_string(),
            rate_limiter: RateLimiter::new(Duration::from_millis(500)),
        }
    }
}

impl Default for WikidataArtworkProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ArtworkProvider for WikidataArtworkProvider {
    fn name(&self) -> &'static str {
        "wikidata"
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
        self.rate_limiter.acquire().await;

        let sparql_query = format!(
            "SELECT ?item ?image WHERE {{ ?item wdt:P434 \"{}\" . ?item wdt:P18 ?image . }} LIMIT 5",
            artist_mbid
        );

        let req = self
            .client
            .get(&self.endpoint_url)
            .query(&[("query", &sparql_query), ("format", &"json".to_string())]);

        let resp = match req.send().await {
            Ok(r) => r,
            Err(e) => {
                if e.is_timeout() {
                    return Err(ProviderError::Timeout);
                }
                return Err(ProviderError::Network(e.to_string()));
            }
        };

        if resp.status().as_u16() == 429 {
            return Err(ProviderError::RateLimited {
                retry_after_secs: Some(5),
            });
        }

        if !resp.status().is_success() {
            let status = resp.status();
            return Err(ProviderError::Http {
                status: status.as_u16(),
                message: format!("Wikidata SPARQL request failed with status {status}"),
            });
        }

        let body = resp
            .text()
            .await
            .map_err(|e| ProviderError::Network(e.to_string()))?;

        let parsed: WikidataSparqlResponse = serde_json::from_str(&body).map_err(|e| {
            ProviderError::Parse(format!("Failed to parse Wikidata SPARQL JSON: {e}"))
        })?;

        let mut candidates = Vec::new();
        for (i, binding) in parsed.results.bindings.into_iter().enumerate() {
            if let Some(img_val) = binding.image {
                let img_url = img_val.value;
                if !img_url.is_empty() {
                    // Generate Wikimedia thumbnail URL if special:filepath
                    let thumb_url = if img_url.contains("Special:FilePath") {
                        format!("{}?width=500", img_url)
                    } else {
                        img_url.clone()
                    };

                    candidates.push(ArtworkCandidate {
                        id: format!("wd_{}_{}", artist_mbid, i),
                        provider_name: "Wikidata / Wikimedia Commons".to_string(),
                        source_type: ArtworkSourceType::Wikidata {
                            image_url: img_url.clone(),
                        },
                        kind: ArtworkKind::ArtistPortrait,
                        original_url: img_url,
                        preview_thumbnail_url: thumb_url,
                        width: 1000,
                        height: 1000,
                        format: "JPEG".to_string(),
                        size_bytes: None,
                        match_confidence: if i == 0 { 0.90 } else { 0.80 },
                        is_canonical: i == 0,
                    });
                }
            }
        }

        Ok(candidates)
    }
}
