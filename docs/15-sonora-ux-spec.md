# Sonora User Experience Architecture & Interface Specification

## 1. Executive Summary & Design Vision

Sonora is architected as an ultra-responsive, visually stunning, and deeply customizable desktop music player. Its UX architecture bridges the best aspects of classic modular power players (foobar2000, MusicBee) with the visual elegance, fluid physics, and kinetic typography of modern streaming clients (Apple Music, Plexamp, Cider).

---

## 2. Core UX Subsystem Specifications

### 2.1 Primary Navigation & Spatial Model

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          PRIMARY NAVIGATION ARCHITECTURE                    │
│                                                                             │
│  ┌───┬────────────────────────────────────────────────────────────┬──────┐  │
│  │ ☰ │ ◀  ▶  Library  >  Rock  >  Pink Floyd  >  The Dark Side...  │ 🔍 ⚙ │  │
│  ├───┴────────────────────────────────────────────────────────────┴──────┤  │
│  │ [Icon Rail / Expanded Tree]      [Main Content Canvas]                 │  │
│  │ • 🏠 Home / Spotlight            • Album Grid / Track Table / Folder   │  │
│  │ • 📚 Library (Albums/Artists)    • Kinetic Syllable Lyrics Stage       │  │
│  │ • 📂 Folder File-System Tree     • Visualizer Canvas (Milkdrop/CAVA)   │  │
│  │ • 📜 Playlists & Smart Mixes     • Artist Discography & Liner Notes    │  │
│  │ • 🎛️ DSP Studio & Equalizer                                             │  │
│  │ • 🧩 Plugin & Theme Marketplace                                        │  │
│  └───┬────────────────────────────────────────────────────────────┬──────┘  │
│      │ [Dynamic Now Playing & Waveform Control Dock]               │        │
│      └────────────────────────────────────────────────────────────┘        │
└─────────────────────────────────────────────────────────────────────────────┘
```

#### Navigation Primitives:
1. **Collapsible Navigation Rail**: Toggles between a compact 56px icon rail and an expanded 220px hierarchical tree.
2. **Global Command Palette (`Ctrl+K` / `Cmd+K`)**: Instant fuzzy search across tracks, albums, artists, DSP presets, layout switches, and settings.
3. **Breadcrumb Trail & History**: Clear navigation hierarchy with back/forward history tracking (`Alt+Left` / `Alt+Right`).
4. **Multi-Tab Workspaces**: Ability to open multiple playlists or library queries in tabbed views.

---

### 2.2 Library Organization & Multi-View Engine

Sonora provides four interchangeable view modes for any collection:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          LIBRARY VIEW PARADIGMS                             │
├──────────────────────┬──────────────────────────────────────────────────────┤
│ 1. Album Grid        │ High-res art cards, release year badges, dynamic     │
│    (Curator Mode)    │ card hover actions, animated vinyl slip-out preview. │
├──────────────────────┼──────────────────────────────────────────────────────┤
│ 2. Track Data Table  │ High-density spreadsheet (24px rows), customizable   │
│    (Power Mode)      │ columns (BPM, Key, Bitrate, ReplayGain, Play Count). │
├──────────────────────┼──────────────────────────────────────────────────────┤
│ 3. Hybrid Expanded   │ Album art header with track list embedded directly   │
│    (Vinyl/CD Box)    │ beneath it; perfect for full album listening.        │
├──────────────────────┼──────────────────────────────────────────────────────┤
│ 4. File-System Tree  │ Direct physical folder navigation for raw archives   │
│    (Archive Mode)    │ without requiring pre-indexed metadata.              │
└──────────────────────┴──────────────────────────────────────────────────────┘
```

---

### 2.3 Now-Playing Experience & Ambient Engine

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                      AMBIENT COLOR EXTRACTION & MESH SHADER                 │
│                                                                             │
│  [Album Art Image] ──► [Fast Color Quantization Engine]                      │
│                               │                                             │
│          ┌────────────────────┼────────────────────┐                        │
│          ▼                    ▼                    ▼                        │
│   [Dominant Tone]      [Vibrant Accent]     [Muted Contrast]                │
│          │                    │                    │                        │
│          ▼                    ▼                    ▼                        │
│  [GPU Shader Mesh]    [UI Highlights/EQ]   [WCAG 4.5:1 Text]                │
│  (Atmospheric Blur)   (Knobs, Progress)    (Legible Typography)             │
└─────────────────────────────────────────────────────────────────────────────┘
```

- **Footer Player**: Sleek persistent bar with high-res cover art, interactive waveform scrubber, time/duration counters, volume knob, audio format badge (`FLAC 96kHz 24bit`), and quick-toggle icons (Lyrics, Queue, Visualizer, DSP).
- **Theater / Canvas Mode (`F11`)**: Full-bleed immersive display with animated cover art, kinetic lyrics, and procedural background shaders.
- **Mini Player Mode (`Ctrl+M`)**: Compact floating widget with always-on-top capability.

---

### 2.4 Three-Stage Queue Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        SONORA DUAL-STAGE QUEUE MODEL                        │
│                                                                             │
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │ 📌 PRIORITY QUEUE ("Play Next" & "Play Later" - 3 tracks)             │  │
│  │   1. Track A - Artist (Drag to reorder / ✕ Remove)                     │  │
│  │   2. Track B - Artist                                                 │  │
│  ├───────────────────────────────────────────────────────────────────────┤  │
│  │ 🌊 ACTIVE CONTEXT STREAM (Album: The Dark Side of the Moon - 8 tracks)│  │
│  │   ▶ 3. Time - Pink Floyd (Currently Playing)                          │  │
│  │     4. The Great Gig in the Sky                                       │  │
│  │     5. Money                                                          │  │
│  ├───────────────────────────────────────────────────────────────────────┤  │
│  │ 📜 RECENT PLAYBACK HISTORY (Last 50 tracks)                           │  │
│  │   • Breathe (In the Air) - Played 4 mins ago                          │  │
│  │   • Speak to Me - Played 7 mins ago                                   │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────────┘
```

- **Drag-and-Drop Reordering**: Smooth inertial dragging to reorganize upcoming tracks.
- **Save Queue as Playlist**: 1-click action to export the current play queue into a persistent playlist.

---

### 2.5 State-of-the-Art Lyrics System

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         KINETIC LYRICS RENDERER                             │
│                                                                             │
│        [Past Line - 40% Opacity, Muted Blur]                                │
│        "Ticking away the moments that make up a dull day"                   │
│                                                                             │
│        ▶ [ACTIVE LINE - 100% Opacity, Dynamic Scale, Syllable Highlight]    │
│        "Fritter and  [  w  a  s  t  e  ]  the hours in an offhand way"      │
│                     └─── Glowing Fill ───┘                                  │
│                                                                             │
│        [Upcoming Line - 60% Opacity, Smooth Fade]                           │
│        "Kicking around on a piece of ground in your hometown"               │
└─────────────────────────────────────────────────────────────────────────────┘
```

- **Multi-Granularity Engine**:
  - *Syllable/Word Karaoke*: Smooth glowing text fill with spring-physics auto-scrolling.
  - *Line-by-Line LRC*: Crisp auto-scrolling line highlight.
  - *Unsynced Text*: Clean scrollable typography.
- **Multi-Lingual Annotations**: Furigana over Japanese Kanji, Romaji/Pinyin transcription pills, and translated subtitles.
- **Integrated Sync Editor**: Interactive timeline editor allowing users to tap and adjust line offsets (`+/- 50ms`) and submit improvements to the community LRCLIB database.

---

### 2.6 Visualizer & DSP Studio

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           DSP & VISUALIZER STUDIO                           │
│                                                                             │
│  ┌─────────────────────────────────┬─────────────────────────────────────┐  │
│  │  10-Band Parametric EQ Graph    │  projectM / Milkdrop 3 Canvas       │  │
│  │  [ Interactive Spline Curve ]   │  [ GPU Shaders & Algorithmic Presets]│  │
│  │                                 │                                     │  │
│  ├─────────────────────────────────┼─────────────────────────────────────┤  │
│  │  AutoEQ Profile Importer        │  CAVA Spectrum FFT Audio Bars       │  │
│  │  [ Sennheiser HD650 Target ▾ ]  │  |||||||||||||||||||||||||||||||||| │  │
│  └─────────────────────────────────┴─────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────────┘
```

- **Interactive Parametric EQ**: Drag filter handles (Gain, Q-factor, Frequency) with live visual response curves and 0dB headroom pre-cut indicators.
- **AutoEQ Search & Match**: Search and load over 5,000 headphone correction profiles in one click.
- **Dual Visualizer Engine**: Embedded projectM Milkdrop GLSL canvas + reactive CAVA-style logarithmic FFT audio bars.

---

### 2.7 Customization, Layout Studio & Marketplace UX

- **Layout Studio**: Interactive workspace editor where users can drag, dock, split, or float panels (Lyrics, Spectrum, Queue, Discography, Artwork) and save custom layout configurations.
- **Design Token System**: Complete real-time control over colors, radii, blur levels, fonts, and dynamic saturation.
- **Extension Marketplace**: In-app catalog to browse, preview, and 1-click install community themes, visualizer shaders, lyrics providers, and DSP plugins.

---

## 3. Three Candidate Desktop Compositions

### Composition A: The "Modern Curator" (Adaptive Canvas & Modular Rail)

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  ☰ Sonora  ◀ ▶   Library > Albums                   🔍 Search  [Theme] ⚙  │
├─────────┬──────────────────────────────────────────┬────────────────────────┤
│ 🏠 Home │  ALBUMS (248)           [Grid ▾] [Sort ▾]│ 🎵 NOW PLAYING         │
│ 📚 Lib  │  ┌──────────┐ ┌──────────┐ ┌──────────┐  │  ┌──────────────────┐  │
│ 📂 Fold │  │ [Art 1]  │ │ [Art 2]  │ │ [Art 3]  │  │  │                  │  │
│ 📜 Play │  │ Dark Side│ │ Abbey Rd │ │ Random   │  │  │    Cover Art     │  │
│ 🎛️ DSP  │  │ P. Floyd │ │ Beatles  │ │ Daft P.  │  │  │                  │  │
│ 🧩 Ext  │  └──────────┘ └──────────┘ └──────────┘  │  └──────────────────┘  │
│         │  ┌──────────┐ ┌──────────┐ ┌──────────┐  │  Time - Pink Floyd     │
│         │  │ [Art 4]  │ │ [Art 5]  │ │ [Art 6]  │  │  ──────────────────    │
│         │  │ Discovery│ │ In Rainbow││ Rumours  │  │  [Syllable Lyrics Box] │
│         │  │ Daft P.  │ │ Radiohead│ │ Fleetwood│  │  "Ticking away..."     │
├─────────┴──────────────────────────────────────────┴────────────────────────┤
│ ▶  ⏮  ⏭  02:45 ━━━━━━━●━━━━━━━━━━━━━ 06:53  🔊 ━━●━━  FLAC 96k/24b  [Lyrics]│
└─────────────────────────────────────────────────────────────────────────────┘
```
- **Target Persona**: Casual to dedicated music lovers who appreciate clean aesthetics, high-res cover art, and easy playlist curation.
- **Pros**: Balanced density; intuitive 3-zone layout; low cognitive load; gorgeous responsive scaling.
- **Cons**: Shows fewer tracks simultaneously on small screens compared to pure tabular views.

---

### Composition B: The "Audiophile Studio" (High-Density Multi-Pane Dock)

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ File  Edit  View  Playback  DSP  Tools  Help         Bit-Perfect: WASAPI Ex │
├──────────────┬───────────────────────────────────────┬──────────────────────┤
│ LIBRARY TREE │ TRACK PLAYLIST (Columns UI Style)     │ DSP & SPECTRUM       │
│ ▾ Pink Floyd │ #  Title        Artist   Time  Format │ ┌──────────────────┐ │
│   ▸ Meddle   │ 1  Speak to Me  P. Floyd 01:08 FLAC24 │ │ Parametric EQ    │ │
│   ▾ Dark Side│ 2  Breathe      P. Floyd 02:43 FLAC24 │ └──────────────────┘ │
│     01 Speak │ 3 ▶Time         P. Floyd 06:53 FLAC24 │ ┌──────────────────┐ │
│     02 Breath│ 4  The Great Gig P.Floyd 04:44 FLAC24 │ │ CAVA Audio Bars  │ │
│     03 Time  │ 5  Money        P. Floyd 06:22 FLAC24 │ │ |||||||||||||||| │ │
│     04 Great │ 6  Us and Them  P. Floyd 07:49 FLAC24 │ └──────────────────┘ │
├──────────────┴───────────────────────────────────────┴──────────────────────┤
│ 44.1kHz ──► Passthrough ──► WASAPI Exclusive (DAC Direct) ──► ReplayGain OFF │
│ ▶  ⏮  ⏭  [Track Waveform Scrubber]  Vol: 100% (Hardware Master Attenuation)  │
└─────────────────────────────────────────────────────────────────────────────┘
```
- **Target Persona**: Power users, audio engineers, foobar2000 veterans, and large-collection archivists.
- **Pros**: Maximum information density; instant access to technical stream metrics, DSP graphs, and multi-column sorting; zero wasted space.
- **Cons**: Higher visual complexity; steeper initial learning curve for casual listeners.

---

### Composition C: The "Atmospheric Theater" (Artwork & Lyrics Immersive Canvas)

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  Sonora Theater                                                    [✕] [🗖] │
│                                                                             │
│          ┌──────────────────────┐   "Ticking away the moments               │
│          │                      │    that make up a dull day                │
│          │                      │                                           │
│          │   High-Res Album     │  ▶ FRITTER AND WASTE                      │
│          │   Gatefold Artwork   │    THE HOURS IN AN OFFHAND WAY"           │
│          │                      │                                           │
│          │                      │   "Kicking around on a piece of           │
│          │                      │    ground in your hometown"               │
│          └──────────────────────┘                                           │
│          Time — Pink Floyd                                                  │
│          The Dark Side of the Moon (1973)                                   │
│                                                                             │
│  [ Ambient Dynamic GPU Fluid Mesh Glowing in Background Matching Cover Art ]│
│                                                                             │
│  ⏮  ▶  ⏭  02:45 ━━━━━━━━━━━━━━━━━━●━━━━━━━━━━━━ 06:53  🔊 ━━●━━  [Exit F11] │
└─────────────────────────────────────────────────────────────────────────────┘
```
- **Target Persona**: Focused listeners, evening listening sessions, full-screen living room / HTPC displays.
- **Pros**: Pure emotional immersion; zero UI distraction; stunning typography and fluid color mesh shaders.
- **Cons**: Not designed for active library management or deep metadata editing.

---

## 4. Explicit Classification

### Research Evidence
- User testing and community discussions across Plexamp, Apple Music, and Spicetify confirm that visual immersion (ambient color extraction and kinetic lyrics) produces the highest emotional satisfaction during playback.
- Power users in the foobar2000 and MusicBee communities insist on high-density customizable track tables with instant search and metadata editing.

### Sonora-Specific Recommendations
- Provide **Compositions A, B, and C as 1-click built-in Workspace Presets**, allowing users to effortlessly switch between "Curator Grid", "Audiophile Studio", and "Atmospheric Theater" modes via hotkey (`F10` / `F11`).
- Ensure every panel in these compositions is a modular dockable widget within Sonora's Layout Engine.

### Unresolved Questions
- Should the DSP Studio allow users to route visualizers pre-EQ or post-EQ via a graphical node graph?
- What is the most intuitive drag gesture for queue reordering on touch-enabled laptops?
