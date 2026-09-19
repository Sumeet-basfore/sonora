//! Application facade: orchestration over subsystem services.
//!
//! [`SonoraApp`] owns no subsystem state directly. It composes
//! [`LibraryService`], [`PlaybackService`], [`QueueService`],
//! [`LyricsService`], and [`PluginHost`], all sharing one [`EventBus`].
//! Cross-cutting flows ("play a library track") are orchestrated here using
//! only the services' public APIs; subsystem-to-subsystem communication
//! happens through [`SonoraEvent`]s on the bus, never through shared fields.
//!
//! Mutations can also enter through [`SonoraCommand`] via [`SonoraApp::handle_command`].

use crate::bus::EventBus;
use crate::command::SonoraCommand;
use crate::config::SonoraConfig;
use crate::event::SonoraEvent;
use crate::queue::QueueItem;
use crate::services::{LibraryService, LyricsService, PlaybackService, QueueService};
use crate::PlaybackQueue;
use serde::{Deserialize, Serialize};
use sonora_audio::AudioPlayer;
use sonora_common::{PlaybackState, Result, SonoraError, TrackId};
use sonora_library::{
    AlbumDto, ArtistDto, ArtworkCache, Database, LibraryRepository, LibrarySummary, ScanStats,
    SearchResult,
};
use sonora_plugin::{PluginHost, PluginManifest};
use sonora_registry::Marketplace;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;

/// Consolidated status of the Sonora player and queue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybackStatus {
    pub state: PlaybackState,
    pub current_track: Option<QueueItem>,
    pub position_ms: u64,
    pub duration_ms: u64,
    pub volume: f32,
    pub queue_length: usize,
    pub current_queue_index: Option<usize>,
}

/// Primary application context: service composition + command routing.
pub struct SonoraApp {
    config: SonoraConfig,
    db: Arc<Database>,
    bus: EventBus,
    library: LibraryService,
    playback: PlaybackService,
    queue: QueueService,
    lyrics: LyricsService,
    enrichment: crate::enrichment::EnrichmentRouter,
    plugins: Arc<PluginHost>,
    marketplace: Marketplace,
}

impl SonoraApp {
    fn assemble(config: SonoraConfig, db: Database, player: AudioPlayer) -> Result<Self> {
        let db = Arc::new(db);
        let player = Arc::new(player);
        let queue_handle = Arc::new(Mutex::new(PlaybackQueue::new()));
        let artwork_cache = Arc::new(ArtworkCache::new());
        let plugins = Arc::new(PluginHost::new().map_err(sonora_common::SonoraError::from)?);
        let bus = EventBus::default();
        let marketplace = Marketplace::new(&config.data_dir, Arc::clone(&plugins), None, None)
            .map_err(sonora_common::SonoraError::from)?;
        let enrichment = crate::enrichment::EnrichmentRouter::new(Arc::clone(&db), None);

        let app = Self {
            library: LibraryService::new(Arc::clone(&db), Arc::clone(&artwork_cache), bus.clone()),
            playback: PlaybackService::new(Arc::clone(&player), bus.clone()),
            queue: QueueService::new(Arc::clone(&queue_handle), bus.clone()),
            lyrics: LyricsService::new(Arc::clone(&db), Arc::clone(&plugins), bus.clone()),
            enrichment,
            config,
            db,
            bus,
            plugins,
            marketplace,
        };

        app.playback
            .player()
            .set_volume(app.config.default_volume)
            .map_err(|e| SonoraError::Audio(e.to_string()))?;
        Ok(app)
    }

    /// Initialize the application context with configuration, SQLite storage, and audio player.
    pub fn new(config: SonoraConfig) -> Result<Self> {
        std::fs::create_dir_all(&config.data_dir)
            .map_err(|e| SonoraError::Config(format!("Failed to create data dir: {e}")))?;

        let db_path = config.data_dir.join("library.sqlite3");
        let db = Database::open(&db_path)?;
        let player = AudioPlayer::new().unwrap_or_else(|e| {
            tracing::warn!(
                "Hardware audio device unavailable ({e}), falling back to virtual audio output"
            );
            AudioPlayer::virtual_player(48000, 2).expect("virtual audio initialization")
        });
        Self::assemble(config, db, player)
    }

    /// Initialize an in-memory application context with virtual audio (for tests and headless verification).
    pub fn in_memory(config: SonoraConfig) -> Result<Self> {
        let db = Database::in_memory()?;
        let player = AudioPlayer::virtual_player(48000, 2)?;
        Self::assemble(config, db, player)
    }

    // --- Composition accessors ---

    pub fn config(&self) -> &SonoraConfig {
        &self.config
    }

    /// Direct database handle (compatibility for hosts that build their own
    /// repositories, e.g. the desktop shell). New code should prefer the
    /// service APIs and the event bus.
    pub fn db(&self) -> &Database {
        &self.db
    }

    pub fn player(&self) -> &AudioPlayer {
        self.playback.player()
    }

    pub fn queue(&self) -> &Arc<Mutex<PlaybackQueue>> {
        self.queue.handle()
    }

    pub fn plugin_host(&self) -> &Arc<PluginHost> {
        &self.plugins
    }

    pub fn marketplace(&self) -> &Marketplace {
        &self.marketplace
    }

    pub fn event_bus(&self) -> &EventBus {
        &self.bus
    }

    pub fn subscribe_events(&self) -> broadcast::Receiver<SonoraEvent> {
        self.bus.subscribe()
    }

    pub fn broadcast_event(&self, event: SonoraEvent) {
        self.bus.publish(event);
    }

    /// Forward pending [`PluginHost`] lifecycle transitions onto the event
    /// bus as `PluginStateChanged`. Called automatically after every plugin
    /// command; UI loops may also poll it.
    pub fn forward_plugin_events(&self) {
        for e in self.plugins.drain_events() {
            self.bus.publish(SonoraEvent::PluginStateChanged {
                plugin_id: e.plugin_id,
                from: e.from.to_string(),
                to: e.to.to_string(),
            });
        }
    }

    // --- Command plane ---

    /// Route a control-plane mutation to its owning service.
    pub fn handle_command(&self, command: SonoraCommand) -> Result<()> {
        match command {
            SonoraCommand::PlayFile(path) => self.play_file(path),
            SonoraCommand::PlayTrack(id) => self.play_track_id(id),
            SonoraCommand::PlayAlbum(id) => self.play_album(id),
            SonoraCommand::PlayQueueIndex(i) => self.play_queue_index(i),
            SonoraCommand::Pause => self.pause(),
            SonoraCommand::Resume => self.resume(),
            SonoraCommand::Stop => self.stop(),
            SonoraCommand::Seek(ms) => self.seek(ms),
            SonoraCommand::SetVolume(v) => self.set_volume(v),
            SonoraCommand::QueueNext => self.queue_next().map(|_| ()),
            SonoraCommand::QueuePrevious => self.queue_previous().map(|_| ()),
            SonoraCommand::EnqueueTrack(id) => self.enqueue_track(id),
            SonoraCommand::RemoveFromQueue(i) => self.remove_from_queue(i).map(|_| ()),
            SonoraCommand::MoveQueueItem { from, to } => self.move_queue_item(from, to).map(|_| ()),
            SonoraCommand::ClearQueue => {
                self.clear_queue();
                Ok(())
            }
            SonoraCommand::ScanDirectory(path) => self.scan_directory(path).map(|_| ()),
            SonoraCommand::PluginRegister { manifest, dir } => {
                self.register_plugin(*manifest, dir)?;
                self.forward_plugin_events();
                Ok(())
            }
            SonoraCommand::PluginLoad { id } => {
                self.plugins.load(&id).map_err(SonoraError::from)?;
                self.forward_plugin_events();
                Ok(())
            }
            SonoraCommand::PluginLoadBytes { id, wasm } => {
                self.plugins
                    .load_bytes(&id, &wasm)
                    .map_err(SonoraError::from)?;
                self.forward_plugin_events();
                Ok(())
            }
            SonoraCommand::PluginStart { id } => {
                self.plugins.start(&id).map_err(SonoraError::from)?;
                self.forward_plugin_events();
                Ok(())
            }
            SonoraCommand::PluginStop { id } => {
                self.plugins.stop(&id).map_err(SonoraError::from)?;
                self.forward_plugin_events();
                Ok(())
            }
            SonoraCommand::PluginUnload { id } => {
                self.plugins.unload(&id).map_err(SonoraError::from)?;
                self.forward_plugin_events();
                Ok(())
            }
            SonoraCommand::RegistryRefresh => self.market_refresh().map(|_| ()),
            SonoraCommand::MarketInstall { id, version } => {
                self.market_install(&id, version.as_deref()).map(|_| ())
            }
            SonoraCommand::MarketUpdate { id } => self.market_update(&id).map(|_| ()),
            SonoraCommand::MarketRollback { id, version } => {
                self.market_rollback(&id, version.as_deref()).map(|_| ())
            }
            SonoraCommand::MarketUninstall { id } => self.market_uninstall(&id),
            SonoraCommand::MarketSetActiveTheme { id } => {
                self.market_set_active_theme(id.as_deref())
            }
        }
    }

    // --- Plugin orchestration ---

    /// Register a validated manifest (discover-equivalent for explicit bytes).
    pub fn register_plugin(&self, manifest: PluginManifest, dir: PathBuf) -> Result<()> {
        self.plugins
            .register(manifest, dir)
            .map_err(SonoraError::from)?;
        self.forward_plugin_events();
        Ok(())
    }

    /// Discover plugin directories under `dir` (validate-only; use
    /// [`SonoraCommand::PluginLoad`] to instantiate).
    pub fn discover_plugins(&self, dir: &Path) -> Vec<String> {
        let ids = self.plugins.discover(dir);
        self.forward_plugin_events();
        ids
    }

    // --- Marketplace (delegated to the registry service) ---

    /// Refresh the registry index from the network.
    pub fn market_refresh(&self) -> Result<sonora_registry::Catalog> {
        self.marketplace.refresh().map_err(SonoraError::from)
    }

    /// Catalog with offline fallback to the last validated cache.
    pub fn market_catalog(&self) -> Result<sonora_registry::Catalog> {
        self.marketplace.catalog().map_err(SonoraError::from)
    }

    /// Installed extensions merged with live host state.
    pub fn market_installed(&self) -> Result<Vec<sonora_registry::InstalledEntry>> {
        self.marketplace.installed().map_err(SonoraError::from)
    }

    /// Available updates for installed extensions.
    pub fn market_updates(&self) -> Result<Vec<sonora_registry::UpdateInfo>> {
        self.marketplace.updates().map_err(SonoraError::from)
    }

    pub fn market_install(
        &self,
        id: &str,
        version: Option<&str>,
    ) -> Result<sonora_registry::InstallReport> {
        let report = self
            .marketplace
            .install(id, version)
            .map_err(SonoraError::from)?;
        self.forward_plugin_events();
        Ok(report)
    }

    pub fn market_update(&self, id: &str) -> Result<sonora_registry::InstallReport> {
        let report = self.marketplace.update(id).map_err(SonoraError::from)?;
        self.forward_plugin_events();
        Ok(report)
    }

    pub fn market_rollback(
        &self,
        id: &str,
        version: Option<&str>,
    ) -> Result<sonora_registry::RollbackReport> {
        let report = self
            .marketplace
            .rollback(id, version)
            .map_err(SonoraError::from)?;
        self.forward_plugin_events();
        Ok(report)
    }

    pub fn market_uninstall(&self, id: &str) -> Result<()> {
        self.marketplace.uninstall(id).map_err(SonoraError::from)?;
        self.forward_plugin_events();
        Ok(())
    }

    /// Installed, validated themes for the theme selector.
    pub fn market_themes(&self) -> Result<Vec<sonora_registry::InstalledTheme>> {
        self.marketplace
            .installed_themes()
            .map_err(SonoraError::from)
    }

    /// Full validated definition of one installed theme.
    pub fn market_theme_definition(&self, id: &str) -> Result<sonora_registry::ThemeDefinition> {
        self.marketplace
            .theme_definition(id)
            .map_err(SonoraError::from)
    }

    /// Supplemental stylesheet of one installed theme, if any.
    pub fn market_theme_css(&self, id: &str) -> Result<Option<String>> {
        self.marketplace.theme_css(id).map_err(SonoraError::from)
    }

    /// Persisted marketplace theme selection (`None` = built-in default).
    pub fn market_active_theme(&self) -> Result<Option<String>> {
        self.marketplace.active_theme().map_err(SonoraError::from)
    }

    pub fn market_set_active_theme(&self, id: Option<&str>) -> Result<()> {
        self.marketplace
            .set_active_theme(id)
            .map_err(SonoraError::from)
    }

    // --- Library Operations (delegated) ---

    /// Scan a directory recursively, extracting metadata with Lofty and indexing into SQLite.
    pub fn scan_directory<P: AsRef<Path>>(&self, path: P) -> Result<ScanStats> {
        self.library.scan_directory(path)
    }

    /// Search library tracks using FTS5 trigram full-text query.
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        self.library.search(query, limit)
    }

    pub fn get_all_tracks(&self, limit: usize) -> Result<Vec<SearchResult>> {
        self.library.get_all_tracks(limit)
    }

    pub fn get_all_albums(&self) -> Result<Vec<AlbumDto>> {
        self.library.get_all_albums()
    }

    pub fn get_all_artists(&self) -> Result<Vec<ArtistDto>> {
        self.library.get_all_artists()
    }

    pub fn get_album_tracks(&self, album_id: i64) -> Result<Vec<SearchResult>> {
        self.library.get_album_tracks(album_id)
    }

    pub fn get_artist_tracks(&self, artist_id: i64) -> Result<Vec<SearchResult>> {
        self.library.get_artist_tracks(artist_id)
    }

    pub fn get_track_artwork(&self, track_id: TrackId, thumbnail: bool) -> Result<Option<String>> {
        self.library.get_track_artwork(track_id, thumbnail)
    }

    pub fn get_album_artwork(&self, album_id: i64, thumbnail: bool) -> Result<Option<String>> {
        self.library.get_album_artwork(album_id, thumbnail)
    }

    pub fn get_library_summary(&self) -> Result<LibrarySummary> {
        self.library.get_library_summary()
    }

    // --- Playback Operations (orchestrated) ---

    /// Open and play an audio track by file path.
    pub fn play_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let p = path.as_ref();
        if !p.exists() {
            return Err(SonoraError::Audio(format!(
                "File does not exist: {}",
                p.display()
            )));
        }

        let queue_item = QueueItem {
            track_id: None,
            file_path: p.to_path_buf(),
            title: p
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default(),
            artist: None,
            album: None,
            duration_ms: 0,
        };
        self.queue.replace(vec![queue_item], 0);
        self.playback.play_file(p)
    }

    /// Open and play a track from the library by its TrackId.
    pub fn play_track_id(&self, track_id: TrackId) -> Result<()> {
        let details = self
            .library
            .get_track_details(track_id)?
            .ok_or_else(|| SonoraError::Library(format!("Track ID {track_id:?} not found")))?;

        let queue_item = QueueItem {
            track_id: Some(details.track_id),
            file_path: PathBuf::from(&details.file_path),
            title: details.title.clone(),
            artist: details.artist_name.clone(),
            album: details.album_title.clone(),
            duration_ms: details.duration_ms as u64,
        };
        self.queue.replace(vec![queue_item], 0);
        self.playback.play_file(&details.file_path)?;
        self.playback.announce_track(
            track_id,
            details.title,
            details.artist_name,
            details.duration_ms as u64,
        );
        Ok(())
    }

    pub fn pause(&self) -> Result<()> {
        self.playback.pause()
    }

    pub fn resume(&self) -> Result<()> {
        self.playback.resume()
    }

    pub fn stop(&self) -> Result<()> {
        self.playback.stop()
    }

    pub fn seek(&self, position_ms: u64) -> Result<()> {
        self.playback.seek(position_ms)
    }

    pub fn set_volume(&self, volume: f32) -> Result<()> {
        self.playback.set_volume(volume)
    }

    pub fn volume(&self) -> f32 {
        self.playback.volume()
    }

    pub fn playback_state(&self) -> PlaybackState {
        self.playback.state()
    }

    // --- Queue Management (orchestrated) ---

    pub fn enqueue_track(&self, track_id: TrackId) -> Result<()> {
        let details = self
            .library
            .get_track_details(track_id)?
            .ok_or_else(|| SonoraError::Library(format!("Track ID {track_id:?} not found")))?;

        self.queue.enqueue(QueueItem {
            track_id: Some(details.track_id),
            file_path: PathBuf::from(&details.file_path),
            title: details.title,
            artist: details.artist_name,
            album: details.album_title,
            duration_ms: details.duration_ms as u64,
        });
        Ok(())
    }

    pub fn queue_next(&self) -> Result<bool> {
        match self.queue.next() {
            Some(item) => {
                self.playback.play_file(&item.file_path)?;
                if let Some(tid) = item.track_id {
                    self.playback
                        .announce_track(tid, item.title, item.artist, item.duration_ms);
                }
                Ok(true)
            }
            None => {
                self.playback.stop()?;
                Ok(false)
            }
        }
    }

    pub fn queue_previous(&self) -> Result<bool> {
        // Standard player behavior: if song played for > 3s, seek to 0:00
        if self.playback.position_ms() > 3000 {
            self.playback.seek(0)?;
            return Ok(true);
        }

        match self.queue.previous() {
            Some(item) => {
                self.playback.play_file(&item.file_path)?;
                if let Some(tid) = item.track_id {
                    self.playback
                        .announce_track(tid, item.title, item.artist, item.duration_ms);
                }
                Ok(true)
            }
            None => {
                // At start of queue; seek back to beginning of track
                self.playback.seek(0)?;
                Ok(false)
            }
        }
    }

    /// Check if current track finished and advance queue if more items exist.
    pub fn poll_queue(&self) -> Result<()> {
        if self.playback.is_finished() {
            let _ = self.queue_next()?;
        }
        Ok(())
    }

    pub fn status(&self) -> PlaybackStatus {
        let snap = self.playback.snapshot();
        PlaybackStatus {
            state: snap.state,
            current_track: self.queue.current(),
            position_ms: snap.position_ms,
            duration_ms: snap.duration_ms,
            volume: snap.volume,
            queue_length: self.queue.len(),
            current_queue_index: self.queue.current_index(),
        }
    }

    pub fn play_album(&self, album_id: i64) -> Result<()> {
        let tracks = self.library.get_album_tracks(album_id)?;
        if tracks.is_empty() {
            return Ok(());
        }

        let queue_items: Vec<QueueItem> = tracks
            .iter()
            .map(|t| QueueItem {
                track_id: Some(t.track_id),
                file_path: PathBuf::from(&t.file_path),
                title: t.title.clone(),
                artist: t.artist_name.clone(),
                album: t.album_title.clone(),
                duration_ms: t.duration_ms as u64,
            })
            .collect();

        let first = queue_items[0].clone();
        self.queue.replace(queue_items, 0);
        self.playback.play_file(&first.file_path)?;
        if let Some(tid) = first.track_id {
            self.playback
                .announce_track(tid, first.title, first.artist, first.duration_ms);
        } else {
            self.bus
                .publish(SonoraEvent::PlaybackStateChanged(PlaybackState::Playing));
        }
        Ok(())
    }

    pub fn get_queue(&self) -> Vec<QueueItem> {
        self.queue.items()
    }

    pub fn remove_from_queue(&self, index: usize) -> Result<Option<QueueItem>> {
        Ok(self.queue.remove(index))
    }

    pub fn move_queue_item(&self, from: usize, to: usize) -> Result<bool> {
        Ok(self.queue.move_item(from, to))
    }

    pub fn play_queue_index(&self, index: usize) -> Result<()> {
        if let Some(item) = self.queue.set_current_index(index) {
            self.playback.play_file(&item.file_path)?;
            if let Some(tid) = item.track_id {
                self.playback
                    .announce_track(tid, item.title, item.artist, item.duration_ms);
            } else {
                self.bus
                    .publish(SonoraEvent::PlaybackStateChanged(PlaybackState::Playing));
            }
        }
        Ok(())
    }

    pub fn clear_queue(&self) {
        self.queue.clear();
    }

    pub fn visualizer_data(&self) -> Vec<f32> {
        self.playback.visualizer_data()
    }

    /// Retrieve lyrics for a track, checking SQLite cache first and cascading through providers on cache miss.
    pub fn get_lyrics(
        &self,
        query: sonora_lyrics::provider::TrackLyricsQuery,
    ) -> Result<Option<sonora_lyrics::model::LyricsDocument>> {
        self.lyrics.get_lyrics(query)
    }

    /// Save manual offset adjustment to SQLite cache.
    pub fn save_lyrics_offset(
        &self,
        track_id: Option<i64>,
        file_path: Option<&str>,
        offset_ms: i64,
    ) -> Result<()> {
        self.lyrics
            .save_lyrics_offset(track_id, file_path, offset_ms)
    }

    // --- Online Enrichment Operations (Contextual) ---

    /// Reference to the enrichment router subsystem.
    pub fn enrichment(&self) -> &crate::enrichment::EnrichmentRouter {
        &self.enrichment
    }

    /// Asynchronously find and rank metadata candidates for a library track.
    pub async fn find_metadata_candidates(
        &self,
        track_id: TrackId,
    ) -> Result<Vec<crate::metadata::RankedCandidateMatch>> {
        let details = self
            .library
            .get_track_details(track_id)?
            .ok_or_else(|| SonoraError::Library(format!("Track ID {track_id:?} not found")))?;

        let local = crate::metadata::LocalTrackMetadata {
            title: details.title,
            artist: details.artist_name,
            album: details.album_title,
            duration_ms: details.duration_ms as u64,
            track_number: details.track_number.map(|n| n as u32),
            ..Default::default()
        };

        self.enrichment
            .find_metadata_candidates(&local)
            .await
            .map_err(|e| SonoraError::Internal(e.to_string()))
    }

    /// Apply approved metadata candidate to the local SQLite database.
    /// This never writes to or alters physical audio files on disk.
    pub fn apply_metadata(
        &self,
        track_id: TrackId,
        candidate: &crate::metadata::RankedCandidateMatch,
    ) -> Result<()> {
        let title = &candidate.candidate_track.title;
        let artist_name = candidate
            .candidate_track
            .artist_credits
            .first()
            .map(|c| c.name.as_str());
        let album_title = candidate
            .candidate_release
            .as_ref()
            .map(|r| r.title.as_str())
            .or_else(|| {
                candidate
                    .candidate_release_group
                    .as_ref()
                    .map(|rg| rg.title.as_str())
            });
        let release_year: Option<i32> = candidate
            .candidate_release
            .as_ref()
            .and_then(|r| r.date.as_ref())
            .or_else(|| {
                candidate
                    .candidate_release_group
                    .as_ref()
                    .and_then(|rg| rg.first_release_date.as_ref())
            })
            .and_then(|d| d.split('-').next())
            .and_then(|y| y.parse::<i32>().ok());
        let track_number: Option<i32> = candidate
            .candidate_track
            .position
            .map(|p| p as i32)
            .or_else(|| {
                candidate
                    .candidate_track
                    .number
                    .as_ref()
                    .and_then(|n| n.parse::<i32>().ok())
            });
        let disc_number: Option<i32> = None;
        let artist_mbid = candidate
            .candidate_track
            .artist_credits
            .first()
            .map(|c| c.artist_mbid.as_str());
        let album_mbid = candidate
            .candidate_release
            .as_ref()
            .map(|r| r.mbid.as_str())
            .or_else(|| {
                candidate
                    .candidate_release_group
                    .as_ref()
                    .map(|rg| rg.mbid.as_str())
            });

        let repo = LibraryRepository::new(&self.db);
        repo.update_track_metadata(
            track_id,
            title,
            artist_name,
            album_title,
            release_year,
            track_number,
            disc_number,
            artist_mbid,
            album_mbid,
        )
    }

    /// Asynchronously find artwork candidates across Cover Art Archive, Wikidata, and Fanart.tv.
    pub async fn find_artwork_candidates(
        &self,
        query: &crate::artwork::ArtworkQuery,
    ) -> Result<Vec<crate::artwork::ArtworkCandidate>> {
        self.enrichment
            .find_artwork_candidates(query)
            .await
            .map_err(|e| SonoraError::Internal(e.to_string()))
    }

    /// Download and set candidate artwork as active for an album or artist in SQLite.
    pub async fn apply_artwork(
        &self,
        target_type: &str,
        target_id: i64,
        image_url: &str,
    ) -> Result<crate::artwork::CachedArtworkAsset> {
        let asset = self
            .enrichment
            .download_and_cache_artwork(image_url)
            .await
            .map_err(|e| SonoraError::Internal(e.to_string()))?;

        let repo = LibraryRepository::new(&self.db);
        match target_type {
            "album" => {
                repo.update_album_artwork(target_id, &asset.full_path)?;
            }
            "artist" => {
                repo.update_artist_artwork(target_id, &asset.full_path)?;
            }
            _ => {}
        }

        Ok(asset)
    }

    /// Asynchronously find lyrics candidates from online provider.
    pub async fn find_lyrics_candidates(
        &self,
        query: &crate::lyrics_online::LyricsCandidateQuery,
    ) -> Result<Vec<crate::lyrics_online::LyricsCandidate>> {
        self.enrichment
            .find_lyrics_candidates(query)
            .await
            .map_err(|e| SonoraError::Internal(e.to_string()))
    }

    /// Apply chosen lyrics candidate to the persistent SQLite cache.
    pub fn apply_lyrics_candidate(
        &self,
        track_id: Option<i64>,
        file_path: Option<&str>,
        candidate: &crate::lyrics_online::LyricsCandidate,
    ) -> Result<()> {
        use sonora_lyrics::parser::{LrcLyricsParser, LyricsParser, PlainTextLyricsParser};
        let lrc_parser = LrcLyricsParser;
        let plain_parser = PlainTextLyricsParser;
        let doc = match candidate.sync_type {
            crate::lyrics_online::LyricsSyncType::LineSynced
            | crate::lyrics_online::LyricsSyncType::SyllableSynced => lrc_parser
                .parse(&candidate.raw_content)
                .or_else(|_| plain_parser.parse(&candidate.raw_content))?,
            crate::lyrics_online::LyricsSyncType::PlainText => {
                plain_parser.parse(&candidate.raw_content)?
            }
        };

        let repo = LibraryRepository::new(&self.db);
        repo.save_cached_lyrics(
            track_id,
            file_path,
            &candidate.track_name,
            Some(&candidate.artist_name),
            &doc,
            &candidate.provider_name,
        )
    }

    /// Export synchronized/plain lyrics as a `.lrc` sidecar next to the local audio file.
    pub fn export_lrc_sidecar(
        &self,
        track_id: Option<i64>,
        file_path: Option<&str>,
        lrc_content: &str,
    ) -> Result<String> {
        let path_str = if let Some(fp) = file_path {
            fp.to_string()
        } else if let Some(tid) = track_id {
            let details = self
                .library
                .get_track_details(TrackId(tid))?
                .ok_or_else(|| SonoraError::Library(format!("Track ID {tid} not found")))?;
            details.file_path
        } else {
            return Err(SonoraError::Config(
                "Either track_id or file_path must be provided to export .lrc".to_string(),
            ));
        };

        let audio_path = Path::new(&path_str);
        let lrc_path = audio_path.with_extension("lrc");

        std::fs::write(&lrc_path, lrc_content).map_err(SonoraError::Io)?;

        Ok(lrc_path.to_string_lossy().to_string())
    }
}
