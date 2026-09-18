# Genius Lyrics Provider (`org.sonora.lyrics_genius`)

A metadata-based lyrics provider: it turns track metadata (artist + title)
into lyrics via Genius public web endpoints. Genius offers no token-free
lyrics API, so the plugin uses the same two endpoints a browser does — the
unauthenticated JSON search API to find the song page, then the song page's
`data-lyrics-container` divs. Every byte flows through the sandboxed host
network API; the sandbox performs no I/O itself.

```
plugins/lyrics-genius/
├── manifest.json   # id, api_version, capabilities, genius.com allow-list
├── plugin.wasm     # built module (checked in; rebuild with ./build.sh)
├── build.sh        # Rust -> wasm32-unknown-unknown build
├── Cargo.toml      # standalone crate (NOT a workspace member, on purpose)
├── Cargo.lock      # pinned plugin dependencies
├── src/
│   ├── lib.rs      # crate root: client always, ABI on wasm32 only
│   ├── client.rs   # pure Genius logic (URLs, search parsing, HTML extraction)
│   └── abi.rs      # WASM imports/exports, static memory regions
└── README.md       # this file
```

## Manifest

```json
{
  "id": "org.sonora.lyrics_genius",
  "version": "1.0.0",
  "api_version": 1,
  "capabilities": ["lyrics:provider", "network:fetch"],
  "entry": "plugin.wasm",
  "allowed_domains": ["https://genius.com"]
}
```

The plugin imports exactly `sonora.log`, `sonora.net_fetch`, and
`sonora.net_last_status`. It declares no other capability — verify with: the
host rejects any module importing more, and loading this module under a
manifest without `network:fetch` fails closed.

## Capabilities

| Capability | Used for |
|---|---|
| `lyrics:provider` | `lyrics_fetch` export; registration in the lyrics cascade (after cache and built-ins) |
| `network:fetch` | `sonora.net_fetch` (genius.com GETs only) + `sonora.net_last_status` (status mapping) |

## API usage

Per query, the plugin:

1. Reads the host's `LyricsQueryDto` JSON (`title`, `artist?`, …) from plugin
   memory.
2. Builds `GET https://genius.com/api/search/multi?per_page=5&q={artist title}`
   with RFC 3986 percent-encoding (`src/client.rs::build_search_url`). Empty
   titles and >2000-char URLs are misses without network traffic.
3. Fetches the search JSON through `sonora.net_fetch` (host enforces the
   allow-list, timeout, response cap, and no-redirect policy) and takes the
   first `song`-section hit whose URL is an `https://genius.com/…-lyrics`
   song page. Artist/album/video hits, off-domain URLs, and odd shapes are
   skipped fail-closed.
4. Fetches the song page the same way and extracts every
   `data-lyrics-container` div (nested divs depth-counted, `<br>` → newline,
   entities decoded incl. curly quotes/dashes), normalizing whitespace. Pages
   without containers, or with under 20 lyric characters, are misses.
5. Returns `{format: "plain", content, attribution: "Genius (genius.com) — <song url>"}`.
   Genius serves no timestamps, so `plain` is the honest format; the host
   parses it with its own parsers.

The scanner jumps tag-to-tag (`str::find`, memchr-backed) instead of
byte-stepping, so even ~600 KiB song pages stay inside the host fuel budget;
anything beyond degrades to a miss, never a hang.

Scraping is inherently coupled to Genius markup: the parser is strict on
purpose — any structural surprise is a miss, and the lyrics cascade falls
through to the next provider. A token-based Genius API variant would need a
secrets ABI extension (explicitly deferred, not implemented).

## Failure behavior

| Condition | Behavior |
|---|---|
| Empty title / overlong URL | miss, no network call, log line |
| Search 404 / timeout / transport error | miss + classified log via `net_last_status` |
| No song hit / non-song or off-domain URL | miss, log line |
| Song page 404 / 403 / 429 / timeout | miss + classified log |
| Changed markup / no containers / thin lyrics | miss, log line |
| Invalid JSON anywhere | miss |
| Plugin trap (bug/panic/OOM/fuel) | host marks it `Crashed`, retries with backoff, disables after 3/60 s |
| Manifest tampered (extra capability, edited domains) | validation or load fails closed |

## Build instructions

Prerequisites: Rust with the `wasm32-unknown-unknown` target
(`rustup target add wasm32-unknown-unknown`).

```sh
cd plugins/lyrics-genius
./build.sh        # -> plugin.wasm
cargo test        # pure-logic unit tests (URLs, search parsing, extraction), host-side
```

Host-side verification (from the workspace root):

```sh
cargo test -p sonora-plugin --test genius   # sandbox: scripted search+pages, misses, gating
cargo test -p sonora-plugin --test genius -- --ignored   # live Genius check (network-dependent)
```
