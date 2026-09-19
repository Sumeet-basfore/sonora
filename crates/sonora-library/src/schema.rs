pub const SCHEMA_SQL: &str = r#"
-- 1. Artists Table
CREATE TABLE IF NOT EXISTS artists (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE COLLATE NOCASE,
    sort_name TEXT COLLATE NOCASE,
    musicbrainz_id TEXT UNIQUE,
    bio TEXT,
    image_uri TEXT,
    created_at INTEGER NOT NULL DEFAULT (unixepoch())
);
CREATE INDEX IF NOT EXISTS idx_artists_name ON artists(name);

-- 2. Albums Table
CREATE TABLE IF NOT EXISTS albums (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT NOT NULL COLLATE NOCASE,
    sort_title TEXT COLLATE NOCASE,
    artist_id INTEGER REFERENCES artists(id) ON DELETE SET NULL,
    release_year INTEGER,
    total_tracks INTEGER,
    total_discs INTEGER DEFAULT 1,
    musicbrainz_id TEXT UNIQUE,
    cover_art_uri TEXT,
    dominant_color TEXT,
    vibrant_color TEXT,
    created_at INTEGER NOT NULL DEFAULT (unixepoch()),
    UNIQUE(title COLLATE NOCASE, artist_id)
);
CREATE INDEX IF NOT EXISTS idx_albums_title ON albums(title);
CREATE INDEX IF NOT EXISTS idx_albums_artist ON albums(artist_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_albums_title_null_artist ON albums(title COLLATE NOCASE) WHERE artist_id IS NULL;

-- 3. Tracks Table
CREATE TABLE IF NOT EXISTS tracks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    file_path TEXT NOT NULL UNIQUE,
    file_hash TEXT NOT NULL,
    file_size_bytes INTEGER NOT NULL,
    file_modified_time INTEGER NOT NULL,
    title TEXT NOT NULL COLLATE NOCASE,
    artist_id INTEGER REFERENCES artists(id) ON DELETE SET NULL,
    album_id INTEGER REFERENCES albums(id) ON DELETE CASCADE,
    album_artist_id INTEGER REFERENCES artists(id) ON DELETE SET NULL,
    track_number INTEGER,
    disc_number INTEGER DEFAULT 1,
    duration_ms INTEGER NOT NULL,
    sample_rate INTEGER NOT NULL,
    bit_depth INTEGER,
    channels INTEGER NOT NULL,
    bitrate_kbps INTEGER,
    codec TEXT NOT NULL,
    replaygain_track_gain REAL,
    replaygain_track_peak REAL,
    replaygain_album_gain REAL,
    replaygain_album_peak REAL,
    play_count INTEGER NOT NULL DEFAULT 0,
    last_played_at INTEGER,
    rating INTEGER CHECK(rating BETWEEN 0 AND 5) DEFAULT 0,
    added_at INTEGER NOT NULL DEFAULT (unixepoch())
);
CREATE INDEX IF NOT EXISTS idx_tracks_album ON tracks(album_id);
CREATE INDEX IF NOT EXISTS idx_tracks_artist ON tracks(artist_id);
CREATE INDEX IF NOT EXISTS idx_tracks_play_count ON tracks(play_count DESC);
CREATE INDEX IF NOT EXISTS idx_tracks_mtime_size ON tracks(file_path, file_modified_time, file_size_bytes);

-- 4. Playlists & Playlist Tracks
CREATE TABLE IF NOT EXISTS playlists (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    description TEXT,
    is_smart INTEGER NOT NULL DEFAULT 0,
    smart_rules_json TEXT,
    cover_art_uri TEXT,
    created_at INTEGER NOT NULL DEFAULT (unixepoch()),
    updated_at INTEGER NOT NULL DEFAULT (unixepoch())
);

CREATE TABLE IF NOT EXISTS playlist_tracks (
    playlist_id INTEGER REFERENCES playlists(id) ON DELETE CASCADE,
    track_id INTEGER REFERENCES tracks(id) ON DELETE CASCADE,
    position INTEGER NOT NULL,
    added_at INTEGER NOT NULL DEFAULT (unixepoch()),
    PRIMARY KEY (playlist_id, position)
);

-- 5. FTS5 Trigram Full-Text Search Table
CREATE VIRTUAL TABLE IF NOT EXISTS fts_tracks USING fts5(
    track_id UNINDEXED,
    title,
    artist_name,
    album_title,
    genre_names,
    tokenize = 'trigram'
);

-- 6. Lyrics Cache Table
CREATE TABLE IF NOT EXISTS lyrics_cache (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    track_id INTEGER REFERENCES tracks(id) ON DELETE CASCADE,
    file_path TEXT,
    title TEXT NOT NULL COLLATE NOCASE,
    artist TEXT COLLATE NOCASE,
    is_synced INTEGER NOT NULL DEFAULT 0,
    format TEXT NOT NULL,
    offset_ms INTEGER NOT NULL DEFAULT 0,
    content_json TEXT NOT NULL,
    provider TEXT NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (unixepoch())
);
CREATE INDEX IF NOT EXISTS idx_lyrics_cache_track ON lyrics_cache(track_id);
CREATE INDEX IF NOT EXISTS idx_lyrics_cache_title_artist ON lyrics_cache(title COLLATE NOCASE, artist COLLATE NOCASE);
CREATE INDEX IF NOT EXISTS idx_lyrics_cache_file_path ON lyrics_cache(file_path);

-- 7. Online Metadata Entity Cache (MusicBrainz entities, lookups, searches)
CREATE TABLE IF NOT EXISTS online_metadata_cache (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    cache_key TEXT NOT NULL UNIQUE,
    entity_type TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    provider TEXT NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (unixepoch()),
    expires_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_online_metadata_cache_key ON online_metadata_cache(cache_key);
CREATE INDEX IF NOT EXISTS idx_online_metadata_cache_expires ON online_metadata_cache(expires_at);

-- 8. Metadata Ranked Candidates Cache (Track / Album match query results)
CREATE TABLE IF NOT EXISTS metadata_candidates_cache (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    query_fingerprint TEXT NOT NULL UNIQUE,
    candidates_json TEXT NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (unixepoch()),
    expires_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_metadata_candidates_fingerprint ON metadata_candidates_cache(query_fingerprint);

-- 9. Online Artwork Candidates Cache (Cover Art Archive / Wikidata / Fanart.tv)
CREATE TABLE IF NOT EXISTS online_artwork_cache (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    cache_key TEXT NOT NULL UNIQUE,
    entity_type TEXT NOT NULL,
    candidates_json TEXT NOT NULL,
    provider TEXT NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (unixepoch()),
    expires_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_online_artwork_cache_key ON online_artwork_cache(cache_key);

-- 10. Lyrics Candidates Cache (Multi-candidate search results)
CREATE TABLE IF NOT EXISTS lyrics_candidates_cache (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    query_fingerprint TEXT NOT NULL UNIQUE,
    candidates_json TEXT NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (unixepoch()),
    expires_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_lyrics_candidates_fingerprint ON lyrics_candidates_cache(query_fingerprint);
"#;
