//! Plugin lifecycle host: `discover → validate → load → start → stop →
//! unload`, with crash isolation and restart backoff.
//!
//! Crash policy (ADR-005): a WASM trap is caught at the instance boundary,
//! the instance is dropped, and a [`PluginState::Crashed`] event is recorded.
//! The host retries with backoff (1s → 5s → 15s); after
//! [`crate::PLUGIN_MAX_CONSECUTIVE_CRASHES`] crashes inside
//! [`crate::PLUGIN_CRASH_WINDOW_SECS`] the plugin is [`PluginState::Disabled`]
//! until a manual `start`. A crash never propagates to the audio engine.

use crate::api::{LyricsQueryDto, LyricsResultDto, MetadataQueryDto, PluginMetadata};
use crate::manifest::{PluginManifest, PLUGIN_MANIFEST_FILENAME};
use crate::net::{NetworkTransport, RealNetworkTransport};
use crate::wasm::WasmPluginInstance;
use crate::{PluginError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Lifecycle state of a single plugin. Mirrors the state machine in
/// `docs/05-plugin-system.md` (minus the marketplace permission-prompt step,
//  which arrives with the registry UI).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginState {
    Discovered,
    Validated,
    Loaded,
    Running,
    Stopped,
    Crashed { reason: String },
    Unloaded,
    Disabled { reason: String },
}

impl PluginState {
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            PluginState::Discovered => "discovered",
            PluginState::Validated => "validated",
            PluginState::Running => "running",
            PluginState::Loaded => "loaded",
            PluginState::Stopped => "stopped",
            PluginState::Crashed { .. } => "crashed",
            PluginState::Unloaded => "unloaded",
            PluginState::Disabled { .. } => "disabled",
        }
    }
}

/// Host-observable lifecycle transitions, forwarded to the core event bus.
#[derive(Debug, Clone)]
pub struct PluginLifecycleEvent {
    pub plugin_id: String,
    pub from: &'static str,
    pub to: &'static str,
}

/// One managed plugin: manifest, optional live instance, crash bookkeeping.
pub struct PluginRecord {
    pub manifest: PluginManifest,
    pub dir: PathBuf,
    pub state: PluginState,
    pub instance: Option<WasmPluginInstance>,
    crash_count: u32,
    first_crash_at: Option<Instant>,
}

impl PluginRecord {
    fn transition(&mut self, to: PluginState, events: &mut Vec<PluginLifecycleEvent>) {
        let from = self.state.name();
        self.state = to;
        events.push(PluginLifecycleEvent {
            plugin_id: self.manifest.id.clone(),
            from,
            to: self.state.name(),
        });
    }
}

fn backoff_for_crash(count: u32) -> Duration {
    match count {
        0 => Duration::from_secs(0),
        1 => Duration::from_secs(1),
        2 => Duration::from_secs(5),
        _ => Duration::from_secs(15),
    }
}

/// Serializable plugin summary for UIs and diagnostics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub state: String,
    pub capabilities: Vec<String>,
}

/// Lifecycle host. `Send + Sync` via interior mutability so services and the
/// event bus can share one `Arc<PluginHost>`.
pub struct PluginHost {
    engine: wasmtime::Engine,
    transport: Mutex<Arc<dyn NetworkTransport>>,
    inner: Mutex<HostInner>,
    events: Mutex<Vec<PluginLifecycleEvent>>,
}

struct HostInner {
    plugins: HashMap<String, PluginRecord>,
}

impl PluginHost {
    pub fn new() -> Result<Self> {
        let mut config = wasmtime::Config::new();
        config.consume_fuel(true);
        let engine = wasmtime::Engine::new(&config)
            .map_err(|e| PluginError::Load(format!("cannot create wasm engine: {e}")))?;
        let transport = RealNetworkTransport::new();
        Ok(Self {
            engine,
            transport: Mutex::new(Arc::new(transport)),
            inner: Mutex::new(HostInner {
                plugins: HashMap::new(),
            }),
            events: Mutex::new(Vec::new()),
        })
    }

    /// Replace the network transport used for subsequently loaded instances
    /// (tests and plugin developers inject [`crate::FakeNetworkTransport`]).
    /// Already-running instances keep their previous transport.
    pub fn set_network_transport(&self, transport: Arc<dyn NetworkTransport>) {
        *self.transport.lock().unwrap_or_else(|e| e.into_inner()) = transport;
    }

    /// Drain lifecycle events since the last call (core forwards these to the
    /// `SonoraEvent` bus).
    pub fn drain_events(&self) -> Vec<PluginLifecycleEvent> {
        std::mem::take(&mut self.events.lock().unwrap_or_else(|e| e.into_inner()))
    }

    // -- discovery & validation ------------------------------------------------

    /// Scan `dir` for plugin subdirectories containing `manifest.json`.
    /// Each directory is validated independently: one bad plugin never blocks
    /// the others. Returns the ids that reached at least `Validated`.
    pub fn discover(&self, dir: &Path) -> Vec<String> {
        let mut ok = Vec::new();
        let entries: Vec<PathBuf> = std::fs::read_dir(dir)
            .map(|rd| {
                rd.filter_map(std::result::Result::ok)
                    .map(|e| e.path())
                    .collect()
            })
            .unwrap_or_default();
        for path in entries {
            if !path.is_dir() {
                continue;
            }
            let manifest_path = path.join(PLUGIN_MANIFEST_FILENAME);
            if !manifest_path.is_file() {
                continue;
            }
            match PluginManifest::from_file(&manifest_path) {
                Ok(manifest) => {
                    let id = manifest.id.clone();
                    {
                        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
                        let rec = inner
                            .plugins
                            .entry(id.clone())
                            .or_insert_with(|| PluginRecord {
                                manifest: manifest.clone(),
                                dir: path.clone(),
                                state: PluginState::Discovered,
                                instance: None,
                                crash_count: 0,
                                first_crash_at: None,
                            });
                        rec.manifest = manifest;
                        rec.dir = path.clone();
                    }
                    self.set_state(&id, PluginState::Validated);
                    ok.push(id);
                }
                Err(e) => {
                    tracing::warn!("skipping invalid plugin at {}: {e}", path.display());
                }
            }
        }
        ok
    }

    /// Validate a manifest without registering it (used by tests and tooling).
    pub fn validate_manifest_text(text: &str) -> Result<PluginManifest> {
        PluginManifest::from_json(text)
    }

    fn set_state(&self, id: &str, to: PluginState) {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(rec) = inner.plugins.get_mut(id) {
            let mut events = self.events.lock().unwrap_or_else(|e| e.into_inner());
            rec.transition(to, &mut events);
        }
    }

    /// Register an already-validated manifest with explicit WASM bytes
    /// (tests and the example loader path). The plugin reaches `Validated`.
    pub fn register(&self, manifest: PluginManifest, dir: PathBuf) -> Result<()> {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let id = manifest.id.clone();
        let rec = inner
            .plugins
            .entry(id.clone())
            .or_insert_with(|| PluginRecord {
                manifest: manifest.clone(),
                dir: dir.clone(),
                state: PluginState::Discovered,
                instance: None,
                crash_count: 0,
                first_crash_at: None,
            });
        rec.manifest = manifest;
        rec.dir = dir;
        rec.instance = None;
        let mut events = self.events.lock().unwrap_or_else(|e| e.into_inner());
        rec.transition(PluginState::Validated, &mut events);
        Ok(())
    }

    // -- load / start / stop / unload ------------------------------------------

    /// Load the plugin's WASM module (validates imports/exports, checks the
    /// API version). No plugin code runs yet. `Discovered|Validated|Stopped|
    /// Unloaded|Crashed → Loaded`.
    pub fn load(&self, id: &str) -> Result<()> {
        let wasm_bytes = self.read_module_bytes(id)?;
        self.load_bytes(id, &wasm_bytes)
    }

    /// Load from explicit bytes (tests, single-file installs).
    pub fn load_bytes(&self, id: &str, wasm_bytes: &[u8]) -> Result<()> {
        let manifest = self.manifest_of(id)?;
        let transport = self
            .transport
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        let instance = WasmPluginInstance::load(&self.engine, manifest, wasm_bytes, transport)?;
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let Some(rec) = inner.plugins.get_mut(id) else {
            return Err(PluginError::BadState {
                op: "load",
                expected: "registered plugin",
                actual: "unknown",
            });
        };
        match rec.state {
            PluginState::Discovered
            | PluginState::Validated
            | PluginState::Stopped
            | PluginState::Unloaded
            | PluginState::Crashed { .. }
            | PluginState::Disabled { .. } => {}
            PluginState::Loaded | PluginState::Running => {
                return Err(PluginError::BadState {
                    op: "load",
                    expected: "unloaded plugin",
                    actual: "already loaded",
                });
            }
        }
        rec.instance = Some(instance);
        let mut events = self.events.lock().unwrap_or_else(|e| e.into_inner());
        rec.transition(PluginState::Loaded, &mut events);
        Ok(())
    }

    /// Start a loaded instance (`Loaded|Stopped → Running`). Runs
    /// `plugin_init`; a trap moves the plugin to `Crashed` and records it.
    pub fn start(&self, id: &str) -> Result<()> {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let Some(rec) = inner.plugins.get_mut(id) else {
            return Err(PluginError::BadState {
                op: "start",
                expected: "registered plugin",
                actual: "unknown",
            });
        };
        match rec.state {
            PluginState::Loaded | PluginState::Stopped => {}
            PluginState::Disabled { .. } => {
                // Manual restart clears the disabled latch.
                rec.crash_count = 0;
                rec.first_crash_at = None;
            }
            _ => {
                return Err(PluginError::BadState {
                    op: "start",
                    expected: "loaded",
                    actual: rec.state.name(),
                });
            }
        }
        let Some(inst) = rec.instance.as_mut() else {
            return Err(PluginError::BadState {
                op: "start",
                expected: "loaded instance",
                actual: "no instance",
            });
        };
        match inst.start() {
            Ok(()) => {
                let mut events = self.events.lock().unwrap_or_else(|e| e.into_inner());
                rec.transition(PluginState::Running, &mut events);
                Ok(())
            }
            Err(e) => {
                let reason = e.to_string();
                let mut events = self.events.lock().unwrap_or_else(|e| e.into_inner());
                rec.instance = None;
                rec.transition(PluginState::Crashed { reason }, &mut events);
                Err(e)
            }
        }
    }

    /// Stop a running instance (`Running → Stopped`). The instance is kept so
    /// `start` can resume without re-loading the module.
    pub fn stop(&self, id: &str) -> Result<()> {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let Some(rec) = inner.plugins.get_mut(id) else {
            return Err(PluginError::BadState {
                op: "stop",
                expected: "registered plugin",
                actual: "unknown",
            });
        };
        if rec.state != PluginState::Running {
            return Err(PluginError::BadState {
                op: "stop",
                expected: "running",
                actual: rec.state.name(),
            });
        }
        if let Some(inst) = rec.instance.as_mut() {
            // Best-effort shutdown hook; a trap here must not wedge the host.
            if let Err(e) = inst.stop() {
                tracing::warn!("plugin '{}' trapped during shutdown: {e}", rec.manifest.id);
            }
        }
        let mut events = self.events.lock().unwrap_or_else(|e| e.into_inner());
        rec.transition(PluginState::Stopped, &mut events);
        Ok(())
    }

    /// Drop the instance (`* → Unloaded`), keeping the manifest registered.
    pub fn unload(&self, id: &str) -> Result<()> {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let Some(rec) = inner.plugins.get_mut(id) else {
            return Err(PluginError::BadState {
                op: "unload",
                expected: "registered plugin",
                actual: "unknown",
            });
        };
        rec.instance = None;
        let mut events = self.events.lock().unwrap_or_else(|e| e.into_inner());
        rec.transition(PluginState::Unloaded, &mut events);
        Ok(())
    }

    /// Convenience: load → start.
    pub fn load_and_start(&self, id: &str) -> Result<()> {
        self.load(id)?;
        self.start(id)
    }

    /// Record a crash observed outside `start` (e.g. a query trap) and apply
    /// the restart policy. Returns the backoff before a retry is allowed.
    pub fn record_crash(&self, id: &str, reason: String) -> Duration {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let Some(rec) = inner.plugins.get_mut(id) else {
            return Duration::from_secs(0);
        };
        let now = Instant::now();
        match rec.first_crash_at {
            Some(t) if now.duration_since(t).as_secs() > crate::PLUGIN_CRASH_WINDOW_SECS => {
                rec.crash_count = 1;
                rec.first_crash_at = Some(now);
            }
            Some(_) => rec.crash_count += 1,
            None => {
                rec.crash_count = 1;
                rec.first_crash_at = Some(now);
            }
        }
        let count = rec.crash_count;
        rec.instance = None;
        let mut events = self.events.lock().unwrap_or_else(|e| e.into_inner());
        if count >= crate::PLUGIN_MAX_CONSECUTIVE_CRASHES {
            rec.transition(
                PluginState::Disabled {
                    reason: format!("{count} consecutive crashes: {reason}"),
                },
                &mut events,
            );
        } else {
            rec.transition(PluginState::Crashed { reason }, &mut events);
        }
        backoff_for_crash(count)
    }

    // -- queries -----------------------------------------------------------------

    /// Ask a running lyrics-provider plugin. Any trap isolates to this plugin:
    /// the crash is recorded and `Ok(None)` (miss) is returned so the cascade
    /// continues with the next provider.
    pub fn fetch_lyrics(
        &self,
        id: &str,
        query: &LyricsQueryDto,
    ) -> Result<Option<LyricsResultDto>> {
        let request = serde_json::to_string(query)
            .map_err(|e| PluginError::Call(format!("cannot encode lyrics query: {e}")))?;
        let answer =
            self.with_running_instance(id, "lyrics:provider", |inst| inst.fetch_lyrics(&request))?;
        match answer {
            None => Ok(None),
            Some(json) => {
                let dto: LyricsResultDto = serde_json::from_str(&json).map_err(|e| {
                    PluginError::Call(format!("plugin returned invalid lyrics JSON: {e}"))
                })?;
                validate_lyrics_format(&dto.format)?;
                Ok(Some(dto))
            }
        }
    }

    /// Ask a running metadata provider. Trap isolation mirrors `fetch_lyrics`.
    pub fn fetch_metadata(
        &self,
        id: &str,
        query: &MetadataQueryDto,
    ) -> Result<Option<PluginMetadata>> {
        let request = serde_json::to_string(query)
            .map_err(|e| PluginError::Call(format!("cannot encode metadata query: {e}")))?;
        let answer =
            self.with_running_instance(id, "metadata:read", |inst| inst.fetch_metadata(&request))?;
        match answer {
            None => Ok(None),
            Some(json) => {
                let dto: PluginMetadata = serde_json::from_str(&json).map_err(|e| {
                    PluginError::Call(format!("plugin returned invalid metadata JSON: {e}"))
                })?;
                Ok(Some(dto))
            }
        }
    }

    /// Run a closure against a running instance, enforcing state and
    /// capability. Traps are isolated via [`PluginHost::record_crash`].
    fn with_running_instance<T>(
        &self,
        id: &str,
        required_cap: &str,
        f: impl FnOnce(&mut WasmPluginInstance) -> Result<T>,
    ) -> Result<T> {
        let result = {
            let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
            let Some(rec) = inner.plugins.get_mut(id) else {
                return Err(PluginError::BadState {
                    op: "query",
                    expected: "registered plugin",
                    actual: "unknown",
                });
            };
            if rec.state != PluginState::Running {
                return Err(PluginError::BadState {
                    op: "query",
                    expected: "running",
                    actual: rec.state.name(),
                });
            }
            let cap = required_cap
                .parse::<crate::Capability>()
                .map_err(PluginError::CapabilityDenied)?;
            if !rec.manifest.capabilities.contains(cap) {
                return Err(PluginError::CapabilityDenied(format!(
                    "plugin '{id}' does not declare '{required_cap}'"
                )));
            }
            let Some(inst) = rec.instance.as_mut() else {
                return Err(PluginError::BadState {
                    op: "query",
                    expected: "running instance",
                    actual: "no instance",
                });
            };
            f(inst)
        };
        match result {
            Ok(v) => {
                // A completed query proves the instance is healthy: a single
                // crash no longer counts against the restart policy.
                self.reset_crash_count(id);
                Ok(v)
            }
            Err(PluginError::Crashed(reason)) => {
                self.record_crash(id, format!("{reason} ({required_cap} query)"));
                // The cascade treats an isolated crash as provider failure, never
                // fatal: callers check `state()` for the distinction.
                Err(PluginError::Crashed(format!(
                    "plugin '{id}' crashed; isolated"
                )))
            }
            Err(e) => Err(e),
        }
    }

    /// Reset the consecutive-crash counters (a healthy interaction clears
    /// the restart policy).
    fn reset_crash_count(&self, id: &str) {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(rec) = inner.plugins.get_mut(id) {
            rec.crash_count = 0;
            rec.first_crash_at = None;
        }
    }

    // -- introspection --------------------------------------------------------------

    pub fn state(&self, id: &str) -> Option<PluginState> {
        self.inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .plugins
            .get(id)
            .map(|r| r.state.clone())
    }

    pub fn manifest_of(&self, id: &str) -> Result<PluginManifest> {
        self.inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .plugins
            .get(id)
            .map(|r| r.manifest.clone())
            .ok_or(PluginError::BadState {
                op: "lookup",
                expected: "registered plugin",
                actual: "unknown",
            })
    }

    /// Ids of running plugins declaring `cap`.
    pub fn running_with(&self, cap: crate::Capability) -> Vec<String> {
        self.inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .plugins
            .iter()
            .filter(|(_, r)| {
                r.state == PluginState::Running && r.manifest.capabilities.contains(cap)
            })
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Serializable summaries of all registered plugins (any state).
    pub fn list(&self) -> Vec<PluginInfo> {
        self.inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .plugins
            .values()
            .map(|r| PluginInfo {
                id: r.manifest.id.clone(),
                name: r.manifest.name.clone(),
                version: r.manifest.version.to_string(),
                state: r.state.name().to_string(),
                capabilities: r
                    .manifest
                    .capabilities
                    .iter()
                    .map(|c| c.as_str().to_string())
                    .collect(),
            })
            .collect()
    }

    /// Push a visualizer frame to every running instance (read-only tap).
    pub fn push_tap_frame(&self, frame: &[f32]) {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        for rec in inner.plugins.values_mut() {
            if rec.state == PluginState::Running {
                if let Some(inst) = rec.instance.as_mut() {
                    inst.push_tap_frame(frame.to_vec());
                }
            }
        }
    }

    /// Replace the library snapshot served to `sonora.lib_stats`.
    pub fn set_library_snapshot(&self, json: serde_json::Value) {
        let bytes = json.to_string().into_bytes();
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        for rec in inner.plugins.values_mut() {
            if let Some(inst) = rec.instance.as_mut() {
                inst.set_library_snapshot_json(bytes.clone());
            }
        }
    }

    fn read_module_bytes(&self, id: &str) -> Result<Vec<u8>> {
        let (dir, entry) = {
            let inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
            let rec = inner.plugins.get(id).ok_or(PluginError::BadState {
                op: "load",
                expected: "registered plugin",
                actual: "unknown",
            })?;
            (rec.dir.clone(), rec.manifest.entry.clone())
        };
        let path = dir.join(&entry);
        std::fs::read(&path)
            .map_err(|e| PluginError::Load(format!("cannot read {}: {e}", path.display())))
    }

    /// Directories are joined safely: `entry` is a validated bare file name.
    pub fn module_path(&self, id: &str) -> Option<PathBuf> {
        let inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.plugins.get(id).map(|r| r.dir.join(&r.manifest.entry))
    }
}

impl Default for PluginHost {
    fn default() -> Self {
        Self::new().expect("wasmtime engine initialization")
    }
}

fn validate_lyrics_format(format: &str) -> Result<()> {
    match format {
        "plain" | "lrc" | "enhanced_lrc" | "ttml" => Ok(()),
        other => Err(PluginError::Call(format!(
            "unknown lyrics format '{other}'"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_escalates() {
        assert_eq!(backoff_for_crash(1), Duration::from_secs(1));
        assert_eq!(backoff_for_crash(2), Duration::from_secs(5));
        assert_eq!(backoff_for_crash(3), Duration::from_secs(15));
    }
}
