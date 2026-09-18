# Sonora v0.1.0 — Release Notes

**Release date:** 2026-09-18
**Tag:** `v0.1.0`
**License:** MIT OR Apache-2.0

---

## What Is Sonora?

Sonora is a local-first desktop music player built on a Rust audio engine and a
Tauri 2 desktop shell. It is designed for people who own their music and want a
player that is fast, correct, and customisable without being tied to a streaming
service, an account, or telemetry.

The v0.1.0 release delivers:

- A fully-functional desktop GUI (Linux primary; macOS / Windows best-effort)
- A terminal UI (`sonora-tui`) and a scriptable CLI (`sonora-cli`)
- A sandboxed WebAssembly plugin system with a community registry
- A theme and layout customisation engine
- Synchronised lyrics with five display modes
- An FFT-based spectrum visualiser

---

## Major Features

### Audio Engine
- **Symphonia** decodes audio frames on a dedicated Tokio task; decoded samples
  flow into a lock-free ring buffer (`rtrb`) and the CPAL output callback reads
  from it without ever allocating or locking.
- **CPAL** drives the system audio device. Sample-rate and channel-count
  mismatches are handled by the resampler in `sonora-dsp`.
- Pause, resume, seek, per-track volume, and queue transitions.
- Basic playback queue with next/previous, reorder, and add/remove.

### Library
- Recursive directory scanner powered by **Lofty** metadata extraction.
- **SQLite + FTS5** index with full-text search over title, artist, album, and
  genre.
- Albums, artists, and tracks views with artwork thumbnails.
- Artwork cache: thumbnails are generated once and served from disk; original
  artwork is kept available for full-resolution display.

### Synchronized Lyrics
Five display modes — Classic, Focused/Cinematic, Compact, Minimal, Dual-line —
with auto-scroll, active-line highlighting, click-to-seek, and a manual timing
offset adjustment per track.

**Resolution cascade (in priority order):**
1. SQLite lyrics cache (by `track_id`, file path, or title + artist)
2. Embedded tags (via Lofty)
3. Local sidecar `.lrc` file
4. LRCLIB public API
5. Plain-text fallback

### FFT Spectrum Visualiser
The `sonora-dsp` spectrum analyser runs in a Tokio task, reading from the ring
buffer without touching the CPAL callback. Frequency bins are pushed to the
frontend at a configurable FPS. Sensitivity, smoothing, height, bar count, and
colour source are all user-controlled.

### Theme & Layout System
- **Semantic design tokens** in JSON; live theme switching without restart.
- Built-in themes: **Midnight** (dark default), **Arctic** (light), plus
  **Neon Night** and **Sonora Retro** available via the registry.
- Import / export theme files.
- User-selectable accent colours.
- Layout regions (sidebar, library, queue, playback bar, visualiser, lyrics)
  can be shown or hidden independently; multiple named layouts can be saved and
  switched instantly.

### Album-Art Styles
Classic, Minimal, Large Immersive, Blurred Background, and Vinyl/Gatefold —
all consuming the same cached artwork source.

### Plugin System (`sonora-plugin`, `sonora-registry`)
- Plugins run in **WebAssembly sandboxes** managed by Wasmtime.
- Capabilities are declared in the plugin manifest and cannot exceed what the
  user explicitly grants.
- Five extension points: Metadata Source, Lyrics Provider, Visualiser Node,
  UI Widget, and DSP Node.
- Plugins support hot-reload and clean teardown.

### Interfaces

| Interface | Entry point | Notes |
|-----------|-------------|-------|
| Desktop GUI | `apps/desktop` | Tauri 2 + TypeScript; Linux primary |
| Terminal UI | `apps/tui` | Ratatui; keyboard-driven |
| CLI | `apps/cli` | `sonora scan`, `sonora play`, `sonora search`, `sonora queue` |

---

## Plugin Ecosystem

Three first-party plugins ship with v0.1.0. They are pre-built WASM artifacts
in `plugins/` and are also listed in the community registry.

| Plugin | ID | Category | Capability |
|--------|----|----------|------------|
| LRCLIB Lyrics Provider | `org.sonora.lrclib` | lyrics | `lyrics:provider`, `network:fetch` |
| Genius Lyrics Provider | `org.sonora.lyrics_genius` | lyrics | `lyrics:provider`, `network:fetch` |
| Spectrum+ Visualizer | `org.sonora.spectrum_plus` | visualizer | `visualizer:tap` |

The `plugins/example/` directory contains a minimal annotated WASM plugin with
source (WAT) intended as a starting point for plugin authors.

---

## Marketplace / Community Registry

The community registry (`community-registry/`) is a **Git-backed, offline-first
catalog**. No account is required to browse or install extensions.

| Path | Contents |
|------|----------|
| `community-registry/v1/plugins.json` | Plugin catalog |
| `community-registry/v1/themes.json` | Theme catalog |
| `community-registry/packages/*.zip` | Deterministic, sha256-verified packages |
| `community-registry/sources/` | Bundled source themes |
| `community-registry/tools/build.py` | Reproducible package builder |

The CI job verifies that `v1/` and `packages/` are reproducible: re-running
`build.py` must produce an identical diff to nothing.

### v0.1.0 registry contents

**Plugins (3):**
- `org.sonora.lrclib` 1.0.0 — LRCLIB synced/plain lyrics
- `org.sonora.spectrum_plus` 1.0.0 — Spectrum+ bars/wave/mirror visualiser
- `org.sonora.lyrics_genius` 1.0.0 — Genius lyrics

**Themes (2):**
- `org.sonora.theme.neon_night` 1.0.0 — Dark neon cyan/magenta
- `org.sonora.theme.retro` 1.0.0 — Amber-phosphor CRT aesthetic

---

## Supported Audio Formats

Decoding is handled by **Symphonia** with the `all` feature flag.

| Format | Container | Notes |
|--------|-----------|-------|
| FLAC | `.flac` | Lossless; all standard bit depths |
| MP3 | `.mp3` | MPEG Layer 3 |
| WAV / PCM | `.wav` | Uncompressed PCM |
| OGG Vorbis | `.ogg` | |
| Opus | `.opus` | |
| AAC | `.m4a`, `.aac` | |
| ALAC | `.m4a` | Apple Lossless |

Formats technically supported by Symphonia but not explicitly tested in v0.1.0:
MP4 / M4A (generic), MP2, WavPack, CAF.

---

## Platform Support

| Platform | Status | Audio backend | Notes |
|----------|--------|---------------|-------|
| **Linux** (x86-64) | ✅ Primary | ALSA via CPAL | Requires `libasound2-dev`, `libwebkit2gtk-4.1-dev` |
| macOS (x86-64, Apple Silicon) | ⚠️ Best-effort | CoreAudio via CPAL | Builds and runs; no dedicated CI runner |
| Windows 10+ (x86-64) | ⚠️ Best-effort | WASAPI via CPAL | Builds and runs; no dedicated CI runner |

---

## Known Limitations

- **Gapless playback** is not implemented. There is a brief gap between tracks.
- **ReplayGain / loudness normalisation** is not applied.
- **MusicBrainz / AcousticID** automatic lookup is not implemented.
- **Podcast and audiobook** chapter handling is absent.
- **Multi-window / mini-player** is not supported.
- **System tray integration** is not implemented.
- **Remote / cloud streaming** is out of scope for v0.1.x.
- Plugin **hot-reload** is defined in the API but requires a restart to take
  effect in this release.
- macOS and Windows receive only best-effort testing.
- The **TUI** supports library listing and basic playback control; lyrics and
  visualiser are not rendered in the TUI.

---

## Installation

### Prerequisites

**Linux:**
```sh
sudo apt-get install -y \
  libasound2-dev \
  libwebkit2gtk-4.1-dev \
  libjavascriptcoregtk-4.1-dev \
  libsoup-3.0-dev
```

**All platforms:** Rust stable toolchain + Node.js 20+

### Build from source

```sh
git clone https://github.com/sonora-audio/sonora
cd sonora

# Rust workspace
cargo build --workspace

# Desktop frontend
cd apps/desktop && npm install && npm run build
# Launch: cargo tauri dev  (from apps/desktop)
```

### Run tests

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace

npm --prefix apps/desktop run build
npm --prefix apps/desktop test
```

---

## First Run

1. Launch the desktop application.
2. Click **"Add Music Folder"** in the empty-library welcome screen.
3. Select a directory; the scanner will index tracks, extract metadata, and
   cache artwork.
4. Click any album or track to begin playback.

---

## Contributor and Plugin Developer Links

| Resource | Location |
|----------|----------|
| Architecture overview | `docs/04-system-architecture.md` |
| Plugin system spec | `docs/05-plugin-system.md` |
| Theme / customisation spec | `docs/06-theming-and-customization.md` |
| Lyrics system spec | `docs/07-lyrics-system.md` |
| Marketplace spec | `docs/08-marketplace.md` |
| Plugin API crate | `crates/sonora-plugin/` |
| Registry crate | `crates/sonora-registry/` |
| Example WASM plugin | `plugins/example/` |
| Community registry | `community-registry/` |
| Registry package builder | `community-registry/tools/build.py` |
| Development roadmap | `docs/11-development-roadmap.md` |
| Decision log | `docs/12-decision-log.md` |
| CI workflow | `.github/workflows/ci.yml` |

### Writing a plugin

1. Copy `plugins/example/` as your starting point.
2. Implement the `sonora_plugin_*` ABI functions defined in
   `crates/sonora-plugin/src/api.rs`.
3. Declare required capabilities in `manifest.json`.
4. Build to `wasm32-unknown-unknown` (see any `build.sh` in `plugins/`).
5. Submit a PR to `community-registry/` with your package zip and an entry in
   `v1/plugins.json`.

### Writing a theme

1. Copy `themes/sonora-retro/` or `community-registry/sources/neon-night/`.
2. Edit `theme.json` (semantic design tokens) and optionally `theme.css`.
3. Import via **Settings → Themes → Import** or submit to `v1/themes.json`.

---

*Sonora v0.1.0 — local-first, no telemetry, no accounts.*
