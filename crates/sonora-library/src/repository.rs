use crate::db::Database;
use crate::metadata::MetadataExtractor;
use crate::models::{AlbumDto, ArtistDto, SearchResult, Track};
use rusqlite::params;
use sonora_common::{AlbumId, ArtistId, Result, SonoraError, TrackId};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

pub const SUPPORTED_EXTENSIONS: &[&str] = &[
    "flac", "mp3", "wav", "aac", "m4a", "ogg", "opus", "alac", "aiff",
];

use serde::{Deserialize, Serialize};

/// Statistics collected during a library directory scan.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ScanStats {
    pub scanned_files: usize,
    pub indexed_tracks: usize,
    pub skipped_unmodified: usize,
    pub errors: usize,
    /// Tracks removed because their files no longer exist under the scanned tree.
    pub pruned_missing: usize,
}

/// Repository handling data persistence and FTS5 search queries.
pub struct LibraryRepository<'a> {
    db: &'a Database,
}

impl<'a> LibraryRepository<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// Insert or retrieve an artist ID by artist name.
    pub fn upsert_artist(&self, name: &str) -> Result<i64> {
        let conn = self.db.lock_conn()?;
        conn.execute(
            "INSERT INTO artists (name) VALUES (?1) ON CONFLICT(name) DO NOTHING",
            params![name],
        )
        .map_err(|e| SonoraError::Database(e.to_string()))?;

        let id = conn
            .query_row(
                "SELECT id FROM artists WHERE name = ?1 COLLATE NOCASE",
                params![name],
                |row| row.get(0),
            )
            .map_err(|e| SonoraError::Database(e.to_string()))?;

        Ok(id)
    }

    /// Insert or retrieve an album ID by title and artist ID.
    pub fn upsert_album(
        &self,
        title: &str,
        artist_id: Option<i64>,
        year: Option<i32>,
    ) -> Result<i64> {
        let conn = self.db.lock_conn()?;
        conn.execute(
            "INSERT INTO albums (title, artist_id, release_year) VALUES (?1, ?2, ?3) ON CONFLICT DO NOTHING",
            params![title, artist_id, year],
        )
        .map_err(|e| SonoraError::Database(e.to_string()))?;

        let id = match artist_id {
            Some(aid) => conn.query_row(
                "SELECT id FROM albums WHERE title = ?1 COLLATE NOCASE AND artist_id = ?2",
                params![title, aid],
                |row| row.get(0),
            ),
            None => conn.query_row(
                "SELECT id FROM albums WHERE title = ?1 COLLATE NOCASE AND artist_id IS NULL",
                params![title],
                |row| row.get(0),
            ),
        }
        .map_err(|e| SonoraError::Database(e.to_string()))?;

        Ok(id)
    }

    /// Insert a track into the library and index into FTS5.
    #[allow(clippy::too_many_arguments)]
    pub fn insert_track(
        &self,
        file_path: &str,
        file_hash: &str,
        file_size_bytes: i64,
        file_modified_time: i64,
        title: &str,
        artist_id: Option<i64>,
        album_id: Option<i64>,
        track_number: Option<i32>,
        disc_number: i32,
        duration_ms: i64,
        sample_rate: i32,
        bit_depth: Option<i32>,
        channels: i32,
        bitrate_kbps: Option<i32>,
        codec: &str,
        genre: Option<&str>,
    ) -> Result<TrackId> {
        let conn = self.db.lock_conn()?;
        conn.execute(
            r#"
            INSERT INTO tracks (
                file_path, file_hash, file_size_bytes, file_modified_time,
                title, artist_id, album_id, track_number, disc_number,
                duration_ms, sample_rate, bit_depth, channels, bitrate_kbps, codec
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
            ON CONFLICT(file_path) DO UPDATE SET
                file_hash = excluded.file_hash,
                file_size_bytes = excluded.file_size_bytes,
                file_modified_time = excluded.file_modified_time,
                title = excluded.title,
                artist_id = excluded.artist_id,
                album_id = excluded.album_id,
                track_number = excluded.track_number,
                disc_number = excluded.disc_number,
                duration_ms = excluded.duration_ms,
                sample_rate = excluded.sample_rate,
                bit_depth = excluded.bit_depth,
                channels = excluded.channels,
                bitrate_kbps = excluded.bitrate_kbps,
                codec = excluded.codec
            "#,
            params![
                file_path,
                file_hash,
                file_size_bytes,
                file_modified_time,
                title,
                artist_id,
                album_id,
                track_number,
                disc_number,
                duration_ms,
                sample_rate,
                bit_depth,
                channels,
                bitrate_kbps,
                codec
            ],
        )
        .map_err(|e| SonoraError::Database(e.to_string()))?;

        let track_id: i64 = conn
            .query_row(
                "SELECT id FROM tracks WHERE file_path = ?1",
                params![file_path],
                |r| r.get(0),
            )
            .map_err(|e| SonoraError::Database(e.to_string()))?;

        // Synchronize in FTS5
        let artist_name: Option<String> = artist_id.and_then(|id| {
            conn.query_row("SELECT name FROM artists WHERE id = ?1", params![id], |r| {
                r.get(0)
            })
            .ok()
        });
        let album_title: Option<String> = album_id.and_then(|id| {
            conn.query_row("SELECT title FROM albums WHERE id = ?1", params![id], |r| {
                r.get(0)
            })
            .ok()
        });

        // Delete any existing entry for this track_id in FTS5
        let _ = conn.execute(
            "DELETE FROM fts_tracks WHERE track_id = ?1",
            params![track_id],
        );

        conn.execute(
            "INSERT INTO fts_tracks (track_id, title, artist_name, album_title, genre_names) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![track_id, title, artist_name, album_title, genre],
        )
        .map_err(|e| SonoraError::Database(format!("FTS5 index error: {e}")))?;

        Ok(TrackId(track_id))
    }

    /// Retrieve a track by ID.
    pub fn get_track_by_id(&self, track_id: TrackId) -> Result<Option<Track>> {
        let conn = self.db.lock_conn()?;
        let res = conn.query_row(
            r#"
            SELECT id, file_path, file_hash, file_size_bytes, file_modified_time,
                   title, artist_id, album_id, track_number, disc_number,
                   duration_ms, sample_rate, bit_depth, channels, bitrate_kbps, codec
            FROM tracks WHERE id = ?1
            "#,
            params![track_id.0],
            |row| {
                Ok(Track {
                    id: TrackId(row.get(0)?),
                    file_path: row.get(1)?,
                    file_hash: row.get(2)?,
                    file_size_bytes: row.get(3)?,
                    file_modified_time: row.get(4)?,
                    title: row.get(5)?,
                    artist_id: row.get::<_, Option<i64>>(6)?.map(ArtistId),
                    album_id: row.get::<_, Option<i64>>(7)?.map(AlbumId),
                    track_number: row.get(8)?,
                    disc_number: row.get(9)?,
                    duration_ms: row.get(10)?,
                    sample_rate: row.get(11)?,
                    bit_depth: row.get(12)?,
                    channels: row.get(13)?,
                    bitrate_kbps: row.get(14)?,
                    codec: row.get(15)?,
                })
            },
        );

        match res {
            Ok(track) => Ok(Some(track)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(SonoraError::Database(e.to_string())),
        }
    }

    /// Retrieve full track details with joined artist and album names.
    pub fn get_track_details(&self, track_id: TrackId) -> Result<Option<SearchResult>> {
        let conn = self.db.lock_conn()?;
        let res = conn.query_row(
            r#"
            SELECT t.id, t.file_path, t.title, ar.name, al.title, t.duration_ms, t.track_number
            FROM tracks t
            LEFT JOIN artists ar ON t.artist_id = ar.id
            LEFT JOIN albums al ON t.album_id = al.id
            WHERE t.id = ?1
            "#,
            params![track_id.0],
            |row| {
                Ok(SearchResult {
                    track_id: TrackId(row.get(0)?),
                    file_path: row.get(1)?,
                    title: row.get(2)?,
                    artist_name: row.get(3)?,
                    album_title: row.get(4)?,
                    duration_ms: row.get(5)?,
                    track_number: row.get(6)?,
                })
            },
        );

        match res {
            Ok(details) => Ok(Some(details)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(SonoraError::Database(e.to_string())),
        }
    }

    /// Search tracks using FTS5 trigram full-text query.
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        let query_trimmed = query.trim();
        if query_trimmed.is_empty() {
            return Ok(Vec::new());
        }

        let conn = self.db.lock_conn()?;

        // If query is 3 or more characters, attempt FTS5 first
        if query_trimmed.chars().count() >= 3 {
            let sanitized = format!("\"{}\"", query_trimmed.replace('"', ""));
            let stmt_res = conn.prepare(
                r#"
                SELECT t.id, t.file_path, t.title, ar.name, al.title, t.duration_ms, t.track_number
                FROM fts_tracks fts
                JOIN tracks t ON fts.track_id = t.id
                LEFT JOIN artists ar ON t.artist_id = ar.id
                LEFT JOIN albums al ON t.album_id = al.id
                WHERE fts_tracks MATCH ?1
                ORDER BY rank
                LIMIT ?2
                "#,
            );

            if let Ok(mut stmt) = stmt_res {
                let fts_res: std::result::Result<Vec<SearchResult>, _> = stmt
                    .query_map(params![sanitized, limit as i64], |row| {
                        Ok(SearchResult {
                            track_id: TrackId(row.get(0)?),
                            file_path: row.get(1)?,
                            title: row.get(2)?,
                            artist_name: row.get(3)?,
                            album_title: row.get(4)?,
                            duration_ms: row.get(5)?,
                            track_number: row.get(6)?,
                        })
                    })
                    .map(|mapped| mapped.filter_map(|r| r.ok()).collect());

                if let Ok(results) = fts_res {
                    if !results.is_empty() {
                        return Ok(results);
                    }
                }
            }
        }

        // Fallback to robust SQL LIKE query across title, artist, and album.
        // Handles queries < 3 chars, punctuation, symbols, and substring searches without crashing.
        let pattern = format!("%{}%", query_trimmed);
        let mut fallback_stmt = conn
            .prepare(
                r#"
                SELECT t.id, t.file_path, t.title, ar.name, al.title, t.duration_ms, t.track_number
                FROM tracks t
                LEFT JOIN artists ar ON t.artist_id = ar.id
                LEFT JOIN albums al ON t.album_id = al.id
                WHERE t.title LIKE ?1
                   OR ar.name LIKE ?1
                   OR al.title LIKE ?1
                ORDER BY t.title COLLATE NOCASE ASC
                LIMIT ?2
                "#,
            )
            .map_err(|e| SonoraError::Database(e.to_string()))?;

        let results = fallback_stmt
            .query_map(params![pattern, limit as i64], |row| {
                Ok(SearchResult {
                    track_id: TrackId(row.get(0)?),
                    file_path: row.get(1)?,
                    title: row.get(2)?,
                    artist_name: row.get(3)?,
                    album_title: row.get(4)?,
                    duration_ms: row.get(5)?,
                    track_number: row.get(6)?,
                })
            })
            .map_err(|e| SonoraError::Database(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(results)
    }

    /// Retrieve all indexed tracks ordered by title.
    pub fn get_all_tracks(&self, limit: usize) -> Result<Vec<SearchResult>> {
        let conn = self.db.lock_conn()?;
        let mut stmt = conn
            .prepare(
                r#"
                SELECT t.id, t.file_path, t.title, ar.name, al.title, t.duration_ms, t.track_number
                FROM tracks t
                LEFT JOIN artists ar ON t.artist_id = ar.id
                LEFT JOIN albums al ON t.album_id = al.id
                ORDER BY t.title COLLATE NOCASE ASC
                LIMIT ?1
                "#,
            )
            .map_err(|e| SonoraError::Database(e.to_string()))?;

        let rows = stmt
            .query_map(params![limit as i64], |row| {
                Ok(SearchResult {
                    track_id: TrackId(row.get(0)?),
                    file_path: row.get(1)?,
                    title: row.get(2)?,
                    artist_name: row.get(3)?,
                    album_title: row.get(4)?,
                    duration_ms: row.get(5)?,
                    track_number: row.get(6)?,
                })
            })
            .map_err(|e| SonoraError::Database(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(rows)
    }

    /// Retrieve all indexed albums with aggregate track statistics.
    pub fn get_all_albums(&self) -> Result<Vec<AlbumDto>> {
        let conn = self.db.lock_conn()?;
        let mut stmt = conn
            .prepare(
                r#"
                SELECT al.id, al.title, al.artist_id, ar.name, al.release_year,
                       COUNT(t.id) as track_count,
                       COALESCE(SUM(t.duration_ms), 0) as total_duration_ms
                FROM albums al
                LEFT JOIN artists ar ON al.artist_id = ar.id
                LEFT JOIN tracks t ON t.album_id = al.id
                GROUP BY al.id
                ORDER BY al.title COLLATE NOCASE ASC
                "#,
            )
            .map_err(|e| SonoraError::Database(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| {
                Ok(AlbumDto {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    artist_id: row.get(2)?,
                    artist_name: row.get(3)?,
                    release_year: row.get(4)?,
                    track_count: row.get::<_, i64>(5)? as usize,
                    total_duration_ms: row.get::<_, i64>(6)? as u64,
                })
            })
            .map_err(|e| SonoraError::Database(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(rows)
    }

    /// Retrieve all indexed artists with album and track counts.
    pub fn get_all_artists(&self) -> Result<Vec<ArtistDto>> {
        let conn = self.db.lock_conn()?;
        let mut stmt = conn
            .prepare(
                r#"
                SELECT ar.id, ar.name,
                       COUNT(DISTINCT al.id) as album_count,
                       COUNT(t.id) as track_count
                FROM artists ar
                LEFT JOIN albums al ON al.artist_id = ar.id
                LEFT JOIN tracks t ON t.artist_id = ar.id
                GROUP BY ar.id
                ORDER BY ar.name COLLATE NOCASE ASC
                "#,
            )
            .map_err(|e| SonoraError::Database(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| {
                Ok(ArtistDto {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    album_count: row.get::<_, i64>(2)? as usize,
                    track_count: row.get::<_, i64>(3)? as usize,
                })
            })
            .map_err(|e| SonoraError::Database(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(rows)
    }

    /// Retrieve tracks belonging to an album in disc and track order.
    pub fn get_album_tracks(&self, album_id: i64) -> Result<Vec<SearchResult>> {
        let conn = self.db.lock_conn()?;
        let mut stmt = conn
            .prepare(
                r#"
                SELECT t.id, t.file_path, t.title, ar.name, al.title, t.duration_ms, t.track_number
                FROM tracks t
                LEFT JOIN artists ar ON t.artist_id = ar.id
                LEFT JOIN albums al ON t.album_id = al.id
                WHERE t.album_id = ?1
                ORDER BY t.disc_number ASC, COALESCE(t.track_number, 9999) ASC, t.id ASC
                "#,
            )
            .map_err(|e| SonoraError::Database(e.to_string()))?;

        let rows = stmt
            .query_map(params![album_id], |row| {
                Ok(SearchResult {
                    track_id: TrackId(row.get(0)?),
                    file_path: row.get(1)?,
                    title: row.get(2)?,
                    artist_name: row.get(3)?,
                    album_title: row.get(4)?,
                    duration_ms: row.get(5)?,
                    track_number: row.get(6)?,
                })
            })
            .map_err(|e| SonoraError::Database(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(rows)
    }

    /// Retrieve tracks by artist ID.
    pub fn get_artist_tracks(&self, artist_id: i64) -> Result<Vec<SearchResult>> {
        let conn = self.db.lock_conn()?;
        let mut stmt = conn
            .prepare(
                r#"
                SELECT t.id, t.file_path, t.title, ar.name, al.title, t.duration_ms, t.track_number
                FROM tracks t
                LEFT JOIN artists ar ON t.artist_id = ar.id
                LEFT JOIN albums al ON t.album_id = al.id
                WHERE t.artist_id = ?1
                ORDER BY al.title COLLATE NOCASE ASC, COALESCE(t.track_number, 9999) ASC, t.id ASC
                "#,
            )
            .map_err(|e| SonoraError::Database(e.to_string()))?;

        let rows = stmt
            .query_map(params![artist_id], |row| {
                Ok(SearchResult {
                    track_id: TrackId(row.get(0)?),
                    file_path: row.get(1)?,
                    title: row.get(2)?,
                    artist_name: row.get(3)?,
                    album_title: row.get(4)?,
                    duration_ms: row.get(5)?,
                    track_number: row.get(6)?,
                })
            })
            .map_err(|e| SonoraError::Database(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(rows)
    }

    /// Retrieve track artwork either from embedded tags or local directory cover images.
    pub fn get_track_artwork(&self, track_id: TrackId) -> Result<Option<String>> {
        let track = self.get_track_by_id(track_id)?;
        let Some(track) = track else {
            return Ok(None);
        };

        let p = Path::new(&track.file_path);
        // 1. Try extracting embedded picture with Lofty
        use lofty::file::TaggedFileExt;
        if let Ok(tagged_file) = lofty::probe::Probe::open(p).and_then(|pr| pr.read()) {
            let tag = tagged_file
                .primary_tag()
                .or_else(|| tagged_file.first_tag());
            if let Some(tag) = tag {
                if let Some(picture) = tag.pictures().first() {
                    let mime = picture
                        .mime_type()
                        .map(|m| m.as_str())
                        .unwrap_or("image/jpeg");
                    let b64 = to_base64(picture.data());
                    return Ok(Some(format!("data:{mime};base64,{b64}")));
                }
            }
        }

        // 2. Try looking for cover art in parent directory
        if let Some(parent) = p.parent() {
            let candidates = [
                "cover.jpg",
                "cover.png",
                "cover.jpeg",
                "folder.jpg",
                "folder.png",
                "front.jpg",
                "album.jpg",
            ];
            for c in &candidates {
                let art_path = parent.join(c);
                if art_path.is_file() {
                    if let Ok(data) = std::fs::read(&art_path) {
                        let mime = if c.ends_with(".png") {
                            "image/png"
                        } else {
                            "image/jpeg"
                        };
                        let b64 = to_base64(&data);
                        return Ok(Some(format!("data:{mime};base64,{b64}")));
                    }
                }
            }
        }

        Ok(None)
    }

    /// Retrieve the first track of an album (used for album artwork resolution).
    pub fn get_album_first_track(&self, album_id: i64) -> Result<Option<Track>> {
        let conn = self.db.lock_conn()?;
        let res = conn.query_row(
            r#"
            SELECT id, file_path, file_hash, file_size_bytes, file_modified_time,
                   title, artist_id, album_id, track_number, disc_number,
                   duration_ms, sample_rate, bit_depth, channels, bitrate_kbps, codec
            FROM tracks WHERE album_id = ?1
            ORDER BY COALESCE(track_number, 9999) ASC, id ASC
            LIMIT 1
            "#,
            params![album_id],
            |row| {
                Ok(Track {
                    id: TrackId(row.get(0)?),
                    file_path: row.get(1)?,
                    file_hash: row.get(2)?,
                    file_size_bytes: row.get(3)?,
                    file_modified_time: row.get(4)?,
                    title: row.get(5)?,
                    artist_id: row.get::<_, Option<i64>>(6)?.map(ArtistId),
                    album_id: row.get::<_, Option<i64>>(7)?.map(AlbumId),
                    track_number: row.get(8)?,
                    disc_number: row.get(9)?,
                    duration_ms: row.get(10)?,
                    sample_rate: row.get(11)?,
                    bit_depth: row.get(12)?,
                    channels: row.get(13)?,
                    bitrate_kbps: row.get(14)?,
                    codec: row.get(15)?,
                })
            },
        );

        match res {
            Ok(track) => Ok(Some(track)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(SonoraError::Database(e.to_string())),
        }
    }

    /// Get total counts of tracks, albums, and artists in the library.
    pub fn get_library_counts(&self) -> Result<(usize, usize, usize)> {
        let conn = self.db.lock_conn()?;
        let track_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM tracks", [], |r| r.get(0))
            .unwrap_or(0);
        let album_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM albums", [], |r| r.get(0))
            .unwrap_or(0);
        let artist_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM artists", [], |r| r.get(0))
            .unwrap_or(0);
        Ok((
            track_count as usize,
            album_count as usize,
            artist_count as usize,
        ))
    }

    /// Scan a directory recursively, extracting metadata with Lofty and indexing into SQLite.
    /// Utilizes mtime/size checks to skip unchanged files and commits in batches.
    pub fn scan_directory<P: AsRef<Path>>(&self, dir: P) -> Result<ScanStats> {
        let mut stats = ScanStats::default();
        let supported: HashSet<&str> = SUPPORTED_EXTENSIONS.iter().copied().collect();

        let mut files_to_process = Vec::new();
        let mut visited_dirs = HashSet::new();
        Self::collect_audio_files(
            dir.as_ref(),
            &supported,
            &mut files_to_process,
            &mut visited_dirs,
        )?;
        stats.scanned_files = files_to_process.len();

        for path in &files_to_process {
            let metadata_res = std::fs::metadata(path);
            let fs_meta = match metadata_res {
                Ok(m) => m,
                Err(_) => {
                    stats.errors += 1;
                    continue;
                }
            };

            let file_size = fs_meta.len() as i64;
            let mtime = fs_meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);

            let file_path_str = path.to_string_lossy().to_string();

            // Check if existing track matches mtime & size
            let is_unmodified = {
                let conn = self.db.lock_conn()?;
                let existing: Option<(i64, i64)> = conn
                    .query_row(
                        "SELECT file_modified_time, file_size_bytes FROM tracks WHERE file_path = ?1",
                        params![file_path_str],
                        |r| Ok((r.get(0)?, r.get(1)?)),
                    )
                    .ok();

                if let Some((stored_mtime, stored_size)) = existing {
                    stored_mtime == mtime && stored_size == file_size
                } else {
                    false
                }
            };

            if is_unmodified {
                stats.skipped_unmodified += 1;
                continue;
            }

            // Extract tags with Lofty
            match MetadataExtractor::extract(path) {
                Ok(tag) => {
                    let title = tag.title.unwrap_or_else(|| {
                        path.file_stem()
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or_else(|| "Unknown Title".to_string())
                    });

                    let artist_name = tag.artist.as_deref().unwrap_or("Unknown Artist");
                    let artist_id = self.upsert_artist(artist_name).ok();

                    let album_title = tag.album.as_deref().unwrap_or("Unknown Album");
                    let album_id = self
                        .upsert_album(album_title, artist_id, tag.year.map(|y| y as i32))
                        .ok();

                    let codec = path
                        .extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("unknown")
                        .to_lowercase();

                    let file_hash = format!("{mtime:x}:{file_size:x}");

                    let track_res = self.insert_track(
                        &file_path_str,
                        &file_hash,
                        file_size,
                        mtime,
                        &title,
                        artist_id,
                        album_id,
                        tag.track_number.map(|t| t as i32),
                        tag.disc_number.map(|d| d as i32).unwrap_or(1),
                        tag.duration_ms as i64,
                        tag.sample_rate.unwrap_or(44100) as i32,
                        tag.bit_depth.map(|b| b as i32),
                        tag.channels.unwrap_or(2) as i32,
                        tag.bitrate_kbps.map(|b| b as i32),
                        &codec,
                        tag.genre.as_deref(),
                    );

                    if track_res.is_ok() {
                        stats.indexed_tracks += 1;
                    } else {
                        stats.errors += 1;
                    }
                }
                Err(_) => {
                    stats.errors += 1;
                }
            }
        }

        // Prune entries whose files vanished from this tree (moved, renamed,
        // or deleted tracks would otherwise linger as ghosts forever).
        stats.pruned_missing = self.prune_missing_files(dir.as_ref(), &files_to_process)?;

        Ok(stats)
    }

    /// Remove track rows under `dir` whose files no longer exist on disk.
    /// Scoped to the scanned tree so scanning one folder never touches tracks
    /// imported from another. Missing-file checks are best-effort: transient
    /// I/O errors leave the row in place rather than deleting user data.
    fn prune_missing_files(&self, dir: &Path, scanned: &[PathBuf]) -> Result<usize> {
        let canonical_root = match dir.canonicalize() {
            Ok(p) => p,
            Err(_) => return Ok(0),
        };
        let mut prefix = canonical_root.to_string_lossy().into_owned();
        if !prefix.ends_with(std::path::MAIN_SEPARATOR) {
            prefix.push(std::path::MAIN_SEPARATOR);
        }
        // Escape LIKE wildcards so the prefix match is literal.
        let like_pattern = format!(
            "{}%",
            prefix
                .replace('\\', "\\\\")
                .replace('%', "\\%")
                .replace('_', "\\_")
        );

        let present: HashSet<String> = scanned
            .iter()
            .filter_map(|p| {
                p.canonicalize()
                    .ok()
                    .map(|c| c.to_string_lossy().into_owned())
            })
            .collect();

        let candidates: Vec<(i64, String)> = {
            let conn = self.db.lock_conn()?;
            let mut stmt = conn
                .prepare("SELECT id, file_path FROM tracks WHERE file_path LIKE ?1 ESCAPE '\\'")
                .map_err(|e| SonoraError::Database(e.to_string()))?;
            let rows = stmt
                .query_map([like_pattern], |row| Ok((row.get(0)?, row.get(1)?)))
                .map_err(|e| SonoraError::Database(e.to_string()))?;
            rows.filter_map(|r| r.ok()).collect()
        };

        let mut pruned = 0;
        for (track_id, file_path) in candidates {
            // Compare against canonical scanned paths (resolves symlinks and
            // `.`/`..` consistently with the prefix match above).
            let canonical = std::path::Path::new(&file_path)
                .canonicalize()
                .map(|c| c.to_string_lossy().into_owned())
                .unwrap_or(file_path.clone());
            if present.contains(&canonical) {
                continue;
            }
            // Only delete when the OS confirms absence; on any I/O doubt,
            // keep the row (fail-open for user data, fail-closed for ghosts).
            match std::fs::metadata(&file_path) {
                Ok(_) => continue,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(_) => continue,
            }
            if self.remove_track_by_id(TrackId(track_id)).is_ok() {
                pruned += 1;
            }
        }
        Ok(pruned)
    }

    /// Delete a track row plus its FTS index entry and cached lyrics.
    pub fn remove_track_by_id(&self, track_id: TrackId) -> Result<()> {
        let conn = self.db.lock_conn()?;
        let file_path: Option<String> = conn
            .query_row(
                "SELECT file_path FROM tracks WHERE id = ?1",
                params![track_id.0],
                |row| row.get(0),
            )
            .ok();
        conn.execute(
            "DELETE FROM fts_tracks WHERE track_id = ?1",
            params![track_id.0],
        )
        .map_err(|e| SonoraError::Database(e.to_string()))?;
        conn.execute(
            "DELETE FROM lyrics_cache WHERE track_id = ?1",
            params![track_id.0],
        )
        .map_err(|e| SonoraError::Database(e.to_string()))?;
        if let Some(fp) = file_path {
            conn.execute("DELETE FROM lyrics_cache WHERE file_path = ?1", params![fp])
                .map_err(|e| SonoraError::Database(e.to_string()))?;
        }
        conn.execute("DELETE FROM tracks WHERE id = ?1", params![track_id.0])
            .map_err(|e| SonoraError::Database(e.to_string()))?;
        Ok(())
    }

    fn collect_audio_files(
        dir: &Path,
        supported: &HashSet<&str>,
        collected: &mut Vec<PathBuf>,
        visited_dirs: &mut HashSet<PathBuf>,
    ) -> Result<()> {
        if !dir.exists() {
            return Ok(());
        }

        // Canonicalize to resolve symlinks: revisiting a directory means a
        // symlink cycle, which would otherwise recurse until stack overflow.
        let canonical = match dir.canonicalize() {
            Ok(c) => c,
            Err(_) => return Ok(()),
        };
        if !visited_dirs.insert(canonical) {
            return Ok(());
        }

        let entries = std::fs::read_dir(dir).map_err(SonoraError::Io)?;
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_dir() {
                let _ = Self::collect_audio_files(&path, supported, collected, visited_dirs);
            } else if path.is_file() {
                if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                    if supported.contains(&ext.to_lowercase().as_str()) {
                        collected.push(path);
                    }
                }
            }
        }
        Ok(())
    }

    /// Retrieve cached lyrics for a track, by track_id, file_path, or title & artist.
    pub fn get_cached_lyrics(
        &self,
        track_id: Option<i64>,
        file_path: Option<&str>,
        title: &str,
        artist: Option<&str>,
    ) -> Result<Option<sonora_lyrics::model::LyricsDocument>> {
        let conn = self.db.lock_conn()?;

        let mut row_opt = None;

        if let Some(tid) = track_id {
            let mut stmt = conn
                .prepare(
                    "SELECT content_json, offset_ms FROM lyrics_cache WHERE track_id = ?1 ORDER BY id DESC LIMIT 1",
                )
                .map_err(|e| SonoraError::Database(e.to_string()))?;
            let mut rows = stmt
                .query(params![tid])
                .map_err(|e| SonoraError::Database(e.to_string()))?;
            if let Some(row) = rows
                .next()
                .map_err(|e| SonoraError::Database(e.to_string()))?
            {
                let json: String = row
                    .get(0)
                    .map_err(|e| SonoraError::Database(e.to_string()))?;
                let offset: i64 = row
                    .get(1)
                    .map_err(|e| SonoraError::Database(e.to_string()))?;
                row_opt = Some((json, offset));
            }
        }

        if row_opt.is_none() {
            if let Some(fp) = file_path {
                let mut stmt = conn
                    .prepare(
                        "SELECT content_json, offset_ms FROM lyrics_cache WHERE file_path = ?1 ORDER BY id DESC LIMIT 1",
                    )
                    .map_err(|e| SonoraError::Database(e.to_string()))?;
                let mut rows = stmt
                    .query(params![fp])
                    .map_err(|e| SonoraError::Database(e.to_string()))?;
                if let Some(row) = rows
                    .next()
                    .map_err(|e| SonoraError::Database(e.to_string()))?
                {
                    let json: String = row
                        .get(0)
                        .map_err(|e| SonoraError::Database(e.to_string()))?;
                    let offset: i64 = row
                        .get(1)
                        .map_err(|e| SonoraError::Database(e.to_string()))?;
                    row_opt = Some((json, offset));
                }
            }
        }

        if row_opt.is_none() {
            let mut stmt = conn
                .prepare(
                    "SELECT content_json, offset_ms FROM lyrics_cache WHERE title = ?1 COLLATE NOCASE AND (artist = ?2 COLLATE NOCASE OR (?2 IS NULL AND artist IS NULL)) ORDER BY id DESC LIMIT 1",
                )
                .map_err(|e| SonoraError::Database(e.to_string()))?;
            let mut rows = stmt
                .query(params![title, artist])
                .map_err(|e| SonoraError::Database(e.to_string()))?;
            if let Some(row) = rows
                .next()
                .map_err(|e| SonoraError::Database(e.to_string()))?
            {
                let json: String = row
                    .get(0)
                    .map_err(|e| SonoraError::Database(e.to_string()))?;
                let offset: i64 = row
                    .get(1)
                    .map_err(|e| SonoraError::Database(e.to_string()))?;
                row_opt = Some((json, offset));
            }
        }

        if let Some((json, offset)) = row_opt {
            let mut doc: sonora_lyrics::model::LyricsDocument = serde_json::from_str(&json)
                .map_err(|e| {
                    SonoraError::Lyrics(format!("Failed to deserialize cached lyrics: {}", e))
                })?;
            doc.offset_ms = offset;
            Ok(Some(doc))
        } else {
            Ok(None)
        }
    }

    /// Save lyrics to SQLite cache.
    pub fn save_cached_lyrics(
        &self,
        track_id: Option<i64>,
        file_path: Option<&str>,
        title: &str,
        artist: Option<&str>,
        doc: &sonora_lyrics::model::LyricsDocument,
        provider: &str,
    ) -> Result<()> {
        let conn = self.db.lock_conn()?;
        let json = serde_json::to_string(doc)
            .map_err(|e| SonoraError::Lyrics(format!("Failed to serialize lyrics: {}", e)))?;
        let is_synced = if doc.is_synced() { 1 } else { 0 };
        let format_str = match doc.format {
            sonora_lyrics::model::LyricsFormat::Plain => "Plain",
            sonora_lyrics::model::LyricsFormat::Lrc => "Lrc",
            sonora_lyrics::model::LyricsFormat::EnhancedLrc => "EnhancedLrc",
            sonora_lyrics::model::LyricsFormat::Ttml => "Ttml",
        };

        // Clean up prior entries for this track/file if present
        if let Some(tid) = track_id {
            let _ = conn.execute("DELETE FROM lyrics_cache WHERE track_id = ?1", params![tid]);
        } else if let Some(fp) = file_path {
            let _ = conn.execute("DELETE FROM lyrics_cache WHERE file_path = ?1", params![fp]);
        }

        conn.execute(
            r#"INSERT INTO lyrics_cache 
                (track_id, file_path, title, artist, is_synced, format, offset_ms, content_json, provider)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)"#,
            params![
                track_id,
                file_path,
                title,
                artist,
                is_synced,
                format_str,
                doc.offset_ms,
                json,
                provider
            ],
        )
        .map_err(|e| SonoraError::Database(e.to_string()))?;

        Ok(())
    }

    /// Update the manual offset for cached lyrics.
    pub fn update_lyrics_offset(
        &self,
        track_id: Option<i64>,
        file_path: Option<&str>,
        offset_ms: i64,
    ) -> Result<()> {
        let conn = self.db.lock_conn()?;

        if let Some(tid) = track_id {
            conn.execute(
                "UPDATE lyrics_cache SET offset_ms = ?1 WHERE track_id = ?2",
                params![offset_ms, tid],
            )
            .map_err(|e| SonoraError::Database(e.to_string()))?;
        } else if let Some(fp) = file_path {
            conn.execute(
                "UPDATE lyrics_cache SET offset_ms = ?1 WHERE file_path = ?2",
                params![offset_ms, fp],
            )
            .map_err(|e| SonoraError::Database(e.to_string()))?;
        }

        Ok(())
    }
}

fn to_base64(data: &[u8]) -> String {
    const CHARSET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as usize;
        let b1 = chunk.get(1).copied().unwrap_or(0) as usize;
        let b2 = chunk.get(2).copied().unwrap_or(0) as usize;
        let triple = (b0 << 16) | (b1 << 8) | b2;
        out.push(CHARSET[(triple >> 18) & 0x3F] as char);
        out.push(CHARSET[(triple >> 12) & 0x3F] as char);
        if chunk.len() > 1 {
            out.push(CHARSET[(triple >> 6) & 0x3F] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(CHARSET[triple & 0x3F] as char);
        } else {
            out.push('=');
        }
    }
    out
}
