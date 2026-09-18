use serde::{Deserialize, Serialize};
use sonora_common::{PlaybackState, TrackId};

/// System-wide broadcast events.
///
/// Subsystems (library, playback, queue, lyrics, plugins) communicate through
/// these events instead of reaching into each other's state: a service
/// performs its own mutation, then publishes the corresponding event for
/// observers (UI clients, plugin host, diagnostics).
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

    // --- Queue service boundary ---
    QueueChanged {
        queue_length: usize,
        current_index: Option<usize>,
    },

    // --- Library service boundary ---
    LibraryScanStarted {
        path: String,
    },
    LibraryScanCompleted {
        indexed_tracks: usize,
        scanned_files: usize,
    },

    // --- Lyrics service boundary ---
    LyricsResolved {
        title: String,
        provider: String,
    },
    LyricsResolutionFailed {
        title: String,
    },

    // --- Plugin host boundary ---
    PluginStateChanged {
        plugin_id: String,
        from: String,
        to: String,
    },
}
