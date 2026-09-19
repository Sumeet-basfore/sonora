# Sonora

<p align="center">
  <strong>The Sovereign, Bit-Perfect Audiophile Music Player & Instrument</strong>
</p>

<p align="center">
  <a href="https://github.com/Sumeet-basfore/sonora/releases/latest"><img src="https://img.shields.io/github/v/release/Sumeet-basfore/sonora?color=6366f1&label=Release&style=flat-square" alt="Release"></a>
  <a href="LICENSE-MIT"><img src="https://img.shields.io/badge/License-MIT%20%2F%20Apache--2.0-blue?style=flat-square" alt="License"></a>
  <a href="https://github.com/Sumeet-basfore/sonora"><img src="https://img.shields.io/badge/Local--First-Zero%20Cloud%20Lock--in-emerald?style=flat-square" alt="Local-First"></a>
  <a href="https://github.com/Sumeet-basfore/sonora"><img src="https://img.shields.io/badge/Audio%20DSP-384kHz%20%2F%2032--bit%20Float-amber?style=flat-square" alt="Audio DSP"></a>
  <a href="https://github.com/Sumeet-basfore/sonora"><img src="https://img.shields.io/badge/Lyrics-Millisecond%20Synced-indigo?style=flat-square" alt="Synced Lyrics"></a>
</p>

---

## 🚀 Experience Sonora

<p align="center">
  <img src="docs/assets/sonora-showcase.gif" alt="Sonora v0.2 Showcase" width="100%" style="max-width: 1000px; border-radius: 12px; box-shadow: 0 20px 50px rgba(0,0,0,0.6);" />
</p>

---

## 📸 4 Modular Workspaces

Sonora provides dedicated, context-tuned workspaces — switch instantly with keyboard shortcuts or the workspace selector.

### 1. Audiophile Studio & Atmospheric Theater
| **Audiophile Studio** (64-Band Real-Time Spectrum, 384kHz/32-bit Float DSP) | **Atmospheric Theater** (Full-Bleed Artwork & Dynamic Synced Lyrics) |
| :---: | :---: |
| [![Audiophile Studio](docs/assets/screenshot-audiophile-studio.png)](docs/assets/screenshot-audiophile-studio.png) | [![Atmospheric Theater](docs/assets/screenshot-atmospheric-theater.png)](docs/assets/screenshot-atmospheric-theater.png) |

### 2. Modern Curator & Synced Lyrics Engine
| **Modern Curator** (Instant Library Search & MusicBrainz Metadata) | **Synced Lyrics View** (`Cmd + L`, Millisecond Precision Offset) |
| :---: | :---: |
| [![Modern Curator](docs/assets/screenshot-modern-curator.png)](docs/assets/screenshot-modern-curator.png) | [![Synced Lyrics](docs/assets/screenshot-lyrics-view.png)](docs/assets/screenshot-lyrics-view.png) |

### 3. Album Deep Dive & Minimal Player
| **Album Detail** (Multi-Provider Artwork & Hi-Res Metadata) | **Minimal Player** (Distraction-Free Desktop Transport) |
| :---: | :---: |
| [![Album Detail](docs/assets/screenshot-album-detail.png)](docs/assets/screenshot-album-detail.png) | [![Minimal Player](docs/assets/screenshot-minimal-player.png)](docs/assets/screenshot-minimal-player.png) |

---

## 🌟 What Makes Sonora Different?

Most modern music players are thin web wrappers for subscription streaming platforms. **Sonora is built on a single core principle: you own your music collection, and playing it should feel like operating a precision high-end instrument.**

* **Sovereign & Local-First**: No accounts, no telemetry, no tracking, no remote server dependency, and zero cloud lock-in.
* **True Bit-Perfect Pipeline**: Dual-decoder gapless engine (Symphonia + Rubato 64-bit sinc resampler) operating at up to 384kHz / 32-bit float with EBU R128 True Peak Limiting.
* **Decoupled 60 FPS Audio DSP**: Real-time 64-band logarithmic FFT spectrum analyzer running on a dedicated audio thread with lock-free ring buffer IPC (`rtrb`).
* **Universal Command Palette (`Cmd + K` / `/`)**: Sub-5ms instant local library queries via SQLite FTS5 alongside ranked online metadata matching (MusicBrainz, Cover Art Archive, Fanart.tv).
* **Millisecond Synced Lyrics (`Cmd + L`)**: Synchronized word/line scrolling with manual `±10ms` offset stepper and LRCLIB integration.
* **Sandboxed WASM Plugins**: Safe, capability-gated Wasmtime extension runtime. Crashing plugins will never interrupt playback.

---

## 📥 Download Sonora

Pre-built binaries are ready to download and run without compiling or installing developer toolchains.

👉 **[Download Sonora v0.1.0 Releases](https://github.com/Sumeet-basfore/sonora/releases/latest)**

### 🖥️ Desktop Applications
| Platform | Format | Package / Asset Name | Status |
| :--- | :--- | :--- | :--- |
| **Linux (x86_64)** | Universal AppImage | `Sonora-v0.1.0-linux-x86_64.AppImage` | **Primary (Fully Tested)** |
| **Linux (Debian/Ubuntu)** | DEB Package | `Sonora-v0.1.0-linux-x86_64.deb` | **Primary (Fully Tested)** |
| **Windows (x86_64)** | MSI Installer | `Sonora-v0.1.0-windows-x86_64.msi` | Supported |
| **Windows (x86_64)** | Portable Executable | `Sonora-v0.1.0-windows-x86_64.exe` | Supported |
| **macOS (Apple Silicon)** | DMG (Universal/ARM64) | `Sonora-v0.1.0-macos-aarch64.dmg` | Supported |
| **macOS (Intel)** | DMG (x86_64) | `Sonora-v0.1.0-macos-x86_64.dmg` | Supported |

### 📟 Terminal Tools (`sonora` CLI + `sonora-tui`)
Standalone bundles containing `sonora`, `sonora-tui`, and documentation:
* **Linux (x86_64):** `Sonora-v0.1.0-linux-x86_64-terminal.tar.gz`
* **Windows (x86_64):** `Sonora-v0.1.0-windows-x86_64-terminal.zip`
* **macOS (Apple Silicon):** `Sonora-v0.1.0-macos-aarch64-terminal.tar.gz`
* **macOS (Intel):** `Sonora-v0.1.0-macos-x86_64-terminal.tar.gz`

---

## 🚀 Quick Start & Installation

### Linux Desktop GUI
```bash
# AppImage (Universal)
chmod +x Sonora-v0.1.0-linux-x86_64.AppImage
./Sonora-v0.1.0-linux-x86_64.AppImage

# Debian / Ubuntu (.deb)
sudo dpkg -i Sonora-v0.1.0-linux-x86_64.deb
```

### Linux Terminal Tools
```bash
tar -xzf Sonora-v0.1.0-linux-x86_64-terminal.tar.gz
./sonora-tui          # Full-screen interactive TUI
./sonora status       # Scriptable CLI
```

### Windows
1. Download `Sonora-v0.1.0-windows-x86_64.msi` or `.exe`.
2. Run the installer and launch Sonora from your Start Menu.
3. For Terminal tools: extract `Sonora-v0.1.0-windows-x86_64-terminal.zip` and run `.\sonora-tui.exe`.

### macOS
1. Download `Sonora-v0.1.0-macos-aarch64.dmg` (Apple Silicon) or `Sonora-v0.1.0-macos-x86_64.dmg` (Intel).
2. Open the `.dmg` and drag `Sonora.app` into **Applications**.

### Release Integrity
Verify downloaded assets against `checksums.txt`:
```bash
# Linux / macOS
sha256sum -c checksums.txt

# Windows PowerShell
Get-FileHash Sonora-v0.1.0-windows-x86_64.msi -Algorithm SHA256
```

---

## 🎵 Supported Audio Formats

| Format | Extension | Decoder Engine | Metadata Tagging |
| :--- | :--- | :--- | :--- |
| **FLAC** | `.flac` | Symphonia (Bit-Perfect Lossless) | Lofty (Vorbis Comments) |
| **ALAC** | `.m4a`, `.alac` | Symphonia (Apple Lossless) | Lofty (MP4 iTunes Tags) |
| **WAV / AIFF** | `.wav`, `.aif`, `.aiff` | Symphonia (PCM 16/24/32-bit float) | Lofty (RIFF / ID3v2) |
| **MP3** | `.mp3` | Symphonia (MPEG Audio Layer III) | Lofty (ID3v1 / ID3v2.4) |
| **AAC** | `.aac`, `.m4a` | Symphonia (Advanced Audio Coding) | Lofty (MP4 Atoms) |
| **Ogg Vorbis** | `.ogg` | Symphonia (Vorbis Codec) | Lofty (Vorbis Comments) |
| **Opus** | `.opus` | Symphonia (Low-Latency Opus) | Lofty (Opus Tags) |

---

## 🛠️ For Developers: Build From Source

Building from source is completely optional. If you would like to hack on the audio engine or contribute:

### Prerequisites
* Rust stable toolchain (`rustup update stable`)
* Node.js 20+ & npm
* Linux system libraries: `sudo apt-get install -y libasound2-dev libwebkit2gtk-4.1-dev libjavascriptcoregtk-4.1-dev libsoup-3.0-dev`

### Clone & Build
```bash
git clone https://github.com/Sumeet-basfore/sonora.git
cd sonora

# Build entire Rust workspace (audio core, DSP, FTS5 indexer, CLI, TUI, Tauri desktop backend)
cargo build --workspace

# Build modern desktop frontend
npm --prefix apps/desktop install
npm --prefix apps/desktop run build

# Run applications locally
cargo run -p sonora-cli -- status    # Scriptable CLI
cargo run -p sonora-tui              # Interactive TUI
npm --prefix apps/desktop run tauri  # Desktop GUI
```

### Running Test Suite & Quality Checks
```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
npm --prefix apps/desktop test
```

---

## 🔌 Plugins, Themes & Extensions

* **WASM Plugins**: Sandboxed WebAssembly modules implementing the `sonora_plugin_*` ABI. See [`docs/05-plugin-system.md`](docs/05-plugin-system.md) and [`plugins/example/`](plugins/example/).
* **Theme Customization**: Pure JSON token schemas with hot-reloading CSS textures. See [`docs/06-theming-and-customization.md`](docs/06-theming-and-customization.md) and [`themes/sonora-retro/`](themes/sonora-retro/).
* **Community Registry**: Reproducible manifest format and SHA-256 package registry. See [`community-registry/`](community-registry/).

---

## 📚 Technical Specifications & Documentation

* [System Architecture Specification](docs/04-system-architecture.md)
* [Audio Engine & Real-Time DSP](docs/04-system-architecture.md#audio-engine)
* [Online Metadata & Enrichment Architecture](docs/23-online-metadata.md)
* [Universal Search & Keyboard Palette UX](docs/24-online-search-ux.md)
* [Artwork Retrieval & Multi-Provider Caching](docs/25-artwork-system.md)
* [Synchronized Lyrics Pipeline](docs/26-lyrics-experience.md)
* [Deterministic Metadata Matching & Scoring](docs/27-metadata-matching.md)
* [Architecture Decision Records (ADRs)](docs/12-decision-log.md)

---

## 📄 License

Sonora is dual-licensed under [MIT](LICENSE-MIT) and [Apache-2.0](LICENSE-APACHE).
