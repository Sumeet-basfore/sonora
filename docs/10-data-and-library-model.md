# Sonora: Data Model, Library Indexing & Storage Architecture

## 1. Storage Architecture & Engine Selection

Sonora uses an embedded **SQLite** database configured in Write-Ahead Logging (WAL) mode paired with the **FTS5 (Full-Text Search)** extension. This architecture delivers sub-5ms query performance across 100,000+ tracks while ensuring zero lock contention between background file indexing and frontend UI rendering.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           STORAGE ENGINE TOPOLOGY                           │
│                                                                             │
│   ┌─────────────────────────────────────────────────────────────────────┐   │
│   │                         SQLite Storage Core                         │   │
│   │  • PRAGMA journal_mode = WAL (Write-Ahead Logging)                  │   │
│   │  • PRAGMA synchronous = NORMAL (Optimal SSD throughput)             │   │
│   │  • PRAGMA temp_store = MEMORY                                       │   │
│   │  • PRAGMA cache_size = -64000 (64MB dedicated query cache)          │   │
│   └──────────────────────────────────┬──────────────────────────────────┘   │
│                                      │                                       │
│                ┌─────────────────────┴─────────────────────┐                │
│                ▼                                           ▼                │
│   ┌───────────────────────────┐               ┌───────────────────────────┐ │
│   │    Relational DB Tables   │               │     FTS5 Search Engine    │ │
│   │ • Tracks, Albums, Artists │               │ • Sub-3ms Fuzzy Search    │ │
│   │ • Playlists, Play History │               │ • BM25 Relevance Scoring  │ │
│   │ • Lyrics Cache & EQ Presets│              │ • Trigram / Prefix Match  │ │
│   └───────────────────────────┘               └───────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 2. Comprehensive Relational Database Schema (DDL)

```sql
-- 1. Artists Table
CREATE TABLE artists (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL COLLATE NOCASE,
    sort_name TEXT COLLATE NOCASE,
    musicbrainz_id TEXT UNIQUE,
    bio TEXT,
    image_uri TEXT,
    created_at INTEGER NOT NULL DEFAULT (unixepoch())
);
CREATE INDEX idx_artists_name ON artists(name);

-- 2. Albums Table
CREATE TABLE albums (
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
    created_at INTEGER NOT NULL DEFAULT (unixepoch())
);
CREATE INDEX idx_albums_title ON albums(title);
CREATE INDEX idx_albums_artist ON albums(artist_id);

-- 3. Tracks Table
CREATE TABLE tracks (
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
CREATE INDEX idx_tracks_album ON tracks(album_id);
CREATE INDEX idx_tracks_artist ON tracks(artist_id);
CREATE INDEX idx_tracks_play_count ON tracks(play_count DESC);

-- 4. Genres & Track-Genre Junction Table
CREATE TABLE genres (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE COLLATE NOCASE
);

CREATE TABLE track_genres (
    track_id INTEGER REFERENCES tracks(id) ON DELETE CASCADE,
    genre_id INTEGER REFERENCES genres(id) ON DELETE CASCADE,
    PRIMARY KEY (track_id, genre_id)
);

-- 5. Playlists & Playlist Items
CREATE TABLE playlists (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    description TEXT,
    is_smart INTEGER NOT NULL DEFAULT 0,
    smart_rules_json TEXT,
    cover_art_uri TEXT,
    created_at INTEGER NOT NULL DEFAULT (unixepoch()),
    updated_at INTEGER NOT NULL DEFAULT (unixepoch())
);

CREATE TABLE playlist_tracks (
    playlist_id INTEGER REFERENCES playlists(id) ON DELETE CASCADE,
    track_id INTEGER REFERENCES tracks(id) ON DELETE CASCADE,
    position INTEGER NOT NULL,
    added_at INTEGER NOT NULL DEFAULT (unixepoch()),
    PRIMARY KEY (playlist_id, position)
);

-- 6. Playback History & Analytics
CREATE TABLE play_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    track_id INTEGER REFERENCES tracks(id) ON DELETE CASCADE,
    started_at INTEGER NOT NULL DEFAULT (unixepoch()),
    duration_played_ms INTEGER NOT NULL,
    completed INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX idx_history_track ON play_history(track_id);

-- 7. Lyrics Cache Table
CREATE TABLE lyrics_cache (
    track_id INTEGER PRIMARY KEY REFERENCES tracks(id) ON DELETE CASCADE,
    format TEXT NOT NULL, -- 'plain', 'lrc', 'enhanced_lrc', 'ttml'
    offset_ms INTEGER NOT NULL DEFAULT 0,
    content TEXT NOT NULL,
    source TEXT NOT NULL,
    updated_at INTEGER NOT NULL DEFAULT (unixepoch())
);

-- 8. Parametric EQ Presets
CREATE TABLE eq_presets (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    bands_json TEXT NOT NULL,
    preamp_db REAL NOT NULL DEFAULT 0.0,
    is_autoeq INTEGER NOT NULL DEFAULT 0,
    headphone_model TEXT,
    created_at INTEGER NOT NULL DEFAULT (unixepoch())
);

-- 9. FTS5 Full-Text Search Virtual Table
CREATE VIRTUAL TABLE fts_tracks USING fts5(
    track_id UNINDEXED,
    title,
    artist_name,
    album_title,
    genre_names,
    tokenize = 'trigram'
);
```

---

## 3. Library Indexing & Filesystem Watching Pipeline

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         LIBRARY SCANNING ARCHITECTURE                       │
│                                                                             │
│  Watched Folders ──► [Filesystem Watcher (notify)] ──► [Debounce Queue]     │
│                                                              │              │
│                                                              ▼              │
│  [Background Worker Pool] ◄── [File Discovery & Modified Time Comparison]   │
│            │                                                                │
│            ▼                                                                │
│  [Tag Extraction Engine (Lofty / Symphonia)]                                │
│  • ID3v2.3/2.4, Vorbis, MP4 Atoms, APE, FLAC Metadata Blocks                │
│  • Embedded Cover Art Extraction & WebP Thumbnail Generation                │
│  • ReplayGain / EBU R128 Metadata Parsing                                   │
│            │                                                                │
│            ▼                                                                │
│  [SQLite Transaction Batching] (1,000 Tracks per Transaction)               │
│            │                                                                │
│            ▼                                                                │
│  [FTS5 Index Sync] ──► [Broadcast IPC LibraryProgress Event]                │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 3.1 Scanning Performance Optimization
- **Mtime & Size Filtering**: Files are skipped if `mtime` and `file_size` match database records, allowing 100k-track re-verification in < 2 seconds.
- **Batch Transactions**: SQLite inserts and updates are committed in batches of 1,000 records to maximize I/O throughput.
- **Pure-Rust Non-Blocking Parser**: Metadata parsing uses the memory-safe `lofty` / `symphonia` library on worker threads without invoking external CLI tools or blocking the UI.

---

## 4. Smart Playlist Query Engine

Smart playlists allow users to define rules evaluated dynamically via SQL view compilation:

```json
{
  "combinator": "AND",
  "rules": [
    { "field": "genres.name", "operator": "equals", "value": "Jazz" },
    { "field": "tracks.rating", "operator": "gte", "value": 4 },
    { "field": "albums.release_year", "operator": "between", "value": [1950, 1965] },
    { "field": "tracks.play_count", "operator": "gt", "value": 5 }
  ],
  "sortBy": "tracks.play_count",
  "sortOrder": "DESC",
  "limit": 50
}
```

This DSL compiles into parameterized SQL queries with safety verification against SQL injection.

---

## 5. Caching & Media Asset Storage Architecture

```
$XDG_CACHE_HOME/sonora/ (or %LOCALAPPDATA%\Sonora\Cache\)
├── covers/
│   ├── thumbnails/     # 300x300 WebP compressed cover thumbnails
│   └── full/           # Original uncompressed extracted cover art
├── waveforms/          # Pre-computed 256-point RMS waveform peak cache
└── plugins/            # Sandboxed plugin KV storage partitions
```

---

## 6. Reversibility & Storage Boundaries

> [!IMPORTANT]
> ### Reversible Architecture Decisions
> - **Search Backend**: FTS5 can be swapped for Tantivy (Rust full-text indexer) via the `SearchIndexProvider` interface if phonetic search needs expand.
> - **Image Thumbnail Format**: WebP thumbnail caching can switch to AVIF or standard JPEG without altering the database URI column format.
>
> ### Invariable Boundaries
> - **Single Source of Truth**: The local SQLite database is the canonical state store for all track metadata, playback history, and smart playlists.
