//! Host-side plugin API: traits and JSON DTOs.
//!
//! The traits are intentionally small — a plugin author implements one or more
//! of [`PluginLyricsProvider`], [`PluginMetadataProvider`],
//! [`PluginVisualizer`], [`PluginWidget`]. The WASM ABI exchanges the DTOs
//! below as JSON through plugin linear memory; native (in-process test/Rust)
//! plugins implement the same traits directly, so both paths share review,
//! tests, and capability gating.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Lyrics
// ---------------------------------------------------------------------------

/// Lyrics lookup request. Mirrors the host lyrics resolver query shape.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LyricsQueryDto {
    pub title: String,
    #[serde(default)]
    pub artist: Option<String>,
    #[serde(default)]
    pub album: Option<String>,
    #[serde(default)]
    pub duration_ms: Option<u64>,
}

/// Lyrics answer. `content` carries raw payload text (LRC text for `lrc` /
/// `enhanced_lrc` / `ttml`, prose for `plain`); the host parses it with the
/// lyrics parsers so plugins never touch host parsing internals.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LyricsResultDto {
    /// One of `plain`, `lrc`, `enhanced_lrc`, `ttml`.
    pub format: String,
    pub content: String,
    #[serde(default)]
    pub attribution: Option<String>,
}

/// Host-side interface for lyrics providers (built-in or plugin-backed).
pub trait PluginLyricsProvider: Send + Sync {
    /// Stable provider id, e.g. the plugin id or `"embedded"`.
    fn provider_id(&self) -> &str;
    fn fetch_lyrics(&self, query: &LyricsQueryDto) -> crate::Result<Option<LyricsResultDto>>;
}

// ---------------------------------------------------------------------------
// Metadata
// ---------------------------------------------------------------------------

/// Metadata lookup request (read-only; plugins never mutate the library).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MetadataQueryDto {
    pub title: String,
    #[serde(default)]
    pub artist: Option<String>,
    #[serde(default)]
    pub album: Option<String>,
}

/// Read-only metadata answer fragment.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PluginMetadata {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub artist: Option<String>,
    #[serde(default)]
    pub album: Option<String>,
    #[serde(default)]
    pub year: Option<i32>,
    #[serde(default)]
    pub genre: Option<String>,
    #[serde(default)]
    pub bio: Option<String>,
    #[serde(default)]
    pub art_url: Option<String>,
}

/// Host-side interface for metadata providers.
pub trait PluginMetadataProvider: Send + Sync {
    fn provider_id(&self) -> &str;
    fn fetch_metadata(&self, query: &MetadataQueryDto) -> crate::Result<Option<PluginMetadata>>;
}

// ---------------------------------------------------------------------------
// Visualizer
// ---------------------------------------------------------------------------

/// Static descriptor for a visualizer plugin. Rendering itself stays
/// client-side; the plugin only receives read-only audio taps.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualizerDescriptor {
    pub name: String,
    #[serde(default = "default_fps")]
    pub preferred_fps: u32,
    #[serde(default = "default_true")]
    pub uses_audio_tap: bool,
}

fn default_fps() -> u32 {
    60
}

fn default_true() -> bool {
    true
}

/// Host-side interface for visualizers.
pub trait PluginVisualizer: Send + Sync {
    fn plugin_id(&self) -> &str;
    fn describe(&self) -> crate::Result<VisualizerDescriptor>;
    /// Human-readable frame summary for headless verification (real rendering
    /// happens in GUI/TUI clients from the shared tap).
    fn frame_hint(&self, fft_bins: &[f32]) -> crate::Result<String>;
}

// ---------------------------------------------------------------------------
// UI widget
// ---------------------------------------------------------------------------

/// Declarative widget descriptor. Widgets render through sanitized host slots;
/// plugins never receive DOM access.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WidgetDescriptor {
    /// Slot id, e.g. `"sidebar"`, `"statusbar"`, `"now_playing"`.
    pub slot: String,
    pub title: String,
    #[serde(default)]
    pub version: String,
}

/// Host-side interface for UI widgets.
pub trait PluginWidget: Send + Sync {
    fn plugin_id(&self) -> &str;
    fn describe(&self) -> crate::Result<WidgetDescriptor>;
}
