# Sonora: Product Vision & Strategy

## 1. Product Thesis

The desktop music landscape is caught in a false dichotomy. On one side are legacy audiophile players (foobar2000, MusicBee, Winamp)—hyper-specialized, extensible, and bit-perfect, yet architecturally antiquated, visually stagnated, and largely locked to single operating systems. On the other side are modern streaming clients and their Electron-based wrappers—visually sleek, yet bloated, resource-intensive, cloud-dependent, telemetry-choked, and hostile to user customization.

**Sonora is the high-performance, modular audio canvas for listeners who care about craft, aesthetics, and sovereignty.**

Sonora reimagines desktop music playback by wedding **audiophile-grade audio architecture** and **local-first data ownership** with **fluid modern visual craft** and **deep modularity**. Built on a decoupled headless engine that drives both a GPU-accelerated desktop GUI and an ultra-responsive terminal (TUI) client, Sonora treats music not as an ephemeral streaming commodity, but as an immersive sensory experience.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              THE SONORA POSITION                            │
│                                                                             │
│               Aesthetic Fluidity & Modern UI                                │
│                            ▲                                                │
│                            │            ★ SONORA                            │
│         Streaming Clients  │         (Craft + Modularity +                  │
│       (Spotify, Apple Music)│          High Performance)                     │
│                            │                                                │
│   ◄────────────────────────┼────────────────────────►                       │
│    Monolithic / Closed     │       Modular / Extensible                     │
│                            │                                                │
│                            │   Legacy Powerhouses (foobar2000)              │
│       Minimalist TUIs      │                                                │
│        (cmus, MPD)         │                                                │
│                            ▼                                                │
│               Raw Functional Utility                                        │
└─────────────────────────────────────────────────────────────────────────────┘
```

Sonora exists because music lovers should never have to trade audio fidelity and memory efficiency for a beautiful interface, nor sacrifice customizability for ease of use.

---

## 2. Target Users & Personas

Sonora is deliberately engineered for three distinct, overlapping user archetypes who are currently underserved by mainstream tools:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           TARGET AUDIENCE MATRIX                            │
├──────────────────────┬──────────────────────┬───────────────────────────────┤
│ The Audiophile       │ The Aesthetic        │ The Terminal                  │
│ Collector            │ Customizer           │ Minimalist                    │
├──────────────────────┼──────────────────────┼───────────────────────────────┤
│ • Lossless libraries │ • Dynamic palettes   │ • Keyboard-driven workflows   │
│ • Bit-perfect DSP    │ • Fluid lyrics       │ • Lightweight TUI client      │
│ • Exact metadata     │ • Custom shaders     │ • Headless daemon / CLI       │
│ • Headphone EQ       │ • Modding & themes   │ • Low RAM / CPU footprint     │
└──────────────────────┴──────────────────────┴───────────────────────────────┘
```

### 2.1 Persona 1: "The Audiophile Collector" (Marcus, 34)
- **Profile**: Owns a curated 500GB+ library of FLAC, ALAC, and high-res audio files on a local NVMe or home NAS. Listens on high-end DACs and planar-magnetic headphones.
- **Current Pain Points**: Uses foobar2000 or DeaDBeeF. Frustrated by dated Windows 98-era UIs, broken high-DPI scaling on 4K monitors, fragile theme configurations, and the lack of native macOS/Linux parity.
- **Why Sonora Wins**: Bit-perfect output, zero-overhead audio pipeline, 10-band parametric EQ with 1-click AutoEQ headphone profile matching, gapless playback, and instant search across 100,000+ tracks without lag.

### 2.2 Persona 2: "The Aesthetic Customizer & Rice Enthusiast" (Elena, 23)
- **Profile**: Spends hours customizing desktop environments (r/unixporn, r/desktops, Obsidian, Spicetify). Values visual harmony, typography, dynamic cover-art color extraction, and animated lyrics.
- **Current Pain Points**: Uses Spicetify or Cider. Constantly frustrated when Spotify updates break their CSS/JS patches; annoyed by 1GB+ RAM consumption for simple audio playback; hates closed DRM ecosystems.
- **Why Sonora Wins**: Dynamic album-art palette adaptation, hardware-accelerated shaders (projectM/Milkdrop presets, CAVA spectrum visualizers), Apple Music-grade syllable-by-syllable synchronized lyrics, and a clean, sandboxed theme/plugin architecture.

### 2.3 Persona 3: "The Terminal Minimalist & Power Hacker" (Devon, 28)
- **Profile**: Software engineer who lives in tmux, Neovim, and Alacritty/Kitty. Wants distraction-free, lightning-fast audio control via keyboard shortcuts and terminal interfaces.
- **Current Pain Points**: Uses cmus or raw MPD with ncmpcpp. Struggles with clunky lyrics workarounds, brittle ASCII art hacks, and having to maintain separate configurations when switching between laptop terminal and desktop graphical listening.
- **Why Sonora Wins**: A unified headless engine powering both a full GUI and a first-class TUI with Vim keybindings, ASCII/Braille audio visualizers, synchronized text lyrics, and complete IPC scriptability.

---

## 3. The Core Problem: The Desktop Audio Trilemma

Modern desktop music playback suffers from a structural trilemma:

```
                          Aesthetics & Modern UX
                                 ▲
                                / \
                               /   \
                              /     \
                             /   ★   \  Sonora solves
                            /  SONORA \ all three
                           /           \
                          /             \
                         /_______________\
      Extensibility &                    Performance &
      Audio Power                        Resource Efficiency
```

1. **The Legacy Audiophile Trap**: Players like foobar2000, Winamp, and MusicBee offer incredible DSP and modularity, but suffer from 20-year-old architectural baggage, platform lock-in (Win32), and inaccessible configuration that alienates all but the most hardcore tinkerers.
2. **The Modern Web/Electron Trap**: Players like Spotify, Cider, and Spicetify offer slick typography and active theming communities, but are plagued by memory bloat (500MB–1.5GB RAM), battery drain, sandboxed audio limits, and upstream breakage.
3. **The Minimalist CLI Trap**: Tools like cmus and MPD offer sub-10MB footprints and rock-solid stability, but sacrifice visual immersion, lyrics synchronization, dynamic artwork presentation, and approachable customization.

**Sonora breaks the trilemma** by separating a high-performance native audio/database daemon from lightweight, GPU-accelerated graphical and terminal frontends.

---

## 4. Product Principles

These six non-negotiable principles govern every product, UX, and architectural decision in Sonora:

### I. Sound Above All (Audio Pipeline Sanctity)
Audio playback is sacred. Decoding, DSP processing, and audio output run on an isolated, high-priority thread that **never** shares memory or execution loops with UI rendering, disk I/O, or network fetchers. A heavy UI animation, database re-index, or third-party plugin crash must never cause an audible pop, click, or stutter.

### II. Local-First, Sovereign, and Private
Sonora is a tool for the user, not a platform for data harvesting. There are no mandatory accounts, no surveillance telemetry, no analytics beacons, and no cloud dependencies for local playback. Metadata, settings, playlists, and plugin states reside in standard, human-inspectable files and SQLite databases.

### III. Ruthless Resource Efficiency
Sonora rejects the normalization of bloated desktop software. The headless core must idle at under 15MB RAM; the complete graphical client must operate under 80MB RAM during active playback. The player must launch in under 200ms and deliver instant 60/120 FPS rendering without burning CPU cycles on background garbage collection.

### IV. Visual Craft as an Instrument of Immersion
Aesthetics are not superficial skins; they deepen the connection between listener and music. Cover art dynamically informs the ambient canvas and UI palette with strict contrast guarantees. Lyrics move with fluid physical kinetics. Visualizers react with sub-millisecond precision to audio frequency bands.

### V. Modularity Without Fragility
Everything in Sonora is composable. UI panels, metadata fetchers, visualizers, and themes can be snapped, detached, replaced, and shared. However, unlike legacy players whose configurations break across versions, Sonora enforces strict semantic versioning and declarative schema contracts for all extensions.

### VI. Sandboxed Extensibility
Plugins must never destabilize the core player. Extensions run in isolated sandboxes with explicit capability permissions (e.g., network access, metadata mutation). A crashed plugin fails gracefully in its own sandbox without taking down the audio stream or the UI shell.

---

## 5. What Sonora Is vs. What Sonora Is NOT

To avoid product bloat and maintain laser-focused execution, we strictly delineate Sonora's boundaries:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          PRODUCT BOUNDARY DEFINITION                        │
├──────────────────────────────────────┬──────────────────────────────────────┤
│ What Sonora IS                       │ What Sonora is NOT                   │
├──────────────────────────────────────┼──────────────────────────────────────┤
│ • A high-performance desktop music   │ • A Spotify/Apple Music streaming    │
│   player and library manager         │   service clone                      │
│ • A local-first audio powerhouse with│ • A social network or algorithmic    │
│   audiophile DSP (Parametric/AutoEQ) │   recommendation feed                │
│ • A dynamic, themeable visual canvas │ • A DRM bypass or music piracy tool  │
│   for album art, lyrics, and shaders │ • A Digital Audio Workstation (DAW)  │
│ • A unified daemon powering both GUI │ • An ad-supported media platform    │
│   and terminal (TUI) interfaces      │ • A cloud music storage provider     │
└──────────────────────────────────────┴──────────────────────────────────────┘
```

### 5.1 Explicit Non-Goals

1. **NO Social Feeds or Algorithmic Discovery Radios**: Sonora will not build friend-activity feeds, social comment sections, or proprietary ML recommendation engines. Music discovery belongs in dedicated tools or open-source community plugins.
2. **NO DRM Decryption / Stream Ripping**: Sonora will not include tools to decrypt proprietary Widevine DRM streams (e.g., Spotify, Apple Music, Tidal proprietary streams) or bypass copyright protections.
3. **NO Digital Audio Workstation (DAW) Capabilities**: Sonora is a playback, listening, and library environment, not an audio production suite. Multi-track recording, MIDI sequencing, and DAW editing are strictly out of scope.
4. **NO Cloud Hosting or Storage Provider Lock-in**: Sonora will not operate a proprietary paid cloud locker service. Users own their storage (local drives, NAS, or personal Subsonic/Navidrome servers).
5. **NO Mandatory Telemetry or Monetization Funnels**: There will be no ads, no user tracking, no data monetization, and no "pro" subscription paywalls on core player functionality.

---

## 6. High-Level Core Capabilities

Sonora is organized around six foundational capabilities:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           SONORA CORE CAPABILITIES                          │
├─────────────────────────────────────────────────────────────────────────────┤
│ 1. Audiophile Audio Engine & DSP Chain                                      │
│    Bit-perfect playback, gapless transitions, 10-band parametric EQ,        │
│    AutoEQ headphone preset matching, EBU R128/ReplayGain loudness           │
├─────────────────────────────────────────────────────────────────────────────┤
│ 2. Sub-Millisecond Library Engine                                           │
│    SQLite FTS5-backed metadata indexing, instant search across 100k+        │
│    tracks, multi-folder watching, tag parsing (FLAC, MP3, AAC, OPUS, OGG)   │
├─────────────────────────────────────────────────────────────────────────────┤
│ 3. Atmospheric Visual Canvas & Dynamic Theming                             │
│    Real-time palette extraction from cover art, ambient shader backdrops,   │
│    CSS token hierarchy, responsive album art presentation                   │
├─────────────────────────────────────────────────────────────────────────────┤
│ 4. State-of-the-Art Lyrics Engine                                           │
│    Word/syllable karaoke timing, spring-physics auto-scroll, multi-source   │
│    fallback (embedded -> local .lrc/.ttml -> LRCLIB), romaji/furigana       │
├─────────────────────────────────────────────────────────────────────────────┤
│ 5. Dual Desktop GUI & Terminal TUI Frontends                                │
│    Modular graphical interface + lightning-fast terminal TUI with Vim keys  │
│    and CAVA-inspired ASCII/Unicode visualizers, powered by a single daemon  │
├─────────────────────────────────────────────────────────────────────────────┤
│ 6. Sandboxed Plugin & Theme Registry                                        │
│    Declarative manifests, capability permissions, hot-reloadable JS/WASM    │
│    runtime, and decentralized community package indexing                    │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 7. Future Ecosystem Vision (Phased Horizons)

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           THE THREE-HORIZON VISION                          │
│                                                                             │
│   HORIZON 1: THE CORE (MVP)                                                 │
│   • Headless audio core + SQLite indexer                                    │
│   • Dual GUI & TUI playback clients                                         │
│   • 10-band Parametric EQ & ReplayGain                                      │
│   • Dynamic cover-art theming & synced LRC lyrics                           │
│                      │                                                      │
│                      ▼                                                      │
│   HORIZON 2: THE EXTENSIBLE PLATFORM                                        │
│   • Sandboxed JS/WASM plugin runtime & Theme Studio                         │
│   • LRCLIB & TTML syllable-level karaoke sync                               │
│   • AutoEQ database integration & projectM Milkdrop visualizer              │
│   • Subsonic / Navidrome remote audio backend source                        │
│                      │                                                      │
│                      ▼                                                      │
│   HORIZON 3: THE LIVING AUDIO ECOSYSTEM                                     │
│   • Multi-room headless daemon streaming (Sonora Node network)              │
│   • Real-time vocal separation / isolation (Apple Sing paradigm)            │
│   • Modular DSP graph node editor (spatializers, custom VST3 bridges)       │
│   • Interactive Lyric Offset & Community Sync Contribution Studio           │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 8. Success Criteria & North Star Metrics

To ensure Sonora meets its promises of performance, craft, and stability, the product is measured against clear, objective guardrails:

### 8.1 Performance & Reliability Guardrails
- **Startup Time**: Cold launch to active playback in **< 200ms** (Desktop GUI) and **< 50ms** (TUI).
- **Memory Footprint**:
  - Headless Core / Daemon: **< 20MB RSS**
  - Terminal Client (TUI): **< 15MB RSS**
  - Graphical Client (GUI) during 120 FPS playback: **< 80MB RSS**
- **Zero Audio Glitches**: **0 buffer underruns** under simulated 100% UI and disk load.
- **Search Latency**: **< 5ms** fuzzy search response across a 100,000-track local library.

### 8.2 User Experience & Craft Standards
- **Contrast Guarantee**: 100% compliance with WCAG 2.1 AA contrast ratio across all dynamically generated album-art color schemes.
- **Lyrics Precision**: Sub-50ms synchronization alignment on word-level and line-level lyrics playback.
- **Visual Smoothness**: Consistent 60/120 FPS rendering on modern displays without GPU thermal throttling.

### 8.3 Ecosystem & Platform Health
- **Crash Isolation**: 100% of unhandled plugin exceptions caught without interrupting daemon playback or crashing the GUI shell.
- **Cross-Platform Parity**: Identical feature sets, keybindings, and audio output capabilities across Linux, macOS, and Windows.
