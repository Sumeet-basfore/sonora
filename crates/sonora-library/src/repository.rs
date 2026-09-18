use crate::db::Database;
use crate::models::SearchResult;
use rusqlite::params;
use sonora_common::{Result, SonoraError, TrackId};

/// Repository handling data persistence and FTS5 search queries.
pub struct LibraryRepository<'a> {
    db: &'a Database,
}

impl<'a> LibraryRepository<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// Insert or replace an artist returning the artist ID.
    pub fn upsert_artist(&self, name: &str) -> Result<i64> {
        let conn = self.db.lock_conn()?;
        conn.execute(
            "INSERT INTO artists (name) VALUES (?1) ON CONFLICT DO NOTHING",
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

    /// Insert or replace an album returning the album ID.
    pub fn upsert_album(&self, title: &str, artist_id: Option<i64>, year: Option<i32>) -> Result<i64> {
        let conn = self.db.lock_conn()?;
        conn.execute(
            "INSERT INTO albums (title, artist_id, release_year) VALUES (?1, ?2, ?3)",
            params![title, artist_id, year],
        )
        .map_err(|e| SonoraError::Database(e.to_string()))?;

        let id = conn.last_insert_rowid();
        Ok(id)
    }

    /// Insert a track into the library and index into FTS5.
    pub fn insert_track(
        &self,
        file_path: &str,
        file_hash: &str,
        file_size_bytes: i64,
        file_modified_time: i64,
        title: &str,
        artist_id: Option<i64>,
        album_id: Option<i64>,
        duration_ms: i64,
        sample_rate: i32,
        channels: i32,
        codec: &str,
    ) -> Result<TrackId> {
        let conn = self.db.lock_conn()?;
        conn.execute(
            r#"
            INSERT INTO tracks (
                file_path, file_hash, file_size_bytes, file_modified_time,
                title, artist_id, album_id, duration_ms, sample_rate, channels, codec
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
            "#,
            params![
                file_path,
                file_hash,
                file_size_bytes,
                file_modified_time,
                title,
                artist_id,
                album_id,
                duration_ms,
                sample_rate,
                channels,
                codec
            ],
        )
        .map_err(|e| SonoraError::Database(e.to_string()))?;

        let track_id = conn.last_insert_rowid();

        // Index in FTS5
        let artist_name: Option<String> = artist_id.and_then(|id| {
            conn.query_row("SELECT name FROM artists WHERE id = ?1", params![id], |r| r.get(0)).ok()
        });
        let album_title: Option<String> = album_id.and_then(|id| {
            conn.query_row("SELECT title FROM albums WHERE id = ?1", params![id], |r| r.get(0)).ok()
        });

        conn.execute(
            "INSERT INTO tracks_fts (rowid, title, artist_name, album_title) VALUES (?1, ?2, ?3, ?4)",
            params![track_id, title, artist_name, album_title],
        )
        .map_err(|e| SonoraError::Database(format!("FTS5 index error: {e}")))?;

        Ok(TrackId(track_id))
    }

    /// Search tracks using FTS5 trigram full-text query.
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        let conn = self.db.lock_conn()?;
        let mut stmt = conn
            .prepare(
                r#"
                SELECT t.id, t.title, ar.name, al.title, t.duration_ms
                FROM tracks_fts fts
                JOIN tracks t ON fts.rowid = t.id
                LEFT JOIN artists ar ON t.artist_id = ar.id
                LEFT JOIN albums al ON t.album_id = al.id
                WHERE tracks_fts MATCH ?1
                ORDER BY rank
                LIMIT ?2
                "#,
            )
            .map_err(|e| SonoraError::Database(format!("Prepare search query error: {e}")))?;

        let results = stmt
            .query_map(params![query, limit as i64], |row| {
                Ok(SearchResult {
                    track_id: TrackId(row.get(0)?),
                    title: row.get(1)?,
                    artist_name: row.get(2)?,
                    album_title: row.get(3)?,
                    duration_ms: row.get(4)?,
                })
            })
            .map_err(|e| SonoraError::Database(format!("FTS5 query error: {e}")))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(results)
    }
}
