# Sonora

Local-first desktop and terminal music player: a Rust audio engine driving a modern desktop GUI, an interactive terminal UI, and a scriptable CLI. Audiophile-grade playback, synchronized lyrics, dynamic theming, sandboxed WASM plugins, and a community extension marketplace — no accounts, no telemetry, no cloud streaming lock-in.

---

## 📥 Download

Pre-built releases are ready to download and run without compiling or installing developer tools.

👉 **[Download the Latest Release (v0.1.0)](https://github.com/Sumeet-basfore/sonora/releases/latest)**

### Desktop Applications (Graphical)

| Operating System | Package Format | Download Links |
|---|---|---|
| **Linux (x86_64)** *(Primary)* | Universal AppImage<br>Debian / Ubuntu (.deb) | [AppImage](https://github.com/Sumeet-basfore/sonora/releases/latest)<br>[Debian Package (.deb)](https://github.com/Sumeet-basfore/sonora/releases/latest) |
| **Windows (x86_64)** *(Best-effort)* | Windows Installer (.msi)<br>Setup Executable (.exe) | [MSI Installer](https://github.com/Sumeet-basfore/sonora/releases/latest)<br>[Setup EXE](https://github.com/Sumeet-basfore/sonora/releases/latest) |
| **macOS** *(Best-effort)* | Apple Silicon (M1/M2/M3)<br>Intel (x86_64) | [Apple Silicon DMG](https://github.com/Sumeet-basfore/sonora/releases/latest)<br>[Intel DMG](https://github.com/Sumeet-basfore/sonora/releases/latest) |

### Terminal Applications (`sonora` CLI + `sonora-tui`)

Includes both the full-screen terminal player (`sonora-tui`) and the scriptable command-line utility (`sonora`) in a single bundle:

* **Linux (x86_64):** `Sonora-v0.1.0-linux-x86_64-terminal.tar.gz`
* **Windows (x86_64):** `Sonora-v0.1.0-windows-x86_64-terminal.zip`
* **macOS (Apple Silicon & Intel):** `Sonora-v0.1.0-macos-aarch64-terminal.tar.gz` / `Sonora-v0.1.0-macos-x86_64-terminal.tar.gz`

---

## 🚀 Quick Install & Run

### Desktop GUI

#### Linux AppImage (Universal)
```sh
chmod +x Sonora-v*-linux-x86_64.AppImage
./Sonora-v*-linux-x86_64.AppImage
```

#### Linux Debian/Ubuntu (.deb)
```sh
sudo dpkg -i Sonora-v*-linux-x86_64.deb
```

#### Windows & macOS
* **Windows:** Run the `.msi` or `.exe` installer.
* **macOS:** Open the `.dmg` and drag `Sonora.app` to Applications.

### Terminal (CLI & TUI)

Extract the archive and run directly:
```sh
# Run the full-screen terminal player:
./sonora-tui

# Run CLI status or scan commands:
./sonora status
./sonora scan /path/to/music
```

📖 Full step-by-step setup and PATH instructions: **[docs/INSTALLATION.md](docs/INSTALLATION.md)**

---

## 🎵 Supported Formats

FLAC, ALAC, WAV, AIFF, MP3, AAC/M4A, Ogg Vorbis — decoded with **Symphonia**, tagged with **Lofty**. Corrupted or unreadable files are logged and skipped safely without stopping library scans.


---

## ✨ Key Features

* **Real-Time Audio Engine** — dedicated audio thread with zero allocation or mutex locking in the CPAL render loop, lock-free ring buffer (`rtrb`), 10-band parametric EQ, and lock-free visualizer taps.
* **Instant Library Search** — SQLite + FTS5 full-text search index, instant fuzzy search over title, artist, album, and genre, automatic album art caching.
* **Synchronized Lyrics** — 5 display modes (Classic, Cinematic, Compact, Minimal, Dual-line), live timing synchronization, click-to-seek, manual offset calibration, and local `.lrc` / LRCLIB cascade.
* **Visualizer Suite** — FFT-based spectrum analysis off the real-time audio thread, frame-rate independent rendering.
* **Customization & Themes** — semantic color tokens, light/dark modes, customizable accent colors, responsive layout toggles, 5 album art display styles.
* **Sandboxed WASM Plugins** — capability-gated Wasmtime sandbox with strict memory and system call constraints; crashing plugins never interrupt playback.
* **Offline-First Marketplace** — Git-backed registry (`community-registry/`) with SHA-256 checksum-verified extensions, rollback, and uninstallation.

---

## 🛠️ For Developers: Build From Source

Building from source is completely optional. If you want to contribute or build locally:

### Prerequisites
* Rust stable toolchain (`rustup update stable`)
* Node.js 20+
* Linux dependencies: `sudo apt-get install -y libasound2-dev libwebkit2gtk-4.1-dev libjavascriptcoregtk-4.1-dev libsoup-3.0-dev`

### Build & Run
```sh
git clone https://github.com/Sumeet-basfore/sonora.git
cd sonora

# Build entire workspace (engine, library, lyrics, plugin host, CLI, TUI, desktop)
cargo build --workspace

# Build desktop frontend
npm --prefix apps/desktop install
npm --prefix apps/desktop run build

# Run applications
cargo run -p sonora-cli -- status    # Scriptable CLI
cargo run -p sonora-tui              # Interactive TUI
npm --prefix apps/desktop run tauri  # Desktop GUI
```

### Running Tests
```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
npm --prefix apps/desktop test
```

---

## 🔌 Plugin & Theme Development

* **Plugin Architecture:** Sandboxed WASM modules implementing the `sonora_plugin_*` ABI. See [`docs/05-plugin-system.md`](docs/05-plugin-system.md) and [`plugins/example/`](plugins/example/).
* **Theme Development:** Pure JSON token schema + sanitized CSS textures. See [`docs/06-theming-and-customization.md`](docs/06-theming-and-customization.md) and [`themes/sonora-retro/`](themes/sonora-retro/).
* **Community Registry:** Reproducible package generator and manifest schema. See [`community-registry/`](community-registry/).

---

## 📚 Documentation & Roadmap

* [System Architecture Specification](docs/04-system-architecture.md)
* [Plugin System & Sandboxing](docs/05-plugin-system.md)
* [Theming & Design Tokens](docs/06-theming-and-customization.md)
* [Lyrics System Specification](docs/07-lyrics-system.md)
* [Community Marketplace Specification](docs/08-marketplace.md)
* [Security & Permissions Model](docs/09-security-and-permissions.md)
* [Development Roadmap](docs/11-development-roadmap.md)
* [Architecture Decision Records (ADRs)](docs/12-decision-log.md)
* [Release Notes](RELEASE_NOTES.md)

---

## 📄 License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE).
