use serde::{Deserialize, Serialize};

/// Unique identifier for an audio track.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TrackId(pub i64);

impl From<i64> for TrackId {
    fn from(val: i64) -> Self {
        TrackId(val)
    }
}

/// Unique identifier for an artist.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ArtistId(pub i64);

impl From<i64> for ArtistId {
    fn from(val: i64) -> Self {
        ArtistId(val)
    }
}

/// Unique identifier for an album.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct AlbumId(pub i64);

impl From<i64> for AlbumId {
    fn from(val: i64) -> Self {
        AlbumId(val)
    }
}

/// Unique identifier for a playlist.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PlaylistId(pub i64);

impl From<i64> for PlaylistId {
    fn from(val: i64) -> Self {
        PlaylistId(val)
    }
}

/// Audio playback state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum PlaybackState {
    #[default]
    Stopped,
    Playing,
    Paused,
}
