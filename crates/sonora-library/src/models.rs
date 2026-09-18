use serde::{Deserialize, Serialize};
use sonora_common::{AlbumId, ArtistId, TrackId};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Track {
    pub id: TrackId,
    pub file_path: String,
    pub file_hash: String,
    pub file_size_bytes: i64,
    pub file_modified_time: i64,
    pub title: String,
    pub artist_id: Option<ArtistId>,
    pub album_id: Option<AlbumId>,
    pub track_number: Option<i32>,
    pub disc_number: i32,
    pub duration_ms: i64,
    pub sample_rate: i32,
    pub bit_depth: Option<i32>,
    pub channels: i32,
    pub bitrate_kbps: Option<i32>,
    pub codec: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artist {
    pub id: ArtistId,
    pub name: String,
    pub sort_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Album {
    pub id: AlbumId,
    pub title: String,
    pub artist_id: Option<ArtistId>,
    pub release_year: Option<i32>,
    pub total_tracks: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub track_id: TrackId,
    pub title: String,
    pub artist_name: Option<String>,
    pub album_title: Option<String>,
    pub duration_ms: i64,
}
