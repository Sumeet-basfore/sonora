# Sonora: User Experience & Interaction Architecture

## 1. UX Design Philosophy & Core Tenets

Sonora's user experience bridges the gap between utilitarian audiophile tools and fluid, contemporary visual environments. The UX architecture adheres to five foundational pillars:

1. **Audio Immediacy (Zero-Friction Playback)**: Playback controls, queue manipulation, and search are always one interaction away. The player never blocks user input during library rescans, network fetches, or heavy visual rendering.
2. **Atmospheric Immersion**: Music is multi-sensory. The visual interface dynamically harmonizes with the album artwork's visual language through real-time palette extraction, ambient GPU canvas glows, and kinetic typography.
3. **Ergonomic Dual Paradigms**: Complete experiential parity between a hardware-accelerated GUI (mouse, touch, smooth kinetic scrolling) and a keyboard-centric TUI (Vim navigation, dense tabular data, ASCII visualizers).
4. **Non-Destructive Modularity**: Workspaces, panels, and sidebars can be docked, collapsed, resized, or popped out into floating windows without breaking layout cohesion or risking state corruption.
5. **Universal Accessibility & Readability**: Guaranteed WCAG 2.1 AA contrast on dynamically colored backgrounds, scalable typography, full keyboard navigation, and native screen reader / accessibility hooks across platforms.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           SONORA UX ARCHITECTURE MAP                        │
│                                                                             │
│                     ┌────────────────────────────────┐                      │
│                     │       Shared Audio Daemon      │                      │
│                     │       State & Event Bus        │                      │
│                     └───────┬────────────────┬───────┘                      │
│                             │                │                              │
│              IPC / Socket   │                │   IPC / Socket               │
│                             ▼                ▼                              │
│             ┌─────────────────────┐    ┌─────────────────────┐              │
│             │   Sonora GUI Shell  │    │  Sonora TUI Shell   │              │
│             ├─────────────────────┤    ├─────────────────────┤              │
│             │ • Modular Dock Grid │    │ • Grid Layout (TUI) │              │
│             │ • Kinetic Canvas    │    │ • Vim Navigation    │              │
│             │ • Syllable Karaoke  │    │ • Synced Line Text  │              │
│             │ • Shaders / Milkdrop│    │ • ASCII/Braille FFT │              │
│             │ • Palette Adapters  │    │ • ANSI Color Themes │              │
│             └─────────────────────┘    └─────────────────────┘              │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 2. Desktop GUI Architecture

### 2.1 Layout System & Modular Viewports

The Desktop GUI operates on a flexible grid container system structured around four fundamental workspaces:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         DESKTOP GUI DEFAULT WORKSPACE                       │
├───────────────────┬─────────────────────────────────────┬───────────────────┤
│ Top Navigation &  │ Global Search Bar (Ctrl+F)          │ Window Controls & │
│ View Switcher     │ Breadcrumb Navigation               │ Profile/Settings  │
├───────────────────┼─────────────────────────────────────┼───────────────────┤
│ Left Sidebar      │ Main Content Viewport               │ Right Inspector   │
│ • Library         │                                     │ • Active Queue    │
│   - Artists       │ [GridView / TableView / AlbumView]  │ • Synced Lyrics   │
│   - Albums        │                                     │ • Track Metadata  │
│   - Tracks        │                                     │ • Parametric EQ   │
│   - Folders       │                                     │ • FFT Spectrum    │
│ • Playlists       │                                     │                   │
│ • Plugins         │                                     │                   │
├───────────────────┴─────────────────────────────────────┴───────────────────┤
│ Bottom Dock: Persistent Playback & Transport Engine                         │
│ [Art] [Track Info] | [⏮ ⏯ ⏭] [Seekbar / Waveform] [Volume] | [Lyrics/Vis/EQ]│
└─────────────────────────────────────────────────────────────────────────────┘
```

#### Supported Core Layout Modes
1. **Default Tri-Pane Studio**: Left navigation, central library/album browser, right collapsible inspector (lyrics/queue/visualizer), bottom persistent transport.
2. **Full-Bleed Theater / Canvas Mode**: Central artwork or projectM/Milkdrop visualizer occupies full window; floating semi-transparent HUD displays playback status and animated syllable lyrics.
3. **Mini-Player / Compact Floating Widget**: Minimalist floating window (always-on-top capable) showing square album art, kinetic title marquee, scrub bar, and compact playback controls.
4. **Audiophile Inspector View**: Central high-density table view with customizable metadata columns (bit depth, sample rate, codec, DR score, ReplayGain values), paired with a real-time 10-band parametric EQ curve and spectrum analyzer.

### 2.2 Navigation Hierarchy & Information Architecture

```
Home / Dashboard
├── Library
│   ├── Artists (Grid / Alphabetical Index)
│   │   └── Artist Detail (Hero Header, Discography, Top Tracks, Related)
│   ├── Albums (Cover Grid with Dynamic Hover Badges)
│   │   └── Album Detail (Gatefold Art, Tracklist, Dynamic Color Ambient Canvas)
│   ├── Tracks (High-density Sortable/Filterable Table)
│   ├── Genres / Eras / Decades
│   └── Folder Directory Tree (Physical FS Browser)
├── Playlists
│   ├── User Playlists (Manual Ordering, Custom Covers, Smart Filters)
│   └── Auto-Generated / Smart Playlists (Recently Played, Top 100, Lossless High-Res)
├── Lyrics & Immersion Center (Dedicated Large Canvas View)
├── Visualizer Studio (Milkdrop 3 Presets, CAVA Spectrum, Custom GLSL Shaders)
├── DSP & Equalizer Studio (10-Band Parametric, AutoEQ Presets, Convolution)
└── Settings & Extensions (Themes, Plugins, Marketplace, Audio Output, Hotkeys)
```

---

## 3. Terminal TUI Architecture

### 3.1 Terminal Interface Model

The Sonora TUI client (`sonora-tui`) provides a lightning-fast, keyboard-driven interface optimized for terminal multiplexers (`tmux`, `zellij`) and high-density developer workflows.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           SONORA TUI LAYOUT MATRIX                          │
├───────────────────┬─────────────────────────────────────┬───────────────────┤
│ 1: Library        │ Track Title           Artist  Time  │ [Now Playing]     │
│ > All Tracks      │ 01. So What           M. Davis 9:22 │ Miles Davis       │
│   Albums          │ 02. Freddie Freeloader M. Davis 5:49 │ Kind of Blue      │
│   Artists         │ 03. Blue in Green     M. Davis 5:37 │ FLAC 24/96kHz     │
│   Playlists       │ 04. Flamenco Sketches M. Davis 9:26 │ ──────────────    │
│   File Browser    │                                     │ [Lyrics]          │
│ ───────────────── │                                     │ > It starts slow  │
│ 2: Visualizer     │                                     │   with the bass.. │
│ ▄█▄█■█▄█■█▄█■█▄   │                                     │                   │
├───────────────────┴─────────────────────────────────────┴───────────────────┤
│ [▶ PLAYING] 03:24 / 09:22 ━━━━━━━●━━━━━━━━━━━━━━━ [Vol: 85%] [EQ: Flat]    │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 3.2 Keybindings & Interaction Model (Vim-Style)

| Context | Keybinding | Action |
| :--- | :--- | :--- |
| **Global** | `Space` | Play / Pause toggle |
| **Global** | `j` / `k` (or `↓` / `↑`) | Move selection down / up |
| **Global** | `h` / `l` (or `←` / `→`) | Switch panes / collapse/expand trees |
| **Global** | `+` / `-` (or `]` / `[`) | Volume increase / decrease |
| **Global** | `>` / `<` (or `Shift+L`/`H`) | Seek forward / backward 5s |
| **Global** | `n` / `p` | Next track / Previous track |
| **Global** | `/` | Instant fuzzy search filter |
| **Global** | `Tab` / `Shift+Tab` | Cycle active focus across split panes |
| **Global** | `1` .. `5` | Switch active workspace (Library, Queue, Lyrics, Visualizer, EQ) |
| **Global** | `v` | Cycle visualizer rendering modes (Bars, Wave, Matrix, Off) |
| **Global** | `L` | Toggle full-screen lyrics mode |
| **Global** | `q` / `Ctrl+C` | Detach client (daemon keeps playing) / Quit |
| **Table** | `Enter` | Play selected item immediately |
| **Table** | `a` | Append selected track/album to queue |
| **Table** | `d` / `Delete` | Remove selected track from queue / playlist |

---

## 4. Visualizer & Immersion Experience

### 4.1 Visualizer Modalities

Sonora supports three distinct visualization modes across GUI and TUI:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           VISUALIZER ENGINE TIERS                           │
├───────────────────┬──────────────────────────┬──────────────────────────────┤
│ Engine            │ GUI Capability           │ TUI Capability               │
├───────────────────┼──────────────────────────┼──────────────────────────────┤
│ 1. Spectrum Bars  │ GPU-accelerated smooth   │ CAVA-inspired ASCII/Unicode  │
│    (CAVA Core)    │ gradient bars with peak  │ Braille bars (⡀⡄⡆⡇⣇⣧⣷⣿)       │
│                   │ hold and gravity physics │ with ANSI terminal colors    │
├───────────────────┼──────────────────────────┼──────────────────────────────┤
│ 2. projectM       │ Full Milkdrop 3 preset   │ N/A (Graceful degradation    │
│    (Milkdrop GL)  │ rendering (GLSL/Vulkan)  │ to high-rate FFT spectrum)   │
├───────────────────┼──────────────────────────┼──────────────────────────────┤
│ 3. Ambient Wave   │ Oscilloscope and stereo  │ Half-block Unicode wave line │
│    & Lissajous    │ phase correlation vector │ (Stereo Phase & RMS meters)  │
└───────────────────┴──────────────────────────┴──────────────────────────────┘
```

### 4.2 Dynamic Artwork & Palette Extraction UX

When a track loads, the UI dynamically extracts color swatches using an optimized quantization pipeline (Vibrant / Material You algorithm) and adapts the interface:

```
Album Cover Input ──► Palette Quantization ──► WCAG Contrast Audit ──► Token Cascade
                      ├─ Dominant (RGB)        ├─ Lightness Clamp      ├─ --bg-canvas
                      ├─ Vibrant (RGB)         ├─ Saturation Guard     ├─ --accent-primary
                      ├─ DarkVibrant (RGB)     └─ Alpha Channel        ├─ --text-primary
                      └─ Muted (RGB)                                   └─ --border-subtle
```

#### Contrast & Legibility Rules
- Primary and secondary text tokens are guaranteed a minimum contrast ratio of **4.5:1** against dynamically tinted backgrounds.
- If the extracted background is too bright or low-contrast, Sonora applies an automatic luma offset and alpha attenuation layer to maintain readability without washing out color richness.
- Users can choose between four palette reactivity profiles:
  - **Vibrant Ambiance**: Full background glow and dynamic accents.
  - **Subtle Accents**: Static dark/light background with dynamically tinted sliders, highlights, and buttons.
  - **Monochrome / Strict**: Static user theme overrides dynamic cover colors completely.
  - **OLED Pure Black**: Background locked to `#000000` with vibrant cover art highlights.

---

## 5. Lyrics Experience & Typography

### 5.1 Presentation Tiers

1. **Static Plain Text**: Smooth manual scrolling with customizable font family, line height, and font size.
2. **Line-Synchronized LRC**: Active line highlighted in high-contrast primary token, past lines slightly dimmed (60% opacity), future lines dimmed (40% opacity). Spring-physics auto-scroll keeps active line smoothly centered.
3. **Word/Syllable Karaoke (Enhanced LRC / TTML)**:
   - Words illuminate and expand smoothly with continuous fill-color transitions in lockstep with the audio playback clock.
   - Dual-line display for non-romanized scripts (Kanji/Hanzi with toggleable Furigana / Pinyin / Romaji above).
   - Click-to-seek: Clicking any word or line triggers a sample-accurate seek in the audio engine.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          KINETIC LYRICS CANVAS VIEW                         │
│                                                                             │
│                   Previous line faded in background (40%)                   │
│                                                                             │
│         ▶ █W█h█e█r█e   t█h█e   s█t█a█r█s   a█l█i█g█n   t█o█n█i█g█h█t        │
│           ▲                                                                 │
│           └─ Continuous syllable-fill animation (WCAG AA Compliant)         │
│                                                                             │
│                   Next lyric line waiting in queue (60%)                    │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 6. Equalizer & DSP User Experience

### 6.1 Interactive Equalizer Interface

The Parametric Equalizer combines professional precision with intuitive drag-and-drop curve sculpting:
- **Interactive Curve Graph**: 10 interactive node points plotted on a logarithmic frequency axis (20 Hz to 20,000 Hz). Dragging adjusts frequency (X-axis) and gain (Y-axis, ±12 dB); scroll wheel adjusts Q-factor / bandwidth.
- **Filter Type Selector per Band**: Low Pass, High Pass, Low Shelf, High Shelf, Peaking / Bell, Notch.
- **AutoEQ Headphone Preset Integration**: Search bar connected to the AutoEQ repository (5,000+ headphone models). Selecting a model instantly loads the parametric filter set or convolution FIR impulse curve.
- **Pre-Amp & True-Peak Protection**: Live visual meter showing gain before/after filtering with automatic pre-amp attenuation to prevent digital clipping.

---

## 7. OS Integration & Media Keys

### 7.1 Platform-Specific Integrations

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        PLATFORM INTEGRATION CAPABILITIES                    │
├─────────────────────┬───────────────────────┬───────────────────────────────┤
│ Feature             │ Linux                 │ macOS                         │
├─────────────────────┼───────────────────────┼───────────────────────────────┤
│ Media Keys & Remote │ MPRIS2 D-Bus Interface│ MPNowPlayingInfoCenter        │
│ Lockscreen Art      │ D-Bus Cover URI       │ Now Playing Lockscreen Art    │
│ System Tray / Menu  │ StatusNotifierItem    │ NSStatusItem (Menu Bar)       │
│ Desktop Notifs      │ notify-rust / D-Bus   │ NSUserNotification            │
├─────────────────────┼───────────────────────┼───────────────────────────────┤
│ Feature             │ Windows 10/11         │ Headless / CLI Remote         │
├─────────────────────┼───────────────────────┼───────────────────────────────┤
│ Media Keys & Remote │ SMTC (System Media)   │ Sonora Socket Protocol        │
│ Lockscreen Art      │ SMTC Thumbnail Stream │ Raw Socket Stream / Base64    │
│ System Tray / Menu  │ Win32 NotificationTray│ N/A                           │
│ Desktop Notifs      │ Windows Toast Notifs  │ Terminal Bell / OSC 777       │
└─────────────────────┴───────────────────────┴───────────────────────────────┘
```

---

## 8. UX Edge Cases & Error States

| Scenario | UX Handling Strategy |
| :--- | :--- |
| **Missing Album Artwork** | Procedurally generated geometric vinyl record with dynamic gradient derived from artist/album hash. |
| **Unsynced / Missing Lyrics** | Automatic fallback message with 1-click option to trigger manual search or open local `.lrc` file picker. |
| **Audio Output Device Disconnected** | Playback automatically pauses; unobtrusive notification appears offering device re-selection without throwing modal error. |
| **100k+ Track Bulk Import** | Progressive non-blocking background indexer with persistent progress pill in status bar; search is immediately queryable on already indexed records. |
| **Corrupted Audio File** | Playback skips to next track seamlessly after logging non-fatal toast notification ("Unable to decode track: corrupt header"). |
| **Daemon Reconnection** | GUI/TUI automatically reconnects with exponential backoff if `sonorad` is restarted, restoring queue position and playback state seamlessly. |

---

## 9. Reversibility & UX Boundaries

> [!IMPORTANT]
> ### Reversible UX Decisions
> - **Default Panel Arrangement**: Panel docking states are serialized in JSON configuration files and can be fully reset or swapped between layout presets without data loss.
> - **Palette Color Model**: Swatch extraction algorithms (ColorThief vs. Material You HCT) can be switched behind configuration flags without affecting the underlying CSS token schema.
> - **Visualizer Default Mode**: Users can toggle between Milkdrop, CAVA FFT bars, and minimal oscilloscope via single hotkey.
>
> ### Invariable UX Commitments
> - **Zero-Interrupt Audio**: UI latency, layout recalcs, or crash states must never cause audio buffer drops.
> - **WCAG AA Minimum Contrast**: Dynamic themes must always uphold 4.5:1 contrast for legible text elements.
