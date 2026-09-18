//! Library service: SQLite indexing and metadata reads.
//!
//! Owns the [`Database`] handle and the artwork cache. Publishes
//! `LibraryScanStarted/Progress/Completed` around scans; all reads are
//! side-effect free.

use crate::bus::EventBus;
use crate::event::SonoraEvent;
use sonora_common::{Result, TrackId};
use sonora_library::{
    AlbumDto, ArtistDto, ArtworkCache, Database, LibraryRepository, LibrarySummary, ScanStats,
    SearchResult, Track,
};
use std::path::Path;
use std::sync::Arc;

#[derive(Clone)]
pub struct LibraryService {
    db: Arc<Database>,
    artwork_cache: Arc<ArtworkCache>,
    bus: EventBus,
}

impl LibraryService {
    pub fn new(db: Arc<Database>, artwork_cache: Arc<ArtworkCache>, bus: EventBus) -> Self {
        Self {
            db,
            artwork_cache,
            bus,
        }
    }

    /// Scan a directory, emitting the scan lifecycle on the bus.
    pub fn scan_directory<P: AsRef<Path>>(&self, path: P) -> Result<ScanStats> {
        let display = path.as_ref().display().to_string();
        self.bus
            .publish(SonoraEvent::LibraryScanStarted { path: display });
        let repo = LibraryRepository::new(&self.db);
        let stats = repo.scan_directory(path)?;
        self.bus.publish(SonoraEvent::LibraryScanProgress {
            indexed_count: stats.indexed_tracks,
            total_count: stats.scanned_files,
        });
        self.bus.publish(SonoraEvent::LibraryScanCompleted {
            indexed_tracks: stats.indexed_tracks,
            scanned_files: stats.scanned_files,
        });
        Ok(stats)
    }

    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        LibraryRepository::new(&self.db).search(query, limit)
    }

    pub fn get_all_tracks(&self, limit: usize) -> Result<Vec<SearchResult>> {
        LibraryRepository::new(&self.db).get_all_tracks(limit)
    }

    pub fn get_all_albums(&self) -> Result<Vec<AlbumDto>> {
        LibraryRepository::new(&self.db).get_all_albums()
    }

    pub fn get_all_artists(&self) -> Result<Vec<ArtistDto>> {
        LibraryRepository::new(&self.db).get_all_artists()
    }

    pub fn get_album_tracks(&self, album_id: i64) -> Result<Vec<SearchResult>> {
        LibraryRepository::new(&self.db).get_album_tracks(album_id)
    }

    pub fn get_artist_tracks(&self, artist_id: i64) -> Result<Vec<SearchResult>> {
        LibraryRepository::new(&self.db).get_artist_tracks(artist_id)
    }

    pub fn get_track_by_id(&self, track_id: TrackId) -> Result<Option<Track>> {
        LibraryRepository::new(&self.db).get_track_by_id(track_id)
    }

    pub fn get_track_details(&self, track_id: TrackId) -> Result<Option<SearchResult>> {
        LibraryRepository::new(&self.db).get_track_details(track_id)
    }

    pub fn get_album_first_track(&self, album_id: i64) -> Result<Option<Track>> {
        LibraryRepository::new(&self.db).get_album_first_track(album_id)
    }

    pub fn get_library_summary(&self) -> Result<LibrarySummary> {
        let repo = LibraryRepository::new(&self.db);
        let (track_count, album_count, artist_count) = repo.get_library_counts()?;
        Ok(LibrarySummary {
            track_count,
            album_count,
            artist_count,
        })
    }

    pub fn get_track_artwork(&self, track_id: TrackId, thumbnail: bool) -> Result<Option<String>> {
        let track = self.get_track_by_id(track_id)?;
        let Some(track) = track else {
            return Ok(None);
        };
        Ok(self
            .artwork_cache
            .get_or_load_track_artwork(track_id.0, &track.file_path, thumbnail))
    }

    pub fn get_album_artwork(&self, album_id: i64, thumbnail: bool) -> Result<Option<String>> {
        let track = self.get_album_first_track(album_id)?;
        let Some(track) = track else {
            return Ok(None);
        };
        let album_key = -album_id;
        Ok(self
            .artwork_cache
            .get_or_load_track_artwork(album_key, &track.file_path, thumbnail))
    }
}
