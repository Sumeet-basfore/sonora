//! Sonora third-party plugin foundation (v1, WASM-only).
//!
//! This crate provides the stable, minimal surface third-party plugins are
//! built against:
//!
//! - [`manifest`] — `manifest.json` v1 parsing and validation. Validation runs
//!   **before** any code is loaded.
//! - [`capability`] — the closed capability set and enforcement helpers.
//! - [`api`] — host-side plugin traits (`LyricsProvider`, `MetadataProvider`,
//!   `Visualizer`, `UIWidget`) plus the JSON DTOs exchanged over the WASM ABI.
//! - [`wasm`] — the Wasmtime sandbox: one `Store` per plugin instance, fuel
//!   metering, a 32 MiB memory cap, and a strict host-function boundary.
//! - [`host`] — the lifecycle state machine (`discover → validate → load →
//!   start → stop → unload`) with crash isolation and restart backoff.
//!
//! Explicitly out of scope for v1 (see crate docs and `plugins/example/README.md`):
//! marketplace/registry, paid plugins, QuickJS, projectM, DSP plugins, and
//! arbitrary native (`.so`/`.dll`) plugins.

pub mod api;
pub mod capability;
pub mod host;
pub mod manifest;
pub mod net;
pub mod wasm;

pub use api::{
    LyricsQueryDto, LyricsResultDto, MetadataQueryDto, PluginLyricsProvider, PluginMetadata,
    PluginMetadataProvider, PluginVisualizer, PluginWidget, VisualizerDescriptor, WidgetDescriptor,
};
pub use capability::Capability;
pub use host::{PluginHost, PluginInfo, PluginRecord, PluginState};
pub use manifest::{validate_plugin_id, PluginManifest, PLUGIN_MANIFEST_FILENAME};
pub use net::{
    FakeNetworkTransport, FakeOutcome, NetworkError, NetworkTransport, RealNetworkTransport,
};
pub use wasm::{
    WasmPluginInstance, NET_STATUS_NONE, NET_STATUS_TIMEOUT, NET_STATUS_TOO_LARGE,
    NET_STATUS_TRANSPORT,
};

/// Current plugin API version. WASM modules must export `plugin_api_version`
/// returning exactly this value or load is rejected.
pub const PLUGIN_API_VERSION: u32 = 1;

/// Fuel budget per single host→plugin call. Bounds CPU so an infinite loop
/// traps instead of hanging the host.
///
/// Sized from measurement, not guesswork: a tight scan loop costs
/// low-double-digit fuel per input byte at `opt-level="s"`, and real song
/// pages reach ~600 KiB with thousands of tags — a 606 KiB Genius page needs
/// just under 12M units. 16M leaves headroom for larger pages while still
/// trapping runaway plugins almost immediately; anything beyond degrades to
/// a provider miss, never a hang. This is a host-side bound, not plugin ABI:
/// plugins cannot observe it except by trapping.
pub const PLUGIN_CALL_FUEL: u64 = 16_000_000;

/// Maximum linear memory per plugin instance (32 MiB, ADR-005 quota).
pub const PLUGIN_MAX_MEMORY_BYTES: u64 = 32 * 1024 * 1024;

/// Maximum bytes a single plugin may keep in the host-side KV cache.
pub const PLUGIN_MAX_CACHE_BYTES: usize = 1024 * 1024;

/// Timeout for one `sonora.net_fetch` call (seconds).
pub const PLUGIN_NET_TIMEOUT_SECS: u64 = 5;

/// Maximum response body accepted from `sonora.net_fetch` (bytes).
///
/// Sized for real-world song pages (Genius pages run ~600 KiB): still a hard
/// bound, still checked before the bytes reach the sandbox. This is a
/// host-side bound, not plugin ABI.
pub const PLUGIN_NET_MAX_BYTES: usize = 2 * 1024 * 1024;

/// Consecutive crashes within the backoff window before a plugin is disabled.
pub const PLUGIN_MAX_CONSECUTIVE_CRASHES: u32 = 3;

/// Window for counting consecutive crashes (seconds).
pub const PLUGIN_CRASH_WINDOW_SECS: u64 = 60;

/// Error type for plugin operations. Converts into [`sonora_common::SonoraError::Plugin`].
#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    #[error("invalid manifest: {0}")]
    InvalidManifest(String),
    #[error("capability denied: {0}")]
    CapabilityDenied(String),
    #[error("plugin load failed: {0}")]
    Load(String),
    #[error("plugin call failed: {0}")]
    Call(String),
    #[error("plugin crashed: {0}")]
    Crashed(String),
    #[error("invalid plugin state for {op}: expected {expected}, was {actual}")]
    BadState {
        op: &'static str,
        expected: &'static str,
        actual: &'static str,
    },
    #[error("I/O error: {0}")]
    Io(String),
}

impl From<PluginError> for sonora_common::SonoraError {
    fn from(e: PluginError) -> Self {
        Self::Plugin(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, PluginError>;
