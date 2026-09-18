use serde::{Deserialize, Serialize};
use sonora_common::{PlaybackState, TrackId};

/// System-wide broadcast events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SonoraEvent {
    PlaybackStateChanged(PlaybackState),
    TrackChanged {
        track_id: TrackId,
        title: String,
        artist: Option<String>,
        duration_ms: u64,
    },
    VolumeChanged {
        volume: f32,
        muted: bool,
    },
    LibraryScanProgress {
        indexed_count: usize,
        total_count: usize,
    },
    Error(String),
}
