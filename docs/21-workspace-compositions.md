# Sonora Workspace Compositions Specification

**Document ID:** `docs/21-workspace-compositions.md`  
**Author:** Sonora Design Directorate  
**Target Platform:** Desktop Native (Linux / macOS / Windows)  
**Status:** Authoritative Design Specification  

---

## 1. Overview & Workspace Strategy

Sonora rejects the one-size-fits-all SaaS layout. Depending on listening context—browsing discographies, performing audiophile headphone EQ calibration, immersing in late-night synchronized lyrics, or working with a discreet mini companion—Sonora morphs into **four distinct workspace compositions**.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         SONORA WORKSPACE PARADIGMS                          │
├──────────────────────┬──────────────────────────────────────────────────────┤
│ 1. Modern Curator    │ Editorial, artwork-centric, album discovery & preview│
├──────────────────────┼──────────────────────────────────────────────────────┤
│ 2. Audiophile Studio │ High-density tabular layout, technical provenance    │
├──────────────────────┼──────────────────────────────────────────────────────┤
│ 3. Atmospheric Theater│ Full-bleed artwork canvas, ambient mesh, synced lyric│
├──────────────────────┼──────────────────────────────────────────────────────┤
│ 4. Minimal Mode      │ Zero UI chrome, single album card, pure focus        │
└──────────────────────┴──────────────────────────────────────────────────────┘
```

---

## 2. Workspace 1: Modern Curator (Default Workspace)

### 2.1 Intended Emotional Tone
*Cultivated, warm, prestige music salon; reminiscent of browsing a bespoke vinyl collection.*

### 2.2 ASCII Layout Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ [☰] ◀ ▶  Library > Albums              [ 🔍 Search Albums / Artists ] [⚙]  │
├──────────────┬───────────────────────────────────────────────┬──────────────┤
│ NAVIGATION   │ CURATOR SPOTLIGHT HERO                        │ NOW PLAYING  │
│              │ ┌───────────────┬───────────────────────────┐ │ ┌──────────┐ │
│ • Spotlight  │ │ [HERO ART]    │ Random Access Memories    │ │ │ [COVER]  │ │
│ • Albums     │ │ 220px × 220px │ Daft Punk (2013)          │ │ │ 260px    │ │
│ • Tracks     │ │ (Vinyl Glow)  │ 13 Tracks · 74 min · FLAC │ │ └──────────┘ │
│ • Playlists  │ │               │ [ ▶ Play ]  [ + Queue ]   │ │ Get Lucky  │
│              │ └───────────────┴───────────────────────────┘ │ Daft Punk    │
│              │                                               │ ──────────── │
│              │ RECENT & PINNED ALBUMS (GRID)                 │ UP NEXT:     │
│              │ ┌───────┐ ┌───────┐ ┌───────┐ ┌───────┐       │ Lose Yourself│
│              │ │ ART   │ │ ART   │ │ ART   │ │ ART   │       │ Giorgio      │
│              │ └───────┘ └───────┘ └───────┘ └───────┘       │ ──────────── │
│              │ Discovery   Aja     Kid A     Currents        │ [Lyrics] [Q] │
├──────────────┴───────────────────────────────────────────────┴──────────────┤
│ [Track Info] ─── ⏮ [ ▶ PLAY ] ⏭ ─── [02:34 ━━━━●─────── 04:12] ─── [ 🔊 🎚 ]  │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 2.3 Visual Hierarchy & Component Placement
- **Dominant:** Large curated spotlight hero card (220px art) with high-contrast gradient action buttons.
- **Secondary:** Responsive multi-column album grid with release badges and artist tags.
- **Contextual:** Right sidebar surfaces current track artwork, format badge, and next 2 upcoming tracks.
- **What Disappears:** Dense metadata spreadsheets, raw folder hierarchies, complex DSP equalizer graphs.

---

## 3. Workspace 2: Audiophile Studio

### 3.1 Intended Emotional Tone
*Professional, technical, analytical; like an audio mastering suite or broadcast console.*

### 3.2 ASCII Layout Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ [☰] ◀ ▶  Studio > Discography           [ Filter: FLAC / 24-bit ] [🎛️ DSP] [⚙]│
├──────────────┬──────────────────────────────────────────────────────────────┤
│ TREE NAV     │ HIGH-DENSITY AUDIOPHILE DATA TABLE                           │
│              │ #  TITLE            ARTIST     ALBUM      FORMAT  RATE/DEPTH │
│ • 44.1k PCM  │ 01 Give Life Back.. Daft Punk  RAM        FLAC    96k/24-bit │
│ • 96k Hi-Res │ 02 The Game of Love Daft Punk  RAM        FLAC    96k/24-bit │
│ • 192k Studio│ 03 Giorgio by Moran Daft Punk  RAM        FLAC    96k/24-bit │
│ • Bit-Perfect│ 04 Within           Daft Punk  RAM        FLAC    96k/24-bit │
│ • Exclusive  │ 05 Instant Crush    Daft Punk  RAM        FLAC    96k/24-bit │
│              ├──────────────────────────────────────────────────────────────┤
│              │ LIVE SPECTRUM ANALYZER & HARDWARE TELEMETRY DOCK             │
│              │ [||||||||||||||||||||||||||||||||||||||||||||||||||||||||||] │
│              │ ALSA hw:0,0 | Bit-Perfect | Sinc Resample: OFF | Limiter: -0.3│
├──────────────┴──────────────────────────────────────────────────────────────┤
│ [FLAC 24/96] ── ⏮ [ ▶ PLAY ] ⏭ ── [01:14 ━━━━●─────── 04:35] ── [Peak: -0.2]│
└─────────────────────────────────────────────────────────────────────────────┘
```

### 3.3 Visual Hierarchy & Component Placement
- **Dominant:** High-density tabular layout (32px row height) displaying exact codec, sample rate, bit depth, ReplayGain, and bit-perfect indicators.
- **Secondary:** Real-time 48-band FFT spectrum analyzer and hardware output telemetry dock.
- **What Disappears:** Decorative ambient background meshes, oversized hero artwork cards, vinyl disc animations.

---

## 4. Workspace 3: Atmospheric Theater

### 4.1 Intended Emotional Tone
*Immersive, cinematic, meditative; perfect for late-night dedicated listening with poetic lyrics.*

### 4.2 ASCII Layout Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                                                                        [✕]  │
│                                                                             │
│                     ┌───────────────────────────┐                           │
│                     │                           │                           │
│                     │      [ VINYL STAGE ]      │                           │
│                     │        360px × 360px      │                           │
│                     │     (Ambient Canvas Mesh) │                           │
│                     │                           │                           │
│                     └───────────────────────────┘                           │
│                                                                             │
│                                 Touch                                       │
│                        Daft Punk · Random Access                            │
│                                                                             │
│                 "Hold on, if love is the answer you're home"                │
│             ▶  "HOLD ON, IF LOVE IS THE ANSWER YOU'RE HOME"  ◀              │
│                       "Touch, sweet touch, you've given..."                 │
│                                                                             │
│              [03:22 ━━━━━━━━━━━━━━━━━━━━━━━━━●━━━━━━━━━━━━ 08:18]           │
│                           ⏮   [ ❚❚ PAUSE ]   ⏭                             │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 4.3 Visual Hierarchy & Component Placement
- **Dominant:** Giant center-stage artwork (360px) surrounded by dynamic dual-point ambient lighting mesh and large kinetic synchronized lyrics.
- **Secondary:** Floating minimal playback scrubber and transport controls at bottom.
- **What Disappears:** Sidebars, navigation headers, database search bars, table columns, window borders.

---

## 5. Workspace 4: Minimal Mode (Mini Companion)

### 5.1 Intended Emotional Tone
*Unobtrusive, focused, pure listening companion during work or study.*

### 5.2 ASCII Layout Architecture

```
┌───────────────────────────────────────────────┐
│ ┌───────────┐  Instant Crush                  │
│ │           │  Daft Punk feat. Julian Casabl. │
│ │ [ARTWORK] │  Random Access Memories         │
│ │ 80px×80px │  ────────────────────────────── │
│ │           │  01:45 ━━━━━━●──────────── 05:37│
│ └───────────┘  ⏮     [ ❚❚ ]     ⏭     🔊 [Q]  │
└───────────────────────────────────────────────┘
```

### 5.3 Visual Hierarchy & Component Placement
- **Dominant:** 80px artwork thumbnail with track title, artist, and compact scrub bar.
- **What Disappears:** Entire library, sidebars, extra panels, headers.

---

## 6. Workspace Switching Specifications

- **Keyboard Triggers:**
  - `F1`: Modern Curator Workspace
  - `F2`: Audiophile Studio Workspace
  - `F3`: Atmospheric Theater Workspace
  - `F4`: Minimal Mode
- **Transition Animation:** 220ms cross-dissolve with coordinated panel width morphing using CSS Grid animations.
