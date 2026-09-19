use crate::lyrics_online::lrclib::dto::LrclibItemDto;
use crate::lyrics_online::models::{
    LyricsCandidate, LyricsCandidateQuery, LyricsSourceKind, LyricsSyncType,
};
use crate::lyrics_online::traits::OnlineLyricsProvider;
use crate::metadata::error::ProviderError;
use crate::metadata::matching::metrics::{duration_score, jaro_winkler_similarity};
use crate::metadata::matching::normalizer::StringNormalizer;
use crate::metadata::throttle::RateLimiter;
use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, USER_AGENT};
use reqwest::Client;
use sonora_lyrics::model::{LyricsDocument, LyricsFormat};
use sonora_lyrics::parser::{LrcLyricsParser, LyricsParser, PlainTextLyricsParser};
use std::time::Duration;

pub const DEFAULT_LRCLIB_BASE_URL: &str = "https://lrclib.net/api";
pub const DEFAULT_LRCLIB_USER_AGENT: &str =
    "Sonora/0.2.0 ( https://github.com/sonora-audio/sonora )";

#[derive(Clone)]
pub struct LrclibClient {
    client: Client,
    base_url: String,
    rate_limiter: RateLimiter,
}

impl LrclibClient {
    pub fn new() -> Self {
        Self::with_options(
            DEFAULT_LRCLIB_BASE_URL,
            DEFAULT_LRCLIB_USER_AGENT,
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
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            rate_limiter: RateLimiter::new(min_request_interval),
        }
    }

    /// Convert raw text content from a candidate into Sonora's universal LyricsDocument AST.
    pub fn parse_candidate_document(
        candidate: &LyricsCandidate,
    ) -> Result<LyricsDocument, ProviderError> {
        if candidate.is_instrumental {
            return Ok(LyricsDocument {
                title: Some(candidate.track_name.clone()),
                artist: Some(candidate.artist_name.clone()),
                album: candidate.album_name.clone(),
                offset_ms: 0,
                format: LyricsFormat::Plain,
                lines: Vec::new(),
            });
        }

        match candidate.sync_type {
            LyricsSyncType::LineSynced | LyricsSyncType::SyllableSynced => {
                let parser = LrcLyricsParser;
                let mut doc = parser.parse(&candidate.raw_content).map_err(|e| {
                    ProviderError::Parse(format!("Failed to parse synced LRC: {e}"))
                })?;
                if doc.title.is_none() {
                    doc.title = Some(candidate.track_name.clone());
                }
                if doc.artist.is_none() {
                    doc.artist = Some(candidate.artist_name.clone());
                }
                Ok(doc)
            }
            LyricsSyncType::PlainText => {
                let parser = PlainTextLyricsParser;
                let mut doc = parser.parse(&candidate.raw_content).map_err(|e| {
                    ProviderError::Parse(format!("Failed to parse plain lyrics: {e}"))
                })?;
                doc.title = Some(candidate.track_name.clone());
                doc.artist = Some(candidate.artist_name.clone());
                Ok(doc)
            }
        }
    }

    fn item_to_candidate(
        &self,
        item: LrclibItemDto,
        query: &LyricsCandidateQuery,
    ) -> Option<LyricsCandidate> {
        let is_instrumental = item.instrumental.unwrap_or(false);
        let track_name = item.track_name.unwrap_or_else(|| query.track_name.clone());
        let artist_name = item
            .artist_name
            .unwrap_or_else(|| query.artist_name.clone().unwrap_or_default());
        let album_name = item.album_name;
        let cand_duration = item.duration.unwrap_or(0.0);

        let (sync_type, raw_content) = if is_instrumental {
            (LyricsSyncType::PlainText, "[Instrumental]".to_string())
        } else if let Some(synced) = item.synced_lyrics {
            if !synced.trim().is_empty() {
                (LyricsSyncType::LineSynced, synced)
            } else if let Some(plain) = item.plain_lyrics {
                (LyricsSyncType::PlainText, plain)
            } else {
                return None;
            }
        } else if let Some(plain) = item.plain_lyrics {
            if !plain.trim().is_empty() {
                (LyricsSyncType::PlainText, plain)
            } else {
                return None;
            }
        } else {
            return None;
        };

        let duration_delta = if let Some(target_dur) = query.duration_seconds {
            if cand_duration > 0.0 {
                (target_dur - cand_duration).abs()
            } else {
                0.0
            }
        } else {
            0.0
        };

        // Score confidence
        let norm_q_title = StringNormalizer::normalize(&query.track_name);
        let norm_cand_title = StringNormalizer::normalize(&track_name);
        let title_sim = jaro_winkler_similarity(&norm_q_title, &norm_cand_title);
        let mut score = 0.5 * title_sim;

        if let Some(target_artist) = &query.artist_name {
            let norm_q_artist = StringNormalizer::normalize(target_artist);
            let norm_cand_artist = StringNormalizer::normalize(&artist_name);
            let artist_sim = jaro_winkler_similarity(&norm_q_artist, &norm_cand_artist);
            score += 0.3 * artist_sim;
        } else {
            score += 0.3;
        }

        if let Some(target_dur) = query.duration_seconds {
            if cand_duration > 0.0 {
                let dur_factor = duration_score(
                    (target_dur * 1000.0).round() as u64,
                    (cand_duration * 1000.0).round() as u64,
                );
                score += 0.2 * dur_factor;
            } else {
                score += 0.1;
            }
        } else {
            score += 0.2;
        }

        // Syllable / Line synced bonus
        if sync_type == LyricsSyncType::LineSynced {
            score = (score * 1.05).min(1.0);
        }

        Some(LyricsCandidate {
            candidate_id: format!("lrclib_{}", item.id),
            source_kind: LyricsSourceKind::LrclibPublicApi { id: item.id },
            provider_name: "LRCLIB".to_string(),
            sync_type,
            track_name,
            artist_name,
            album_name,
            duration_seconds: cand_duration,
            duration_delta_seconds: duration_delta,
            match_confidence: score.clamp(0.0, 1.0),
            language_code: None,
            is_instrumental,
            raw_content,
        })
    }
}

impl Default for LrclibClient {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl OnlineLyricsProvider for LrclibClient {
    fn name(&self) -> &'static str {
        "lrclib"
    }

    async fn get_lyrics(
        &self,
        query: &LyricsCandidateQuery,
    ) -> Result<Option<LyricsCandidate>, ProviderError> {
        self.rate_limiter.acquire().await;

        let url = format!("{}/get", self.base_url);
        let mut req = self
            .client
            .get(&url)
            .query(&[("track_name", &query.track_name)]);

        if let Some(artist) = &query.artist_name {
            req = req.query(&[("artist_name", artist)]);
        }
        if let Some(album) = &query.album_name {
            req = req.query(&[("album_name", album)]);
        }
        if let Some(duration_sec) = query.duration_seconds {
            let rounded = duration_sec.round() as u64;
            req = req.query(&[("duration", &rounded.to_string())]);
        }

        let resp = match req.send().await {
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
                message: format!("LRCLIB request failed with status {status}"),
            });
        }

        let body = resp
            .text()
            .await
            .map_err(|e| ProviderError::Network(e.to_string()))?;

        let item: LrclibItemDto = serde_json::from_str(&body)
            .map_err(|e| ProviderError::Parse(format!("Failed to parse LRCLIB response: {e}")))?;

        Ok(self.item_to_candidate(item, query))
    }

    async fn search_lyrics(
        &self,
        query: &LyricsCandidateQuery,
    ) -> Result<Vec<LyricsCandidate>, ProviderError> {
        self.rate_limiter.acquire().await;

        let url = format!("{}/search", self.base_url);
        let mut req = self
            .client
            .get(&url)
            .query(&[("track_name", &query.track_name)]);

        if let Some(artist) = &query.artist_name {
            req = req.query(&[("artist_name", artist)]);
        }
        if let Some(album) = &query.album_name {
            req = req.query(&[("album_name", album)]);
        }

        let resp = match req.send().await {
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
            return Ok(Vec::new());
        }

        if status.as_u16() == 429 {
            return Err(ProviderError::RateLimited {
                retry_after_secs: Some(2),
            });
        }

        if !status.is_success() {
            return Err(ProviderError::Http {
                status: status.as_u16(),
                message: format!("LRCLIB search request failed with status {status}"),
            });
        }

        let body = resp
            .text()
            .await
            .map_err(|e| ProviderError::Network(e.to_string()))?;

        let items: Vec<LrclibItemDto> = serde_json::from_str(&body).map_err(|e| {
            ProviderError::Parse(format!("Failed to parse LRCLIB search results: {e}"))
        })?;

        let mut candidates: Vec<LyricsCandidate> = items
            .into_iter()
            .filter_map(|it| self.item_to_candidate(it, query))
            .collect();

        // Sort descending by match confidence
        candidates.sort_by(|a, b| {
            b.match_confidence
                .partial_cmp(&a.match_confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(candidates)
    }
}
