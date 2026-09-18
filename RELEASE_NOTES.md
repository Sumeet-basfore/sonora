# Sonora v0.1.0 — Release Notes

**Release date:** 2026-09-18  
**Tag:** `v0.1.0`  
**License:** MIT OR Apache-2.0  

---

## What Is Sonora?

Sonora is a local-first desktop and terminal music player built on a high-performance Rust audio engine and a modern Tauri desktop shell. It is designed for listeners who own their music and want a player that is fast, correct, and customizable without being tied to a streaming service, an account, or telemetry.

The v0.1.0 release delivers:

- A desktop graphical application (Linux primary; macOS / Windows best-effort)
- Standalone terminal tools: an interactive terminal player (`sonora-tui`) and a scriptable CLI (`sonora`)
- A sandboxed WebAssembly plugin system (`sonora-plugin`) with capability security
- An offline-first community extension registry (`community-registry/`)
- A full theme and layout customization engine
- Synchronized lyrics with 5 display modes and manual timing calibration
- An FFT-based spectrum visualizer off the real-time audio thread

---

## 📥 Downloadable Release Artifacts

Pre-built binaries are available for direct download without needing Rust or Node.js installed:

| Distribution Channel | File Name | Platform & Notes |
|---|---|---|
| **Desktop (AppImage)** | `Sonora-v0.1.0-linux-x86_64.AppImage` | Universal Linux x86_64 standalone package |
| **Desktop (DEB)** | `Sonora-v0.1.0-linux-x86_64.deb` | Debian / Ubuntu / Linux Mint installer package |
| **Desktop (Windows)** | `Sonora-v0.1.0-windows-x86_64.msi` / `.exe` | Windows 10/11 installer |
| **Desktop (macOS)** | `Sonora-v0.1.0-macos-aarch64.dmg` / `Sonora-v0.1.0-macos-x86_64.dmg` | macOS Apple Silicon / Intel disk images |
| **Terminal (Linux)** | `Sonora-v0.1.0-linux-x86_64-terminal.tar.gz` | Includes `sonora` CLI + `sonora-tui` |
| **Terminal (Windows)** | `Sonora-v0.1.0-windows-x86_64-terminal.zip` | Includes `sonora.exe` + `sonora-tui.exe` |
| **Terminal (macOS)** | `Sonora-v0.1.0-macos-aarch64-terminal.tar.gz` | Includes `sonora` + `sonora-tui` for macOS |
| **Verification** | `checksums.txt` | SHA-256 integrity checksums for all files |

Download link: **[github.com/Sumeet-basfore/sonora/releases/tag/v0.1.0](https://github.com/Sumeet-basfore/sonora/releases/tag/v0.1.0)**

---

## Major Features

### Audio Engine
- **Symphonia** decodes audio frames on a dedicated async task; decoded samples flow into a lock-free Single-Producer Single-Consumer (SPSC) ring buffer (`rtrb`).
- **CPAL** drives system audio output with **zero memory allocations and zero mutex locking** inside the real-time audio callback loop.
- Software resampler in `sonora-dsp` handles sample rate and channel mismatches seamlessly.
- Transport controls: pause, resume, seek, per-track volume, and queue transitions.
- Playback queue with next/previous, reordering, and item insertion/removal.

### Library & Search
- Recursive directory scanner powered by **Lofty** metadata extraction.
- **SQLite + FTS5** index with sub-millisecond full-text search over title, artist, album, and genre.
- Albums, artists, and tracks views with artwork thumbnails.
- Disk-backed artwork cache: thumbnails are resized and cached once to avoid repeated decode overhead.

### Synchronized Lyrics
Five display modes: **Classic**, **Focused / Cinematic**, **Compact**, **Minimal**, and **Dual-line**, featuring auto-scroll, active-line highlighting, click-to-seek, and per-track manual timing offset adjustment.

**Resolution cascade (in priority order):**
1. SQLite lyrics cache (by `track_id`, file path, or title + artist)
2. Embedded tags (via Lofty)
3. Local sidecar `.lrc` file
4. LRCLIB public API
5. Plain-text fallback

### FFT Spectrum Visualizer
The `sonora-dsp` spectrum analyzer runs in a background task reading from the lock-free audio tap, decoupled from the CPAL callback. Frequency bins are pushed to the frontend at a configurable FPS. Sensitivity, smoothing, height, bar count, and color source are user-configurable.

### Theme & Layout System
- **Semantic design tokens** in JSON; live theme switching without application restarts.
- Built-in themes: **Midnight** (dark default), **Arctic** (light), plus registry themes (**Neon Night**, **Sonora Retro**).
- Import / export custom theme JSON files.
- User-selectable accent colors.
- Configurable layout regions (sidebar, library, queue, playback bar, visualizer, lyrics) with named preset saving.

### Album-Art Styles
Five swappable styles: **Classic**, **Minimal**, **Large Immersive**, **Blurred Background**, and **Vinyl / Gatefold** — all consuming the same cached artwork source.

### Plugin System (`sonora-plugin`, `sonora-registry`)
- Sandboxed **WebAssembly** runtime powered by Wasmtime with strict capability gating.
- Capabilities are declared in `manifest.json` and enforced by host security interceptors (`lyrics:provider`, `visualizer:tap`, `network:fetch`).
- Crashing plugins are isolated and cannot interrupt audio playback.

### Interfaces

| Interface | Entry Point | Notes |
|---|---|---|
| Desktop GUI | `apps/desktop` | Tauri 2 + TypeScript frontend |
| Terminal UI | `apps/tui` | Ratatui full-screen terminal interface |
| CLI | `apps/cli` | Scriptable commands (`sonora status`, `sonora scan`, `sonora volume`) |

---

## Plugin Ecosystem & Community Registry

Three first-party plugins ship with v0.1.0 in the offline-ready registry:

| Plugin | ID | Category | Capabilities |
|---|---|---|---|
| **LRCLIB Provider** | `org.sonora.lrclib` | lyrics | `lyrics:provider`, `network:fetch` |
| **Genius Provider** | `org.sonora.lyrics_genius` | lyrics | `lyrics:provider`, `network:fetch` |
| **Spectrum+ Visualizer** | `org.sonora.spectrum_plus` | visualizer | `visualizer:tap` |

The `plugins/example/` directory provides an annotated WASM starter module for plugin developers.

---

## Supported Audio Formats

Decoded via **Symphonia** (`all` features):

| Format | Extension | Notes |
|---|---|---|
| FLAC | `.flac` | Lossless; all standard bit depths |
| ALAC | `.m4a` | Apple Lossless |
| WAV / PCM | `.wav` | Uncompressed PCM |
| AIFF | `.aiff` | Audio Interchange File Format |
| MP3 | `.mp3` | MPEG Layer 3 |
| AAC | `.m4a`, `.aac` | Advanced Audio Coding in MP4/ADTS |
| Ogg Vorbis | `.ogg` | Vorbis audio in Ogg container |


---

## Platform Support Matrix

| Platform | Support Status | Audio Backend | Notes |
|---|---|---|---|
| **Linux (x86_64)** | ✅ **Primary (Fully Tested)** | ALSA / PipeWire via CPAL | Automated CI test matrix & package bundling |
| **macOS (Apple Silicon & Intel)** | ⚠️ **Best-Effort** | CoreAudio via CPAL | Builds from portable codebase; no notarization configured yet |
| **Windows (x86_64)** | ⚠️ **Best-Effort** | WASAPI via CPAL | Builds from portable codebase; no code signing certificate configured |

---

## Known Limitations

- **Gapless playback** is not yet sample-accurate between different sample rates; there is a brief transition gap.
- **ReplayGain / loudness normalization** metadata is not applied during decode.
- **MusicBrainz / AcousticID** online fingerprinting is not implemented in v0.1.0.
- **Podcast and audiobook** chapter marks are not indexed.
- **System tray / mini-player** is deferred to future releases.
- **Remote / cloud streaming** is out of scope.
- **Plugin hot-reload** requires restarting the client in this release.
- **Code signing & notarization** are not configured in v0.1.0; macOS may show an unverified developer prompt.

---

## Installation Quick Start

### Linux AppImage
```sh
chmod +x Sonora-v0.1.0-linux-x86_64.AppImage
./Sonora-v0.1.0-linux-x86_64.AppImage
```

### Linux Debian/Ubuntu (.deb)
```sh
sudo dpkg -i Sonora-v0.1.0-linux-x86_64.deb
```

### Terminal Tools
```sh
tar -xzf Sonora-v0.1.0-linux-x86_64-terminal.tar.gz
cd sonora-terminal
./sonora-tui
```

Full installation guide: **[`docs/INSTALLATION.md`](docs/INSTALLATION.md)**

---

## Developer Links

| Resource | Location |
|---|---|
| Architecture Overview | [`docs/04-system-architecture.md`](docs/04-system-architecture.md) |
| Plugin System Spec | [`docs/05-plugin-system.md`](docs/05-plugin-system.md) |
| Theming Spec | [`docs/06-theming-and-customization.md`](docs/06-theming-and-customization.md) |
| Lyrics Spec | [`docs/07-lyrics-system.md`](docs/07-lyrics-system.md) |
| Marketplace Spec | [`docs/08-marketplace.md`](docs/08-marketplace.md) |
| Roadmap | [`docs/11-development-roadmap.md`](docs/11-development-roadmap.md) |
| Decision Log | [`docs/12-decision-log.md`](docs/12-decision-log.md) |

---

*Sonora v0.1.0 — local-first, no telemetry, no accounts.*
