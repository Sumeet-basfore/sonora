use crate::model::LyricsDocument;
use crate::parser::{LrcLyricsParser, LyricsParser, PlainTextLyricsParser};
use lofty::file::TaggedFileExt;
use lofty::probe::Probe;
use lofty::tag::ItemKey;
use serde::Deserialize;
use sonora_common::Result;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

/// Unified query parameters for lyrics resolution.
#[derive(Debug, Clone, Default)]
pub struct TrackLyricsQuery {
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub duration_ms: Option<u64>,
    pub file_path: Option<PathBuf>,
    pub track_id: Option<i64>,
}

/// Trait implemented by lyrics providers in the cascading resolver hierarchy.
pub trait LyricsProvider: Send + Sync {
    fn name(&self) -> &'static str;
    fn fetch(&self, query: &TrackLyricsQuery) -> Result<Option<LyricsDocument>>;
}

// ---------------------------------------------------------------------------
// 1. Embedded Lyrics Provider (ID3 USLT/SYLT, Vorbis, MP4 tags)
// ---------------------------------------------------------------------------

pub struct EmbeddedLyricsProvider;

impl LyricsProvider for EmbeddedLyricsProvider {
    fn name(&self) -> &'static str {
        "embedded"
    }

    fn fetch(&self, query: &TrackLyricsQuery) -> Result<Option<LyricsDocument>> {
        let path = match &query.file_path {
            Some(p) if p.exists() => p,
            _ => return Ok(None),
        };

        let tagged_file = match Probe::open(path).and_then(|pr| pr.read()) {
            Ok(tf) => tf,
            Err(e) => {
                tracing::debug!("Failed to probe audio tags for embedded lyrics: {e}");
                return Ok(None);
            }
        };

        let tag = tagged_file
            .primary_tag()
            .or_else(|| tagged_file.first_tag());
        let lyrics_text = tag.and_then(|t| {
            if let Some(text) = t.get_string(ItemKey::Lyrics) {
                return Some(text.to_string());
            }
            for item in t.items() {
                if let Some(text) = item.value().text() {
                    let key_str = format!("{:?}", item.key()).to_lowercase();
                    if key_str.contains("lyric")
                        || key_str.contains("uslt")
                        || key_str.contains("sylt")
                    {
                        return Some(text.to_string());
                    }
                }
            }
            None
        });

        if let Some(raw) = lyrics_text {
            if !raw.trim().is_empty() {
                let parser = LrcLyricsParser;
                if let Ok(mut doc) = parser.parse(&raw) {
                    if doc.title.is_none() {
                        doc.title = Some(query.title.clone());
                    }
                    if doc.artist.is_none() {
                        doc.artist = query.artist.clone();
                    }
                    return Ok(Some(doc));
                }
            }
        }

        Ok(None)
    }
}

// ---------------------------------------------------------------------------
// 2. Local Sidecar .lrc Provider
// ---------------------------------------------------------------------------

pub struct LocalSidecarLyricsProvider;

impl LyricsProvider for LocalSidecarLyricsProvider {
    fn name(&self) -> &'static str {
        "local_lrc"
    }

    fn fetch(&self, query: &TrackLyricsQuery) -> Result<Option<LyricsDocument>> {
        let path = match &query.file_path {
            Some(p) if p.exists() => p,
            _ => return Ok(None),
        };

        let candidates = find_local_sidecar_paths(path);
        for candidate in candidates {
            if candidate.exists() && candidate.is_file() {
                if let Ok(content) = std::fs::read_to_string(&candidate) {
                    let parser = LrcLyricsParser;
                    if let Ok(mut doc) = parser.parse(&content) {
                        if doc.title.is_none() {
                            doc.title = Some(query.title.clone());
                        }
                        if doc.artist.is_none() {
                            doc.artist = query.artist.clone();
                        }
                        return Ok(Some(doc));
                    }
                }
            }
        }

        Ok(None)
    }
}

fn find_local_sidecar_paths(audio_path: &Path) -> Vec<PathBuf> {
    let mut list = Vec::new();
    // 1. same path with .lrc extension
    list.push(audio_path.with_extension("lrc"));
    // 2. same path with .synced.lrc extension
    list.push(audio_path.with_extension("synced.lrc"));

    if let (Some(parent), Some(stem)) = (audio_path.parent(), audio_path.file_stem()) {
        let stem_str = stem.to_string_lossy();
        list.push(parent.join(format!("{stem_str}.lrc")));
        list.push(parent.join(format!("{stem_str}.synced.lrc")));
    }

    list
}

// ---------------------------------------------------------------------------
// 3. LRCLIB Public API Provider
// ---------------------------------------------------------------------------

pub struct LrclibProvider {
    client: reqwest::blocking::Client,
}

impl Default for LrclibProvider {
    fn default() -> Self {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_millis(2500))
            .user_agent("Sonora/0.1 (https://github.com/sonora-audio/sonora)")
            .build()
            .unwrap_or_else(|_| reqwest::blocking::Client::new());

        Self { client }
    }
}

#[derive(Deserialize, Debug)]
struct LrclibResponse {
    #[serde(rename = "trackName")]
    _track_name: Option<String>,
    #[serde(rename = "artistName")]
    _artist_name: Option<String>,
    #[serde(rename = "syncedLyrics")]
    synced_lyrics: Option<String>,
    #[serde(rename = "plainLyrics")]
    plain_lyrics: Option<String>,
    instrumental: Option<bool>,
}

impl LyricsProvider for LrclibProvider {
    fn name(&self) -> &'static str {
        "lrclib"
    }

    fn fetch(&self, query: &TrackLyricsQuery) -> Result<Option<LyricsDocument>> {
        let mut req = self
            .client
            .get("https://lrclib.net/api/get")
            .query(&[("track_name", &query.title)]);

        if let Some(artist) = &query.artist {
            req = req.query(&[("artist_name", artist)]);
        }
        if let Some(album) = &query.album {
            req = req.query(&[("album_name", album)]);
        }
        if let Some(duration_ms) = query.duration_ms {
            let dur_sec = duration_ms / 1000;
            req = req.query(&[("duration", dur_sec.to_string())]);
        }

        let resp = match req.send() {
            Ok(r) => r,
            Err(e) => {
                tracing::debug!("LRCLIB request failed: {e}");
                return Ok(None);
            }
        };

        if resp.status().as_u16() == 404 {
            return Ok(None);
        }

        if !resp.status().is_success() {
            tracing::debug!("LRCLIB returned status {}", resp.status());
            return Ok(None);
        }

        let body: LrclibResponse = match resp.json() {
            Ok(b) => b,
            Err(e) => {
                tracing::debug!("Failed to parse LRCLIB response: {e}");
                return Ok(None);
            }
        };

        if body.instrumental == Some(true) {
            return Ok(Some(LyricsDocument {
                title: Some(query.title.clone()),
                artist: query.artist.clone(),
                album: query.album.clone(),
                offset_ms: 0,
                format: crate::model::LyricsFormat::Plain,
                lines: Vec::new(),
            }));
        }

        if let Some(synced) = body.synced_lyrics {
            if !synced.trim().is_empty() {
                let parser = LrcLyricsParser;
                if let Ok(mut doc) = parser.parse(&synced) {
                    if doc.title.is_none() {
                        doc.title = Some(query.title.clone());
                    }
                    if doc.artist.is_none() {
                        doc.artist = query.artist.clone();
                    }
                    return Ok(Some(doc));
                }
            }
        }

        if let Some(plain) = body.plain_lyrics {
            if !plain.trim().is_empty() {
                let parser = PlainTextLyricsParser;
                if let Ok(mut doc) = parser.parse(&plain) {
                    doc.title = Some(query.title.clone());
                    doc.artist = query.artist.clone();
                    return Ok(Some(doc));
                }
            }
        }

        Ok(None)
    }
}

// ---------------------------------------------------------------------------
// 4. Cascading Lyrics Resolver
// ---------------------------------------------------------------------------

pub struct CascadingLyricsResolver {
    providers: Vec<Arc<dyn LyricsProvider>>,
}

impl Default for CascadingLyricsResolver {
    fn default() -> Self {
        Self {
            providers: vec![
                Arc::new(EmbeddedLyricsProvider),
                Arc::new(LocalSidecarLyricsProvider),
                Arc::new(LrclibProvider::default()),
            ],
        }
    }
}

impl CascadingLyricsResolver {
    pub fn new(providers: Vec<Arc<dyn LyricsProvider>>) -> Self {
        Self { providers }
    }

    /// Resolves lyrics by querying providers sequentially until a hit is found.
    /// Returns the resolved document and the provider name that satisfied the query.
    pub fn resolve(
        &self,
        query: &TrackLyricsQuery,
    ) -> Result<Option<(LyricsDocument, &'static str)>> {
        for provider in &self.providers {
            match provider.fetch(query) {
                Ok(Some(doc)) => {
                    tracing::info!(
                        "Lyrics resolved for \"{}\" via provider [{}]",
                        query.title,
                        provider.name()
                    );
                    return Ok(Some((doc, provider.name())));
                }
                Ok(None) => continue,
                Err(e) => {
                    tracing::warn!("Provider [{}] error: {e}", provider.name());
                    continue;
                }
            }
        }

        Ok(None)
    }
}
