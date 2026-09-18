//! Lyrics service: cached, cascading resolution.
//!
//! Resolution order: SQLite cache → built-in providers (embedded, sidecar,
//! LRCLIB) → running `lyrics:provider` plugins → miss. Every hit is written
//! back to the cache and announced as `LyricsResolved`; a full miss emits
//! `LyricsResolutionFailed`. A plugin trap is isolated by the host and treated
//! as a provider miss, never a fatal error.

use crate::bus::EventBus;
use crate::event::SonoraEvent;
use sonora_common::Result;
use sonora_library::{Database, LibraryRepository};
use sonora_lyrics::model::LyricsDocument;
use sonora_lyrics::parser::{LrcLyricsParser, LyricsParser, PlainTextLyricsParser};
use sonora_lyrics::provider::{CascadingLyricsResolver, TrackLyricsQuery};
use sonora_plugin::{Capability, LyricsQueryDto, PluginHost};
use std::sync::Arc;

#[derive(Clone)]
pub struct LyricsService {
    db: Arc<Database>,
    plugins: Arc<PluginHost>,
    bus: EventBus,
}

impl LyricsService {
    pub fn new(db: Arc<Database>, plugins: Arc<PluginHost>, bus: EventBus) -> Self {
        Self { db, plugins, bus }
    }

    /// Resolve lyrics for a track through the full cascade.
    pub fn get_lyrics(&self, query: TrackLyricsQuery) -> Result<Option<LyricsDocument>> {
        let repo = LibraryRepository::new(&self.db);

        // 1. SQLite cache.
        if let Ok(Some(cached)) = repo.get_cached_lyrics(
            query.track_id,
            query.file_path.as_deref().and_then(|p| p.to_str()),
            &query.title,
            query.artist.as_deref(),
        ) {
            self.bus.publish(SonoraEvent::LyricsResolved {
                title: query.title.clone(),
                provider: "cache".to_string(),
            });
            return Ok(Some(cached));
        }

        // 2. Built-in providers.
        let resolver = CascadingLyricsResolver::default();
        if let Ok(Some((doc, provider_name))) = resolver.resolve(&query) {
            let _ = repo.save_cached_lyrics(
                query.track_id,
                query.file_path.as_deref().and_then(|p| p.to_str()),
                &query.title,
                query.artist.as_deref(),
                &doc,
                provider_name,
            );
            self.bus.publish(SonoraEvent::LyricsResolved {
                title: query.title.clone(),
                provider: provider_name.to_string(),
            });
            return Ok(Some(doc));
        }

        // 3. Third-party WASM providers (capability-gated, crash-isolated).
        let dto = LyricsQueryDto {
            title: query.title.clone(),
            artist: query.artist.clone(),
            album: query.album.clone(),
            duration_ms: query.duration_ms,
        };
        for id in self.plugins.running_with(Capability::LyricsProvider) {
            match self.plugins.fetch_lyrics(&id, &dto) {
                Ok(Some(answer)) => match parse_plugin_answer(&answer, &query) {
                    Ok(doc) => {
                        let provider = format!("plugin:{id}");
                        let _ = repo.save_cached_lyrics(
                            query.track_id,
                            query.file_path.as_deref().and_then(|p| p.to_str()),
                            &query.title,
                            query.artist.as_deref(),
                            &doc,
                            &provider,
                        );
                        self.bus.publish(SonoraEvent::LyricsResolved {
                            title: query.title.clone(),
                            provider,
                        });
                        return Ok(Some(doc));
                    }
                    Err(e) => {
                        tracing::warn!("plugin '{id}' returned unparseable lyrics: {e}");
                        continue;
                    }
                },
                Ok(None) => continue,
                Err(e) => {
                    // Includes isolated crashes: the cascade continues.
                    tracing::warn!("lyrics plugin '{id}' failed (isolated): {e}");
                    continue;
                }
            }
        }

        self.bus.publish(SonoraEvent::LyricsResolutionFailed {
            title: query.title.clone(),
        });
        Ok(None)
    }

    /// Persist a manual timing-offset adjustment to the cache.
    pub fn save_lyrics_offset(
        &self,
        track_id: Option<i64>,
        file_path: Option<&str>,
        offset_ms: i64,
    ) -> Result<()> {
        LibraryRepository::new(&self.db).update_lyrics_offset(track_id, file_path, offset_ms)
    }
}

/// Parse a plugin's raw payload with the host parsers. Unknown formats are
/// rejected (the host validated the enum at the boundary).
fn parse_plugin_answer(
    answer: &sonora_plugin::LyricsResultDto,
    query: &TrackLyricsQuery,
) -> Result<LyricsDocument> {
    let mut doc = match answer.format.as_str() {
        "plain" => PlainTextLyricsParser.parse(&answer.content),
        "lrc" | "enhanced_lrc" => LrcLyricsParser.parse(&answer.content),
        "ttml" => LrcLyricsParser
            .parse(&answer.content)
            .or_else(|_| PlainTextLyricsParser.parse(&answer.content)),
        other => {
            return Err(sonora_common::SonoraError::Lyrics(format!(
                "unsupported plugin lyrics format '{other}'"
            )));
        }
    }?;
    if doc.title.is_none() {
        doc.title = Some(query.title.clone());
    }
    if doc.artist.is_none() {
        doc.artist = query.artist.clone();
    }
    Ok(doc)
}
