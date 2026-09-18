# Sonora: Product Requirements Document (PRD)

## 1. Document Overview & Scope Strategy

This document establishes the detailed Functional Requirements (FR), Non-Functional Requirements (NFR), System Boundaries, and Priority Matrices for Sonora. 

### 1.1 The Anti-Bloat Philosophy
Mainstream music clients continuously degrade in responsiveness because they attempt to be all-in-one social entertainment hubs. Sonora adopts a **ruthlessly focused product scope**:
- **Primary Core**: Playback fidelity, sub-millisecond library indexing, responsive theming, synchronized lyrics, and terminal/desktop accessibility.
- **Secondary Extensions**: Modularity, custom visualizers, AutoEQ presets, and plugin sandboxing.
- **Excluded**: Social feeds, DRM streaming reverse-engineering, recommendation engines, cloud lockers, and DAW audio creation.

---

## 2. Core Capabilities & Functional Requirements

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         SONORA CAPABILITY HIERARCHY                         │
├───────────────────┬─────────────────────────────────────────────────────────┤
│ Domain            │ Key Capabilities                                        │
├───────────────────┼─────────────────────────────────────────────────────────┤
│ Audio & DSP       │ Bit-perfect output, gapless, ReplayGain, Parametric EQ  │
│ Library & Storage │ SQLite FTS5 search, folder watch, embedded tag parsing  │
│ Visual & Theming  │ Palette extraction, ambient shaders, CSS token runtime  │
│ Lyrics System     │ Multi-source fallback, LRC/TTML sync, kinetic physics   │
│ Client Interfaces │ Desktop GUI (modular panels) & Terminal TUI (Vim keys)  │
│ Daemon & Engine   │ Headless daemon, socket IPC RPC, lockless visual tap    │
│ Extensibility     │ Sandboxed JS/WASM runtime, declarative plugin manifests │
└───────────────────┴─────────────────────────────────────────────────────────┘
```

---

### 2.1 Capability 1: Audio Engine & DSP Pipeline

The audio subsystem is the heart of Sonora. It must guarantee zero-glitch, bit-perfect playback across all platforms.

| Req ID | Priority | Requirement Description | Acceptance Criteria |
| :--- | :--- | :--- | :--- |
| **FR-AUD-01** | **P0 (MVP)** | **Codec Decoding Support** | Native decoding of lossless (FLAC, ALAC, WAV, AIFF) and lossy (MP3, AAC, Ogg Vorbis, Opus) audio files without external transcoding. |
| **FR-AUD-02** | **P0 (MVP)** | **Gapless Audio Playback** | Continuous sample-accurate transitions between tracks with matching sample rates; zero inserted silence or crossfade artifacts. |
| **FR-AUD-03** | **P0 (MVP)** | **Audio Backend Output** | Low-latency, bit-perfect audio output via PipeWire/PulseAudio/ALSA (Linux), CoreAudio (macOS), and WASAPI Exclusive/Shared (Windows). |
| **FR-AUD-04** | **P0 (MVP)** | **Loudness Normalization** | ReplayGain 2.0 and EBU R128 track/album gain normalization with customizable pre-amp and true-peak limiting to prevent clipping. |
| **FR-AUD-05** | **P0 (MVP)** | **10-Band Parametric Equalizer** | Real-time biquad filtering (Peaking, Low Shelf, High Shelf, High Pass, Low Pass) with interactive frequency curve control. |
| **FR-AUD-06** | **P1** | **AutoEQ Headphone Integration** | 1-click search and import of parametric EQ target curves from the AutoEQ headphone calibration database (5,000+ headphone models). |
| **FR-AUD-07** | **P0 (MVP)** | **Lockless Audio Tap Ring Buffer** | Zero-copy shared circular buffer broadcasting downsampled stereo PCM/FFT data to visualizer clients at 60/120 FPS without blocking the audio thread. |
| **FR-AUD-08** | **P2** | **Convolution Reverb & Room Correction**| Loading custom FIR impulse response `.wav` filters for room correction and crossfeed simulation. |

---

### 2.2 Capability 2: Library Indexing & Metadata Engine

Sonora treats large music libraries with database-grade indexing speed.

| Req ID | Priority | Requirement Description | Acceptance Criteria |
| :--- | :--- | :--- | :--- |
| **FR-LIB-01** | **P0 (MVP)** | **Multi-Folder Library Scanning** | Recursive directory indexing supporting custom user folder trees (local storage and mounted network shares). |
| **FR-LIB-02** | **P0 (MVP)** | **Embedded Tag Extraction** | Non-blocking extraction of ID3v2.3/v2.4, Vorbis Comments, MP4/AAC atoms, and APE tags (Title, Artist, Album, Album Artist, Year, Genre, Track Number, Disc Number, ReplayGain, Embedded Artwork). |
| **FR-LIB-03** | **P0 (MVP)** | **Sub-5ms Fuzzy Search** | SQLite FTS5 full-text indexing allowing instant matching across title, artist, album, and genre queries in < 5ms for 100,000+ tracks. |
| **FR-LIB-04** | **P0 (MVP)** | **Filesystem Watcher** | Real-time filesystem event monitoring (`inotify`/`fsevents`/`ReadDirectoryChangesW`) to automatically index added, renamed, or deleted files in the background. |
| **FR-LIB-05** | **P0 (MVP)** | **Playlist Management** | Import, export, and editing of standard playlist formats (`.m3u`, `.m3u8`) and native SQLite smart playlists (e.g., "Recently Added", "Top Played"). |
| **FR-LIB-06** | **P2** | **Remote Backend Source** | Connecting to Subsonic/Navidrome/Jellyfin remote servers as an indexed library source alongside local storage. |

---

### 2.3 Capability 3: Dynamic Visual Canvas & Theming

Visual presentation is immersive, hardware-accelerated, and strictly bound to readability standards.

| Req ID | Priority | Requirement Description | Acceptance Criteria |
| :--- | :--- | :--- | :--- |
| **FR-VIS-01** | **P0 (MVP)** | **Real-Time Palette Extraction** | Real-time extraction of Dominant, Vibrant, DarkVibrant, and Muted color swatches from active album art within < 30ms of track change. |
| **FR-VIS-02** | **P0 (MVP)** | **WCAG AA Contrast Enforcement** | Automatic lightness/chroma mathematical adjustment ensuring text and icon tokens maintain at least a 4.5:1 contrast ratio against dynamic background canvases. |
| **FR-VIS-03** | **P0 (MVP)** | **CSS Design Token Hierarchy** | Semantic CSS variable tokens (`--bg-canvas`, `--accent-primary`, `--text-primary`, `--panel-border`) driving the entire UI shell and custom themes. |
| **FR-VIS-04** | **P0 (MVP)** | **Ambient Canvas Glow Backdrop** | GPU fragment shader rendering smooth, breathing, multi-stop blurred gradients derived from album art in the background. |
| **FR-VIS-05** | **P0 (MVP)** | **CAVA-Inspired FFT Visualizer** | Logarithmically scaled frequency spectrum bars with configurable gravity, peak falloff, and color gradient ramps. |
| **FR-VIS-06** | **P1** | **projectM (Milkdrop) Visualizer** | Integrated OpenGL/Vulkan shader canvas rendering classic `.milk` visualizer presets at native display refresh rates (60/120/144Hz). |
| **FR-VIS-07** | **P1** | **Modular Docking Panel Layout** | Ability to rearrange, resize, collapse, and dock core panels (Library Tree, Track Table, Queue, Lyrics, Visualizer Canvas) via user presets. |

---

### 2.4 Capability 4: Lyrics Engine & Typography

Sonora provides the definitive lyrics experience on the desktop.

| Req ID | Priority | Requirement Description | Acceptance Criteria |
| :--- | :--- | :--- | :--- |
| **FR-LYR-01** | **P0 (MVP)** | **Multi-Tier Lyrics Resolution** | Cascading lyric lookup pipeline: (1) Embedded `SYLT`/`USLT` tags -> (2) Local `.lrc`/`.ttml` sidecar files in track folder -> (3) LRCLIB public API -> (4) Unsynced fallback. |
| **FR-LYR-02** | **P0 (MVP)** | **Line-Level Synced Scrolling** | Inertial spring-physics auto-scrolling with current line magnification and past/future line opacity fading. |
| **FR-LYR-03** | **P1** | **Word/Syllable-by-Syllable Karaoke** | Fluid continuous typographic fill and scale animations for word-level timestamps (Enhanced LRC & TTML format). |
| **FR-LYR-04** | **P1** | **Interactive Lyric Seeking** | Clicking any line in the lyrics panel instantly seeks the audio playback engine to that exact timestamp. |
| **FR-LYR-05** | **P1** | **Multi-Lingual Transliteration** | Toggleable Romaji/Pinyin transcription and Furigana annotations for CJK (Chinese, Japanese, Korean) tracks. |
| **FR-LYR-06** | **P2** | **Lyric Timing Offset & Sync Editor** | In-app editor allowing users to adjust track offsets (±100ms) and export fixed `.lrc` files locally or submit to LRCLIB. |

---

### 2.5 Capability 5: Dual Interface (Desktop GUI & Terminal TUI)

Both graphical and terminal workflows are first-class citizens backed by the same engine.

| Req ID | Priority | Requirement Description | Acceptance Criteria |
| :--- | :--- | :--- | :--- |
| **FR-CLI-01** | **P0 (MVP)** | **Unified Headless Daemon (`sonorad`)** | Standalone headless daemon executing audio decoding, DSP, library database, and IPC socket server. |
| **FR-CLI-02** | **P0 (MVP)** | **High-Performance Desktop GUI (`sonora`)** | Hardware-accelerated desktop interface with album grid, fluid track list, dynamic lyrics canvas, and parametric EQ inspector. |
| **FR-CLI-03** | **P0 (MVP)** | **Terminal TUI Client (`sonora-tui`)** | Complete ncurses/ratatui-based terminal client featuring: Vim navigation (`h/j/k/l`, `/` search), track queue, synced line lyrics, and ASCII/Braille audio visualizers. |
| **FR-CLI-04** | **P0 (MVP)** | **CLI Control Utility (`sonora-cli`)** | Command-line tool for instantaneous scriptable playback control (`sonora-cli play`, `pause`, `next`, `volume +5`, `status --json`). |
| **FR-CLI-05** | **P1** | **Terminal Graphics Protocol Album Art** | Terminal cover art rendering supporting Kitty Graphics Protocol, Sixel, and iTerm2 inline images, with graceful ASCII/half-block fallback. |
| **FR-CLI-06** | **P0 (MVP)** | **OS Integration & Media Keys** | Native MPRIS2 (Linux), SMTC (Windows), and `MPNowPlayingInfoCenter` (macOS) media key and lockscreen control support. |

---

### 2.6 Capability 6: Extensibility & Plugin Architecture

Third-party extensions expand Sonora safely without threatening player stability.

| Req ID | Priority | Requirement Description | Acceptance Criteria |
| :--- | :--- | :--- | :--- |
| **FR-EXT-01** | **P1** | **Declarative Manifest Specification** | Plugins defined via `manifest.json` specifying extension type, permissions, entry points, and schema version. |
| **FR-EXT-02** | **P1** | **Sandboxed Plugin Runtime** | Plugins execute in an isolated runtime (WASM / QuickJS worker) with strict permission gates (`network`, `library:read`, `lyrics:resolve`). |
| **FR-EXT-03** | **P1** | **Hot-Reloadable Themes** | Custom themes (CSS variables, layout presets, shader files) load and hot-reload instantly without player restart. |
| **FR-EXT-04** | **P2** | **Community Theme & Plugin Registry** | Built-in decentralized package browser indexing community GitHub repositories with 1-click install, update, and rollback. |

---

## 3. Non-Functional Requirements (NFRs)

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          PERFORMANCE BUDGET MATRIX                          │
├────────────────────────────┬──────────────────┬─────────────────────────────┤
│ Metric                     │ Target Budget    │ Hard Limit (Failure)        │
├────────────────────────────┼──────────────────┼─────────────────────────────┤
│ GUI Cold Startup Time      │ < 150 ms         │ 300 ms                      │
│ TUI Cold Startup Time      │ < 30 ms          │ 80 ms                       │
│ Headless Core RAM (Idle)   │ < 15 MB          │ 30 MB                       │
│ Desktop GUI RAM (Playing)  │ < 60 MB          │ 100 MB                      │
│ Terminal TUI RAM (Playing) │ < 12 MB          │ 25 MB                       │
│ 100k-Track Search Latency  │ < 3 ms           │ 10 ms                       │
│ UI Render Frame Rate       │ 60 / 120 FPS     │ Drop below 55 FPS           │
│ Audio Buffer Underruns     │ 0 per 24h play   │ > 0 audible glitches        │
└────────────────────────────┴──────────────────┴─────────────────────────────┘
```

### 3.1 Performance & Latency
- **NFR-PERF-01**: Audio decoding and output pipeline must execute on a dedicated real-time thread with high OS scheduling priority (`SCHED_RR` / MMCSS).
- **NFR-PERF-02**: All SQLite database queries must be strictly asynchronous relative to the UI thread; database writes use Write-Ahead Logging (WAL) mode.
- **NFR-PERF-03**: Audio visualizer FFT calculations must consume < 2% CPU utilization on an average modern quad-core machine.

### 3.2 Security & Sandbox Isolation
- **NFR-SEC-01**: Plugins must never have direct unmediated access to filesystem roots, raw shell execution, or native arbitrary pointer manipulation.
- **NFR-SEC-02**: Network requests originating from plugins must be explicitly declared in the manifest and approved by the user upon installation.
- **NFR-SEC-03**: IPC communications between the daemon and clients must use authenticated local Unix Domain Sockets or Windows Named Pipes with strict user permissions.

### 3.3 Platform Portability & Packaging
- **NFR-PORT-01**: First-class support for Linux (x86_64, aarch64), macOS (Apple Silicon, Intel x86_64), and Windows 10/11 (x86_64).
- **NFR-PORT-02**: Clean standalone zero-dependency distribution packages: AppImage / Flatpak (Linux), `.dmg` / Homebrew Cask (macOS), and portable zip / MSI installer (Windows).

---

## 4. MVP Boundary & Phased Roadmap

To prevent feature creep and ensure immediate delivery of an exceptional product, development is divided into three distinct phases:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                             PHASED ROADMAP MATRIX                           │
├─────────────────────────────────────────────────────────────────────────────┤
│ PHASE 0: MVP (THE BULLETPROOF FOUNDATION)                                   │
│ ─────────────────────────────────────────────────────────────────────────── │
│ • Headless Daemon (`sonorad`) + IPC Socket Engine                           │
│ • Bit-perfect playback (FLAC, ALAC, WAV, MP3, AAC, Opus, Ogg)               │
│ • Gapless transitions + ReplayGain 2.0 / EBU R128 normalization             │
│ • 10-band Parametric Equalizer with graphical UI                            │
│ • SQLite FTS5 library indexer (100k+ tracks, sub-5ms search, auto-watcher)  │
│ • Album-art dynamic palette extraction with WCAG AA contrast enforcement    │
│ • Line-level synchronized LRC lyrics with spring auto-scroll                │
│ • Full Desktop GUI with ambient canvas backdrops & CAVA-inspired FFT bars   │
│ • Full Terminal TUI client with Vim keybindings & ASCII visualizer          │
│ • Native OS media controls (MPRIS2, SMTC, macOS NowPlaying)                 │
├─────────────────────────────────────────────────────────────────────────────┤
│ PHASE 1: FAST-FOLLOW (THE EXPANSION)                                        │
│ ─────────────────────────────────────────────────────────────────────────── │
│ • Syllable/word-by-word karaoke typography (Enhanced LRC / TTML)            │
│ • AutoEQ 5,000+ headphone preset database integration                       │
│ • projectM (Milkdrop) GPU shader visualizer canvas                          │
│ • Sandboxed JS/WASM plugin engine & theme loader                            │
│ • Modular panel drag-and-drop workspace layouts                             │
│ • Kitty / Sixel / iTerm terminal graphics protocol for TUI album art       │
│ • Asian typography transliteration (Romaji, Furigana, Pinyin)               │
├─────────────────────────────────────────────────────────────────────────────┤
│ PHASE 2: POST-MVP & ECOSYSTEM (THE LIVING PLATFORM)                         │
│ ─────────────────────────────────────────────────────────────────────────── │
│ • Decentralized community theme and plugin marketplace browser              │
│ • Subsonic / Navidrome remote audio backend integration                     │
│ • In-app lyric sync offset and timing editor with LRCLIB submission         │
│ • Neural vocal reduction / karaoke stem isolation hook                      │
│ • Multi-room daemon audio streaming across local network nodes              │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 5. User Stories & Acceptance Criteria

### Story 1: Lightning Search & Playback
- **As an** audiophile collector with 80,000 FLAC tracks across multiple NVMe drives,
- **I want to** press `Ctrl+F` (or `/` in TUI) and type "Kind of Blue",
- **So that** search results appear in under 5 milliseconds and playback starts instantly with bit-perfect gapless fidelity.
- **Acceptance Criteria**:
  1. Search query returns matching tracks and albums within < 5ms.
  2. Pressing Enter immediately begins track playback without UI freeze or audio delay.
  3. Sample-accurate transition to track 2 without silence or popping.

### Story 2: Immersive Themed Listening
- **As an** aesthetic customizer,
- **I want** the player's background canvas and accent colors to dynamically adapt to the currently playing album art,
- **So that** my desktop listening experience feels visually atmospheric and cohesive.
- **Acceptance Criteria**:
  1. Cover art palette is extracted in < 30ms on track transition.
  2. Background renders a smooth GPU-accelerated gradient glow derived from the artwork.
  3. All text, icons, and lyrics remain 100% WCAG AA contrast compliant.

### Story 3: Keyboard-Driven Terminal Playback
- **As a** developer living in the terminal,
- **I want to** run `sonora-tui` inside a tmux pane,
- **So that** I can browse albums using `h/j/k/l`, view synchronized lyrics, and see an ASCII spectrum visualizer while consuming less than 15MB RAM.
- **Acceptance Criteria**:
  1. TUI connects seamlessly to running `sonorad` daemon via Unix socket.
  2. Full navigation accessible through standard Vim keys.
  3. ASCII frequency bars render smoothly at 30+ FPS in terminal.
  4. Memory usage remains under 15MB RSS.

### Story 4: Audiophile Headphone Tuning
- **As a** critical listener using Sennheiser HD600 headphones,
- **I want to** enable the built-in parametric equalizer and adjust biquad filters,
- **So that** I achieve a balanced sound signature tailored to my listening hardware.
- **Acceptance Criteria**:
  1. 10-band parametric equalizer allows continuous frequency, gain (-12dB to +12dB), and Q-factor adjustment.
  2. Real-time audio changes apply without latency or distortion.
  3. Equalizer presets can be saved, named, and loaded.
