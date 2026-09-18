# Example plugin: WASM lyrics provider

A minimal, working Sonora plugin. It always answers lyrics queries with a
static two-line LRC payload. Copy this directory to start your own plugin.

```
plugins/example/
├── manifest.json   # identity, api_version, capabilities, entry
├── plugin.wat      # WASM source (hand-written; see "Rust path" below)
├── plugin.wasm     # built module (checked in; rebuild any time)
└── README.md       # this file
```

## Try it (host side, Rust)

```rust
use sonora_plugin::{PluginHost, PluginManifest};
use std::path::PathBuf;

let host = PluginHost::new()?;
let dir = PathBuf::from("plugins/example");
let manifest = PluginManifest::from_file(&dir.join("manifest.json"))?;
let id = manifest.id.clone();
host.register(manifest, dir)?;
host.load(&id)?;    // validates imports/exports, checks api_version; runs NO code
host.start(&id)?;   // runs `plugin_init`; a trap here isolates to this plugin

let answer = host.fetch_lyrics(&id, &sonora_plugin::LyricsQueryDto {
    title: "Example".into(),
    ..Default::default()
})?;
assert!(answer.unwrap().content.contains("Hello from the example plugin"));

host.stop(&id)?;    // runs `plugin_shutdown` (best-effort)
host.unload(&id)?;  // drops the instance; manifest stays registered
```

## Manifest v1

Required fields:

| Field | Rules |
|---|---|
| `id` | reverse-DNS, ≥2 lowercase labels (`org.example.lyrics`) |
| `name` | 1–128 chars |
| `version` | semver (`1.0.0`) |
| `author` | string or `{ "name", "url?" }` |
| `description` | 1–1024 chars |
| `api_version` | must be `1` (host rejects anything else) |
| `capabilities` | ≥1 known capability, no duplicates |
| `entry` | bare `.wasm` file name (no paths, no `..`) |
| `allowed_domains` | required iff `network:fetch`; each an `https://` origin |

Extra keys (e.g. `$schema`, `license`) are ignored by the host.

## Capabilities (v1, closed set)

| Capability | Unlocks |
|---|---|
| `lyrics:provider` | `lyrics_fetch` export; registration in the lyrics cascade |
| `metadata:read` | `metadata_fetch` export |
| `library:read` | `sonora.lib_stats` (read-only snapshot JSON) |
| `visualizer:tap` | `sonora.tap_read` (read-only FFT frame) + `visualizer_info` export |
| `ui:widget` | `widget_describe` export |
| `storage:cache` | `sonora.cache_get` / `sonora.cache_put` (1 MiB quota) |
| `network:fetch` | `sonora.net_fetch` — allow-listed `https://` origins only; **v1 returns a deterministic stub, never opens a socket** |

Enforcement is three-layered: manifest validation → load-time import/export
check (WASI and unknown imports rejected) → call-time gating. Declaring an
export without its capability fails the load, fail-closed.

## WASM ABI

Imports (module `sonora`; link only what your capabilities allow):

```
log(ptr: i32, len: i32) -> i32                          # always allowed
cache_get(kptr, klen, vbuf, vcap) -> i32                # bytes written | -1
cache_put(kptr, klen, vptr, vlen) -> i32                # 0 | -1
net_fetch(urlptr, urllen, respbuf, respcap) -> i32       # bytes written | -1
net_last_status() -> i32                                # HTTP status | -1 none, -2 timeout, -3 transport, -4 too large
lib_stats(outbuf, outcap) -> i32                        # bytes written | -1
tap_read(outbuf, outcap) -> i32                         # floats written | -1
```

`net_fetch` performs a real HTTPS request through a host-side transport
(5 s timeout, 512 KiB cap, redirects never followed) but only for origins in
the manifest's `allowed_domains`. Anything else — other origins, plain HTTP,
missing capability — returns `-1` with no socket opened. See
`plugins/lyrics-lrclib/` for a production user of this API.

Required exports: `memory`, `plugin_api_version() -> i32` (= 1).
Optional lifecycle: `plugin_init() -> i32`, `plugin_shutdown() -> i32`
(`0` = ok; missing = no-op).
Query exports (each pair needs its capability + an `alloc` export):

```
alloc(size: i32) -> i32                                 # host writes request here
lyrics_fetch(ptr: i32, len: i32) -> i32                 # 1 = hit, 0 = miss
lyrics_result_ptr() -> i32 / lyrics_result_len() -> i32
metadata_fetch / metadata_result_ptr / metadata_result_len   # same shape
visualizer_info() -> i32 (+ visualizer_result_ptr/len)
widget_describe() -> i32 (+ widget_result_ptr/len)
```

JSON shapes:

```jsonc
// lyrics request
{ "title": "…", "artist": "…?", "album": "…?", "duration_ms": 123? }
// lyrics answer (host parses `content` with its own parsers)
{ "format": "plain|lrc|enhanced_lrc|ttml", "content": "…", "attribution": "…?" }
// metadata request / answer
{ "title": "…", "artist": "…?", "album": "…?" }
{ "title": "…?", "artist": "…?", "album": "…?", "year": 0?,
  "genre": "…?", "bio": "…?", "art_url": "…?" }
// visualizer_info answer
{ "name": "…", "preferred_fps": 60, "uses_audio_tap": true }
// widget_describe answer
{ "slot": "sidebar|statusbar|now_playing", "title": "…", "version": "…" }
```

Host limits: 1 MiB fuel per call (infinite loops trap), 32 MiB linear
memory, 1 MiB request/response payloads, 100 buffered log lines.

## Lifecycle & crash policy

`discover → validate → load → start → stop → unload`, with `crashed` and
`disabled` as failure states. A trap (including `unreachable`, OOM, or fuel
exhaustion) drops only that instance; the host records the crash and retries
with backoff (1s → 5s → 15s). After 3 crashes in 60 s the plugin is
`disabled` until a manual reload. Query-time crashes resolve as provider
misses so the lyrics cascade continues.

## Building your own

1. **Hand-written WAT** (this example): edit `plugin.wat`, rebuild with
   `cargo run -p sonora-plugin --example build_example` (or `wat` CLI).
2. **Rust → WASM** (recommended for real plugins):
   ```sh
   rustup target add wasm32-unknown-unknown
   cargo build --target wasm32-unknown-unknown --release
   ```
   Export the ABI functions with `#[no_mangle]`, avoid WASI imports
   (`#![no_std]` + `extern "std"` off; use `core` + manual memory), and keep
   the module dependency-free. The host links only `sonora.*`, so any other
   import (including `wasi_snapshot_preview1`) fails the load by design.

## What NOT to do

No filesystem / process / socket access exists — there is nothing to
escalate to. Do not shell out, do not fetch unlisted origins. `network:fetch`
reaches only allow-listed `https://` origins through the host transport.
Marketplace, paid plugins, QuickJS scripting, projectM, and DSP plugins are
explicitly out of scope.
