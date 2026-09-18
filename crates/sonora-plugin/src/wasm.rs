//! Wasmtime sandbox for a single plugin instance.
//!
//! Security boundary (fail-closed):
//!
//! - One [`wasmtime::Store`] per plugin; a trap in one instance cannot touch
//!   another instance or the host.
//! - Fuel metering ([`crate::PLUGIN_CALL_FUEL`] per call) turns infinite loops
//!   into traps instead of hangs.
//! - [`wasmtime::StoreLimits`] caps linear memory at 32 MiB.
//! - Only the `sonora.*` host functions listed in
//!   [`CapabilitySet::allowed_imports`] may be imported. WASI and any other
//!   module are rejected at load, so plugins have **no** filesystem, process,
//!   or socket access except through the gated host functions.
//! - Every host function re-checks its capability on each call.
//! - Plugin exports that answer queries (`lyrics_fetch`, …) are loadable only
//!   when the matching capability was declared.
//!
//! JSON ABI: the host writes the request JSON into plugin memory via the
//! exported `alloc`, calls `*_fetch`, then reads the answer JSON from the
//! plugin's result pointers. Response bodies are parsed by host-side parsers,
//! never executed.

use crate::capability::{Capability, CapabilitySet};
use crate::manifest::PluginManifest;
use crate::net::{NetworkError, NetworkTransport};
use crate::{PluginError, Result, PLUGIN_API_VERSION, PLUGIN_CALL_FUEL, PLUGIN_MAX_MEMORY_BYTES};
use std::collections::HashMap;
use std::sync::Arc;
use wasmtime::{
    Caller, Engine, Instance, Linker, Memory, Module, Store, StoreLimits, StoreLimitsBuilder,
};

/// Names of every host function the sandbox may provide.
const KNOWN_HOST_FUNCTIONS: &[&str] = &[
    "log",
    "cache_get",
    "cache_put",
    "net_fetch",
    "net_last_status",
    "lib_stats",
    "tap_read",
];

/// `sonora.net_last_status` codes when no HTTP status applies.
pub const NET_STATUS_NONE: i32 = -1;
pub const NET_STATUS_TIMEOUT: i32 = -2;
pub const NET_STATUS_TRANSPORT: i32 = -3;
pub const NET_STATUS_TOO_LARGE: i32 = -4;

/// Per-instance host state, visible only to the host closures.
struct HostState {
    plugin_id: String,
    caps: CapabilitySet,
    allowed_domains: Vec<String>,
    transport: Arc<dyn NetworkTransport>,
    net_last_status: i32,
    cache: HashMap<Vec<u8>, Vec<u8>>,
    cache_bytes: usize,
    cache_quota: usize,
    library_snapshot_json: Vec<u8>,
    tap_frame: Vec<f32>,
    logs: Vec<String>,
    limits: StoreLimits,
}

impl HostState {
    fn has(&self, cap: Capability) -> bool {
        self.caps.contains(cap)
    }

    fn is_url_allowed(&self, url: &str) -> bool {
        if !self.has(Capability::NetworkFetch) {
            return false;
        }
        let Some(host) = url_host(url) else {
            return false;
        };
        self.allowed_domains.iter().any(|entry| {
            let allowed = entry
                .strip_prefix("https://")
                .unwrap_or(entry)
                .trim_end_matches('/')
                .to_ascii_lowercase();
            host == allowed || host.ends_with(&format!(".{allowed}"))
        })
    }
}

fn url_host(url: &str) -> Option<String> {
    let rest = url.strip_prefix("https://")?;
    let host = rest
        .split(['/', '?', '#'])
        .next()?
        .trim()
        .to_ascii_lowercase();
    if host.is_empty() || host.contains('@') || host.contains(' ') {
        return None;
    }
    Some(host)
}

/// Read `(ptr, len)` from plugin memory with bounds checks.
/// Returns `None` on any violation (the caller maps this to a `-1` denial;
/// host functions never trap).
fn read_mem(caller: &Caller<'_, HostState>, mem: &Memory, ptr: i32, len: i32) -> Option<Vec<u8>> {
    if ptr < 0 || len < 0 {
        return None;
    }
    let offset = ptr as usize;
    let len = len as usize;
    if len > 1024 * 1024 {
        return None;
    }
    let mut buf = vec![0u8; len];
    mem.read(caller, offset, &mut buf).ok()?;
    Some(buf)
}

/// Write `data` into plugin memory at `offset` with bounds checks.
fn write_mem(caller: &mut Caller<'_, HostState>, mem: &Memory, offset: i32, data: &[u8]) -> bool {
    if offset < 0 {
        return false;
    }
    mem.write(caller, offset as usize, data).is_ok()
}

fn memory_of(caller: &mut Caller<'_, HostState>) -> Option<Memory> {
    caller.get_export("memory")?.into_memory()
}

fn define_host_functions(linker: &mut Linker<HostState>) -> Result<()> {
    linker
        .func_wrap(
            "sonora",
            "log",
            |mut caller: Caller<'_, HostState>, ptr: i32, len: i32| -> i32 {
                let Some(mem) = memory_of(&mut caller) else {
                    return -1;
                };
                let Some(bytes) = read_mem(&caller, &mem, ptr, len) else {
                    return -1;
                };
                let msg = String::from_utf8_lossy(&bytes);
                let state = caller.data_mut();
                if state.logs.len() < 100 {
                    state.logs.push(msg.chars().take(1024).collect());
                }
                tracing::info!(plugin = %state.plugin_id, "plugin log: {msg}");
                0
            },
        )
        .map_err(|e| PluginError::Load(format!("cannot define sonora.log: {e}")))?;

    linker
        .func_wrap(
            "sonora",
            "cache_get",
            |mut caller: Caller<'_, HostState>,
             kptr: i32,
             klen: i32,
             vbuf: i32,
             vcap: i32|
             -> i32 {
                if !caller.data().has(Capability::StorageCache) {
                    return -1;
                }
                let Some(mem) = memory_of(&mut caller) else {
                    return -1;
                };
                let Some(key) = read_mem(&caller, &mem, kptr, klen) else {
                    return -1;
                };
                let value = caller.data().cache.get(&key).cloned();
                match value {
                    Some(v) => {
                        if v.len() as i32 > vcap {
                            return -1;
                        }
                        if !write_mem(&mut caller, &mem, vbuf, &v) {
                            return -1;
                        }
                        v.len() as i32
                    }
                    None => -1,
                }
            },
        )
        .map_err(|e| PluginError::Load(format!("cannot define sonora.cache_get: {e}")))?;

    linker
        .func_wrap(
            "sonora",
            "cache_put",
            |mut caller: Caller<'_, HostState>,
             kptr: i32,
             klen: i32,
             vptr: i32,
             vlen: i32|
             -> i32 {
                if !caller.data().has(Capability::StorageCache) {
                    return -1;
                }
                if klen <= 0 || klen > 1024 || !(0..=256 * 1024).contains(&vlen) {
                    return -1;
                }
                let Some(mem) = memory_of(&mut caller) else {
                    return -1;
                };
                let (Some(key), Some(value)) = (
                    read_mem(&caller, &mem, kptr, klen),
                    read_mem(&caller, &mem, vptr, vlen),
                ) else {
                    return -1;
                };
                let state = caller.data_mut();
                let old = state.cache.get(&key).map(Vec::len).unwrap_or(0);
                let next = state
                    .cache_bytes
                    .saturating_sub(old)
                    .saturating_add(value.len());
                if next > state.cache_quota {
                    return -1;
                }
                state.cache_bytes = next;
                state.cache.insert(key, value);
                0
            },
        )
        .map_err(|e| PluginError::Load(format!("cannot define sonora.cache_put: {e}")))?;

    linker
        .func_wrap(
            "sonora",
            "net_fetch",
            |mut caller: Caller<'_, HostState>,
             uptr: i32,
             ulen: i32,
             rbuf: i32,
             rcap: i32|
             -> i32 {
                let Some(mem) = memory_of(&mut caller) else {
                    return -1;
                };
                let Some(url_bytes) = read_mem(&caller, &mem, uptr, ulen) else {
                    return -1;
                };
                let url = String::from_utf8_lossy(&url_bytes).into_owned();
                // Denied (no capability or off allow-list): no status update,
                // no socket. Fail closed.
                if !caller.data().is_url_allowed(&url) {
                    return -1;
                }
                let transport = caller.data().transport.clone();
                match transport.fetch(&url, crate::PLUGIN_NET_MAX_BYTES) {
                    Ok(body) => {
                        caller.data_mut().net_last_status = 200;
                        if body.len() as i32 > rcap || rcap < 0 {
                            return -1;
                        }
                        if !write_mem(&mut caller, &mem, rbuf, &body) {
                            return -1;
                        }
                        body.len() as i32
                    }
                    Err(e) => {
                        let code = match e {
                            NetworkError::HttpStatus(s) => s as i32,
                            NetworkError::Timeout => NET_STATUS_TIMEOUT,
                            NetworkError::TooLarge => NET_STATUS_TOO_LARGE,
                            NetworkError::Io(_) => NET_STATUS_TRANSPORT,
                        };
                        caller.data_mut().net_last_status = code;
                        -1
                    }
                }
            },
        )
        .map_err(|e| PluginError::Load(format!("cannot define sonora.net_fetch: {e}")))?;

    linker
        .func_wrap(
            "sonora",
            "net_last_status",
            |caller: Caller<'_, HostState>| -> i32 {
                if !caller.data().has(Capability::NetworkFetch) {
                    return -1;
                }
                caller.data().net_last_status
            },
        )
        .map_err(|e| PluginError::Load(format!("cannot define sonora.net_last_status: {e}")))?;

    linker
        .func_wrap(
            "sonora",
            "lib_stats",
            |mut caller: Caller<'_, HostState>, obuf: i32, ocap: i32| -> i32 {
                if !caller.data().has(Capability::LibraryRead) {
                    return -1;
                }
                let snapshot = caller.data().library_snapshot_json.clone();
                if snapshot.len() as i32 > ocap {
                    return -1;
                }
                let Some(mem) = memory_of(&mut caller) else {
                    return -1;
                };
                if !write_mem(&mut caller, &mem, obuf, &snapshot) {
                    return -1;
                }
                snapshot.len() as i32
            },
        )
        .map_err(|e| PluginError::Load(format!("cannot define sonora.lib_stats: {e}")))?;

    linker
        .func_wrap(
            "sonora",
            "tap_read",
            |mut caller: Caller<'_, HostState>, obuf: i32, ocap: i32| -> i32 {
                if !caller.data().has(Capability::VisualizerTap) {
                    return -1;
                }
                let frame = caller.data().tap_frame.clone();
                if frame.len() * 4 > ocap as usize || ocap < 0 {
                    return -1;
                }
                let mut raw = Vec::with_capacity(frame.len() * 4);
                for v in &frame {
                    raw.extend_from_slice(&v.to_le_bytes());
                }
                let Some(mem) = memory_of(&mut caller) else {
                    return -1;
                };
                if !write_mem(&mut caller, &mem, obuf, &raw) {
                    return -1;
                }
                frame.len() as i32
            },
        )
        .map_err(|e| PluginError::Load(format!("cannot define sonora.tap_read: {e}")))?;

    Ok(())
}

/// A loaded, sandboxed plugin instance. All fallible WASM interactions go
/// through methods that reset fuel first; a [`wasmtime::Trap`] is reported as
/// [`PluginError::Crashed`] so the host can isolate the failure.
pub struct WasmPluginInstance {
    manifest: PluginManifest,
    store: Store<HostState>,
    instance: Instance,
    memory: Memory,
}

impl WasmPluginInstance {
    /// Compile, link, and instantiate `wasm_bytes` after validating imports,
    /// exports, and the API version. No plugin code runs during load.
    pub fn load(
        engine: &Engine,
        manifest: PluginManifest,
        wasm_bytes: &[u8],
        transport: Arc<dyn NetworkTransport>,
    ) -> Result<Self> {
        if wasm_bytes.is_empty() || wasm_bytes.len() > 16 * 1024 * 1024 {
            return Err(PluginError::Load(
                "empty or oversize (>16 MiB) module".to_string(),
            ));
        }
        let module = Module::new(engine, wasm_bytes)
            .map_err(|e| PluginError::Load(format!("invalid wasm module: {e}")))?;

        // Fail closed on imports: only entitled sonora.* functions, never WASI.
        let allowed = manifest.capabilities.allowed_imports();
        for import in module.imports() {
            if import.module() != "sonora" {
                return Err(PluginError::Load(format!(
                    "forbidden import '{}::{}' (only 'sonora.*' host functions allowed)",
                    import.module(),
                    import.name()
                )));
            }
            if !KNOWN_HOST_FUNCTIONS.contains(&import.name()) {
                return Err(PluginError::Load(format!(
                    "unknown host import 'sonora::{}'",
                    import.name()
                )));
            }
            if !allowed.contains(import.name()) {
                let cap = match import.name() {
                    "cache_get" | "cache_put" => Capability::StorageCache,
                    "net_fetch" | "net_last_status" => Capability::NetworkFetch,
                    "lib_stats" => Capability::LibraryRead,
                    "tap_read" => Capability::VisualizerTap,
                    _ => {
                        return Err(PluginError::Load(format!(
                            "host import 'sonora::{}' has no capability mapping",
                            import.name()
                        )))
                    }
                };
                return Err(PluginError::CapabilityDenied(format!(
                    "plugin '{}' imports 'sonora::{}' without declaring '{cap}'",
                    manifest.id,
                    import.name()
                )));
            }
        }

        // Export/capability consistency: query exports require declarations.
        let export_names: Vec<String> = module.exports().map(|e| e.name().to_string()).collect();
        if let Some(missing) = manifest
            .capabilities
            .missing_capability_for_exports(export_names.iter().map(String::as_str))
        {
            return Err(PluginError::CapabilityDenied(format!(
                "plugin '{}' exports query functions without declaring '{missing}'",
                manifest.id
            )));
        }

        let needs_memory_api = export_names.iter().any(|e| {
            matches!(
                e.as_str(),
                "lyrics_fetch" | "metadata_fetch" | "visualizer_info" | "widget_describe"
            )
        });

        let mut linker = Linker::new(engine);
        define_host_functions(&mut linker)?;

        let state = HostState {
            plugin_id: manifest.id.clone(),
            caps: manifest.capabilities.clone(),
            allowed_domains: manifest.allowed_domains.clone(),
            transport,
            net_last_status: NET_STATUS_NONE,
            cache: HashMap::new(),
            cache_bytes: 0,
            cache_quota: crate::PLUGIN_MAX_CACHE_BYTES,
            library_snapshot_json: b"{\"tracks\":0,\"stub\":true}".to_vec(),
            tap_frame: Vec::new(),
            logs: Vec::new(),
            limits: StoreLimitsBuilder::new()
                .memory_size(PLUGIN_MAX_MEMORY_BYTES as usize)
                .instances(1)
                .memories(1)
                .tables(1)
                .build(),
        };
        let mut store = Store::new(engine, state);
        store.limiter(|s| &mut s.limits);
        store
            .set_fuel(PLUGIN_CALL_FUEL)
            .map_err(|e| PluginError::Load(e.to_string()))?;

        let instance = linker
            .instantiate(&mut store, &module)
            .map_err(|e| PluginError::Load(format!("instantiation failed: {e}")))?;

        let memory = instance
            .get_memory(&mut store, "memory")
            .ok_or_else(|| PluginError::Load("missing required 'memory' export".to_string()))?;

        if needs_memory_api && instance.get_func(&mut store, "alloc").is_none() {
            return Err(PluginError::Load(
                "query exports require an 'alloc(size: i32) -> i32' export".to_string(),
            ));
        }

        let mut inst = Self {
            manifest,
            store,
            instance,
            memory,
        };
        // The version export is mandatory: without it the host cannot prove
        // the module speaks v1, so the load fails closed.
        let has_version = inst.has_export("plugin_api_version");
        if !has_version {
            return Err(PluginError::Load(
                "missing required 'plugin_api_version() -> i32' export".to_string(),
            ));
        }
        let api_version = inst
            .call_simple("plugin_api_version")?
            .ok_or_else(|| PluginError::Load("cannot call 'plugin_api_version'".to_string()))?;
        if api_version as u32 != PLUGIN_API_VERSION {
            return Err(PluginError::Load(format!(
                "unsupported plugin api_version {api_version} (host requires v{})",
                PLUGIN_API_VERSION
            )));
        }
        Ok(inst)
    }

    pub fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }

    pub fn plugin_id(&self) -> &str {
        &self.manifest.id
    }

    fn reset_fuel(&mut self) -> Result<()> {
        self.store
            .set_fuel(PLUGIN_CALL_FUEL)
            .map_err(|e| PluginError::Call(format!("cannot set fuel: {e}")))?;
        Ok(())
    }

    fn typed2(&mut self, name: &str) -> Result<Option<wasmtime::TypedFunc<(i32, i32), i32>>> {
        match self
            .instance
            .get_typed_func::<(i32, i32), i32>(&mut self.store, name)
        {
            Ok(f) => Ok(Some(f)),
            Err(_) => Ok(None),
        }
    }

    fn typed0(&mut self, name: &str) -> Result<Option<wasmtime::TypedFunc<(), i32>>> {
        match self
            .instance
            .get_typed_func::<(), i32>(&mut self.store, name)
        {
            Ok(f) => Ok(Some(f)),
            Err(_) => Ok(None),
        }
    }

    /// Call an optional `() -> i32` export. Missing exports are treated as
    /// success (no-op hooks).
    fn call_simple(&mut self, name: &str) -> Result<Option<i32>> {
        let Some(func) = self.typed0(name)? else {
            return Ok(None);
        };
        self.reset_fuel()?;
        func.call(&mut self.store, ())
            .map(Some)
            .map_err(|e| self.trap(name, e))
    }

    fn trap(&self, op: &str, e: wasmtime::Error) -> PluginError {
        PluginError::Crashed(format!(
            "plugin '{}' trapped in {op}: {e}",
            self.manifest.id
        ))
    }

    /// Run `plugin_init` if exported. A trap marks the plugin crashed.
    pub fn start(&mut self) -> Result<()> {
        self.call_simple("plugin_init")?;
        Ok(())
    }

    /// Run `plugin_shutdown` if exported. Best-effort: a trap is reported but
    /// the host still drops the instance afterwards.
    pub fn stop(&mut self) -> Result<()> {
        self.call_simple("plugin_shutdown")?;
        Ok(())
    }

    fn alloc(&mut self, size: usize) -> Result<i32> {
        if size == 0 || size > 1024 * 1024 {
            return Err(PluginError::Call("request payload over 1 MiB".to_string()));
        }
        let Some(func) = self.typed1("alloc")? else {
            return Err(PluginError::Call("missing 'alloc' export".to_string()));
        };
        self.reset_fuel()?;
        let ptr = func
            .call(&mut self.store, size as i32)
            .map_err(|e| self.trap("alloc", e))?;
        if ptr <= 0 {
            return Err(PluginError::Call("plugin alloc returned null".to_string()));
        }
        Ok(ptr)
    }

    fn typed1(&mut self, name: &str) -> Result<Option<wasmtime::TypedFunc<i32, i32>>> {
        match self
            .instance
            .get_typed_func::<i32, i32>(&mut self.store, name)
        {
            Ok(f) => Ok(Some(f)),
            Err(_) => Ok(None),
        }
    }

    fn write_input(&mut self, data: &[u8]) -> Result<(i32, i32)> {
        let ptr = self.alloc(data.len())?;
        let mem = self.memory;
        mem.write(&mut self.store, ptr as usize, data)
            .map_err(|_| PluginError::Call("cannot write request to plugin memory".to_string()))?;
        Ok((ptr, data.len() as i32))
    }

    fn read_output(&mut self, ptr_fn: &str, len_fn: &str) -> Result<Vec<u8>> {
        let ptr = self
            .call_simple(ptr_fn)?
            .ok_or_else(|| PluginError::Call(format!("missing '{ptr_fn}' export")))?;
        let len = self
            .call_simple(len_fn)?
            .ok_or_else(|| PluginError::Call(format!("missing '{len_fn}' export")))?;
        if ptr < 0 || len < 0 || len as usize > 1024 * 1024 {
            return Err(PluginError::Call(
                "plugin returned invalid result range".to_string(),
            ));
        }
        let mem = self.memory;
        let size = mem.data_size(&self.store);
        let end = (ptr as usize)
            .checked_add(len as usize)
            .filter(|end| *end <= size)
            .ok_or_else(|| PluginError::Call("plugin result out of bounds".to_string()))?;
        let _ = end;
        let mut buf = vec![0u8; len as usize];
        mem.read(&self.store, ptr as usize, &mut buf)
            .map_err(|_| PluginError::Call("cannot read plugin result".to_string()))?;
        Ok(buf)
    }

    /// Generic JSON request/response round-trip over plugin memory.
    fn query_json(
        &mut self,
        fetch_fn: &str,
        ptr_fn: &str,
        len_fn: &str,
        request_json: &str,
    ) -> Result<Option<String>> {
        let fetch = match self.typed2(fetch_fn)? {
            Some(f) => f,
            None => {
                return Err(PluginError::Call(format!("missing '{fetch_fn}' export")));
            }
        };
        let (ptr, len) = self.write_input(request_json.as_bytes())?;
        self.reset_fuel()?;
        let status = fetch
            .call(&mut self.store, (ptr, len))
            .map_err(|e| self.trap(fetch_fn, e))?;
        if status == 0 {
            return Ok(None);
        }
        if status != 1 {
            return Err(PluginError::Call(format!(
                "{fetch_fn} returned unexpected status {status}"
            )));
        }
        let bytes = self.read_output(ptr_fn, len_fn)?;
        String::from_utf8(bytes)
            .map(Some)
            .map_err(|_| PluginError::Call("plugin returned non-UTF8 JSON".to_string()))
    }

    /// Lyrics query. Requires the `lyrics_fetch` export (load-gated).
    pub fn fetch_lyrics(&mut self, request_json: &str) -> Result<Option<String>> {
        self.query_json(
            "lyrics_fetch",
            "lyrics_result_ptr",
            "lyrics_result_len",
            request_json,
        )
    }

    /// Metadata query. Requires the `metadata_fetch` export (load-gated).
    pub fn fetch_metadata(&mut self, request_json: &str) -> Result<Option<String>> {
        self.query_json(
            "metadata_fetch",
            "metadata_result_ptr",
            "metadata_result_len",
            request_json,
        )
    }

    /// Visualizer descriptor query. Requires the `visualizer_info` export.
    pub fn visualizer_info(&mut self) -> Result<String> {
        let info = match self.typed0("visualizer_info")? {
            Some(f) => f,
            None => {
                return Err(PluginError::Call(
                    "missing 'visualizer_info' export".to_string(),
                ))
            }
        };
        self.reset_fuel()?;
        let status = info
            .call(&mut self.store, ())
            .map_err(|e| self.trap("visualizer_info", e))?;
        if status != 1 {
            return Err(PluginError::Call(
                "visualizer_info reported no descriptor".to_string(),
            ));
        }
        let bytes = self.read_output("visualizer_result_ptr", "visualizer_result_len")?;
        String::from_utf8(bytes)
            .map_err(|_| PluginError::Call("plugin returned non-UTF8 JSON".to_string()))
    }

    /// Widget descriptor query. Requires the `widget_describe` export.
    pub fn widget_describe(&mut self) -> Result<String> {
        let describe = match self.typed0("widget_describe")? {
            Some(f) => f,
            None => {
                return Err(PluginError::Call(
                    "missing 'widget_describe' export".to_string(),
                ))
            }
        };
        self.reset_fuel()?;
        let status = describe
            .call(&mut self.store, ())
            .map_err(|e| self.trap("widget_describe", e))?;
        if status != 1 {
            return Err(PluginError::Call(
                "widget_describe reported no descriptor".to_string(),
            ));
        }
        let bytes = self.read_output("widget_result_ptr", "widget_result_len")?;
        String::from_utf8(bytes)
            .map_err(|_| PluginError::Call("plugin returned non-UTF8 JSON".to_string()))
    }

    /// Replace the read-only library snapshot served to `sonora.lib_stats`.
    pub fn set_library_snapshot_json(&mut self, json: Vec<u8>) {
        self.store.data_mut().library_snapshot_json = json;
    }

    /// Push the latest visualizer frame served to `sonora.tap_read`.
    pub fn push_tap_frame(&mut self, frame: Vec<f32>) {
        self.store.data_mut().tap_frame = frame;
    }

    /// Drain captured `sonora.log` lines (for tests and diagnostics).
    pub fn take_logs(&mut self) -> Vec<String> {
        std::mem::take(&mut self.store.data_mut().logs)
    }

    /// Export names present in the loaded module (for capability routing).
    pub fn has_export(&mut self, name: &str) -> bool {
        self.instance.get_func(&mut self.store, name).is_some()
    }
}
