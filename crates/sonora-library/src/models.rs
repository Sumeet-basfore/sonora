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
    pub file_path: String,
    pub title: String,
    pub artist_name: Option<String>,
    pub album_title: Option<String>,
    pub duration_ms: i64,
    #[serde(default)]
    pub track_number: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlbumDto {
    pub id: i64,
    pub title: String,
    pub artist_id: Option<i64>,
    pub artist_name: Option<String>,
    pub release_year: Option<i32>,
    pub track_count: usize,
    pub total_duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtistDto {
    pub id: i64,
    pub name: String,
    pub album_count: usize,
    pub track_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LibrarySummary {
    pub track_count: usize,
    pub album_count: usize,
    pub artist_count: usize,
}
