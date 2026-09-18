# LRCLIB Lyrics Provider (`org.sonora.lrclib`)

The first real Sonora plugin: resolves plain and line-synced lyrics from the
[LRCLIB](https://lrclib.net) public API through the sandboxed host network
API. Ships as WebAssembly; the sandbox performs no I/O itself.

```
plugins/lyrics-lrclib/
├── manifest.json   # id, api_version, capabilities, lrclib.net allow-list
├── plugin.wasm     # built module (checked in; rebuild with ./build.sh)
├── build.sh        # Rust -> wasm32-unknown-unknown build
├── Cargo.toml      # standalone crate (NOT a workspace member, on purpose)
├── Cargo.lock      # pinned plugin dependencies
├── src/
│   ├── lib.rs      # crate root: client always, ABI on wasm32 only
│   ├── client.rs   # pure LRCLIB logic (URL building, response mapping)
│   └── abi.rs      # WASM imports/exports, static memory regions
└── README.md       # this file
```

## Manifest

```json
{
  "id": "org.sonora.lrclib",
  "version": "1.0.0",
  "api_version": 1,
  "capabilities": ["lyrics:provider", "network:fetch"],
  "entry": "plugin.wasm",
  "allowed_domains": ["https://lrclib.net"]
}
```

`network:fetch` without the `allowed_domains` entry fails validation, and any
other origin fails at runtime — the plugin can only ever talk to LRCLIB.

## Capabilities

| Capability | Used for |
|---|---|
| `lyrics:provider` | `lyrics_fetch` export; registration in the lyrics cascade (after cache and built-ins) |
| `network:fetch` | `sonora.net_fetch` (LRCLIB GET only) + `sonora.net_last_status` (status mapping) |

The plugin imports exactly `sonora.log`, `sonora.net_fetch`, and
`sonora.net_last_status`. It declares no other capability and uses no other
host function — verify with: the host rejects any module importing more.

## API usage

Per query, the plugin:

1. Reads the host's `LyricsQueryDto` JSON (`title`, `artist?`, `album?`,
   `duration_ms?`) from plugin memory.
2. Builds `GET https://lrclib.net/api/get?track_name=…&artist_name=…&album_name=…&duration=…`
   with RFC 3986 percent-encoding (`src/client.rs::build_search_url`). Empty
   titles and >2000-char URLs are misses without network traffic.
3. Calls `sonora.net_fetch(url, buf, cap)`. The host enforces the allow-list,
   a 5 s timeout, a 512 KiB cap, and never follows redirects.
4. On `n >= 0`, parses the LRCLIB JSON: non-empty `syncedLyrics` wins
   (`format: "lrc"`), else non-empty `plainLyrics` (`format: "plain"`), else
   miss. `instrumental: true` is a miss. The answer envelope
   `{"format","content","attribution":"LRCLIB (lrclib.net)"}` is returned for
   the host's own parsers — the plugin never touches host internals.
5. On `n < 0`, reads `sonora.net_last_status()` and logs the class
   (404 = no lyrics, 429 = rate-limited, timeout/transport/too-large), then
   returns miss so the cascade falls through gracefully.

Memory: three static regions (`ARENA` 1 MiB request, `RESP_BUF` 512 KiB
response, `OUT_BUF` 1 MiB answer) accessed via raw pointers; a Rust panic
aborts to a trap and the host isolates it to this plugin.

## Build instructions

Prerequisites: Rust with the `wasm32-unknown-unknown` target
(`rustup target add wasm32-unknown-unknown`).

```sh
cd plugins/lyrics-lrclib
./build.sh        # -> plugin.wasm (also runs unit tests first if you want)
cargo test        # pure-logic unit tests (URL building, mapping), host-side
```

Host-side verification (from the workspace root):

```sh
cargo test -p sonora-plugin --test lrclib          # hermetic (fake transport)
cargo test -p sonora-plugin --test lrclib -- --ignored   # live LRCLIB check
```

Install for the desktop app: copy this directory to
`<data_dir>/plugins/lyrics-lrclib/` (e.g. `~/.local/share/sonora/plugins/`).
The desktop discovers, validates, loads, and starts it at boot; lyrics
resolution then consults it automatically after cache and built-ins.
`list_plugins` (Tauri command) reports its state.

## Failure behavior

| Condition | Behavior |
|---|---|
| Empty title / overlong URL | miss, no network call, log line |
| 404 from LRCLIB | miss ("no lyrics"), log |
| 429 rate limit | miss, warning log (no retry storm) |
| Timeout (>5 s) / transport error / >512 KiB | miss, warning log |
| Invalid JSON / empty lyrics / instrumental | miss |
| Plugin trap (bug/panic/OOM/fuel) | host marks it `Crashed`, retries with backoff, disables after 3/60 s; cascade continues with other providers |
| Manifest tampered (extra capability, edited domains) | validation or load fails closed |

Every row above is covered by tests: `cargo test -p sonora-plugin --test lrclib`
(plus `crates/sonora-plugin/tests/plugin_host.rs` for the generic machinery
and `tests/` for the app-level cascade and fallback events).
