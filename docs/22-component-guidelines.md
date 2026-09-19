# Sonora Component Guidelines & Anti-Pattern Catalog

**Document ID:** `docs/22-component-guidelines.md`  
**Author:** Sonora Design Directorate  
**Target Platform:** Desktop Native (Linux / macOS / Windows)  
**Status:** Authoritative Design Specification  

---

## 1. Core Component Guidelines

### 1.1 Header & Global Search Bar
- **Purpose:** Primary spatial orientation, breadcrumbs, search entry, and workspace switcher.
- **Hierarchy:** Sits at the top of the content viewport (`--header-height: 56px`), elevated above scrollable views.
- **Anatomy:**
  - `History Controls`: Back/Forward buttons (`Alt+Left` / `Alt+Right`).
  - `Breadcrumb Trail`: Dynamic path (`Library > Albums > Random Access Memories`).
  - `Search Bar`: Rounded search field (`Ctrl+K` trigger) with clear button.
  - `Workspace Pills`: Segmented selector switching between Curator, Studio, and Theater.
- **States:** Default, Hover, Focused (2px accent outline), Active.
- **When to Use:** Top of all standard desktop workspaces.
- **When NOT to Use:** Never render in full-screen Atmospheric Theater or Minimal floating mode.

---

### 1.2 Sidebar & Collapsible Icon Rail
- **Purpose:** Fast access to core views (Spotlight, Albums, Tracks, Playlists, Marketplace, Settings).
- **Hierarchy:** Sits on the left edge (`240px` expanded, `68px` collapsed rail).
- **Anatomy:**
  - `Brand Area`: Sonora logo glyph + version tag.
  - `Navigation List`: Icon + Label + Count Badge.
  - `Bottom Tools`: Audio Device & Settings trigger.
- **States:** Default, Hover (`rgba(255,255,255,0.05)`), Active (solid left border + accent tint).
- **When to Use:** Standard browsing on screens $\ge 1280\text{px}$.
- **When NOT to Use:** Theater mode.

---

### 1.3 Album Grid & Curator Card
- **Purpose:** Visual browsing of music collection.
- **Hierarchy:** Central grid arranged in responsive columns (3 to 6 columns).
- **Anatomy:**
  - `Artwork Container`: 1:1 square aspect ratio with 8px corner radius and hover elevation shadow.
  - `Quick Play Trigger`: Floating play icon appearing on card hover.
  - `Title Label`: 14px Semibold, single-line truncation.
  - `Artist & Year`: 12px Medium, secondary text color.
- **States:** Default, Hover (image scales 1.03x + shadow intensifies), Playing (active wave badge).

---

### 1.4 Track Table & Audiophile Data Row
- **Purpose:** Power search, sorting, multi-track inspection, and batch queueing.
- **Hierarchy:** High-density list (36px standard / 30px compact row height).
- **Anatomy:**
  - `# Index`: Monospace track number / playing speaker icon.
  - `Title`: Primary bold text.
  - `Artist & Album`: Secondary text.
  - `Format / Provenance Badge`: Monospace pill (e.g. `FLAC 24/96`).
  - `Duration`: Tabular time format (`04:15`).
- **States:** Default, Hover (zebra glow + action trigger), Selected (accent background), Playing.

---

### 1.5 Now-Playing Stage & Signature Screen
- **Purpose:** Primary emotional center answering *What am I hearing?* instantly.
- **Hierarchy:** Right dedicated panel (300px–360px) or center-stage in Theater mode.
- **Anatomy:**
  - `Large Cover Art`: 260px–360px card with ambient glow.
  - `Track Identity`: Large bold Title, Artist link, Album subtitle.
  - `Audio Engine Badge`: Contextual badge showing active format, sample rate, bit depth, and mode.
  - `Upcoming Stream Preview`: Next 2 queued tracks with 1-click play.
- **States:** Playing, Paused, Transitioning (cross-dissolve).

---

### 1.6 Playback Transport Bar
- **Purpose:** Persistent playback status and hardware control.
- **Hierarchy:** Sits at the bottom (`84px` height), sticky across all views.
- **Anatomy:**
  - `Left Group`: Mini album artwork (48px), track title, artist, favorite heart icon.
  - `Center Group`: Previous, Play/Pause (44px hero pill), Next, Shuffle, Repeat, Seek Scrubber.
  - `Right Group`: Volume fader, Mute toggle, Lyrics toggle, Queue toggle, Visualizer toggle.
- **States:** Idle, Playing, Buffering (pulse scrubber), Muted.

---

### 1.7 Synchronized Lyrics Stage
- **Purpose:** Poetic, distraction-free lyric engagement.
- **Hierarchy:** Side drawer or full-bleed stage.
- **Anatomy:**
  - `Active Lyric Line`: 24px–32px, 700 weight, pure white with subtle ambient glow.
  - `Inactive Lyric Lines`: 18px, dimmed to 42% opacity.
  - `Offset Calibrator`: Micro buttons (`-0.1s`, `+0.1s`, `Reset`) to synchronize custom LRC files.
- **States:** Line Active, Line Hover (scrub to timestamp), Instrumental Break (pulsing wave).

---

### 1.8 Queue Drawer
- **Purpose:** Managing upcoming listening sequence.
- **Hierarchy:** Slide-out right overlay drawer (`360px`).
- **Anatomy:**
  - `Now Playing Item`: Sticky top card with live spectrum indicator.
  - `Priority Queue ("Play Next")`: Drag-to-reorder list with individual remove triggers.
  - `Context Stream ("Upcoming from Album")`: Upcoming natural album sequence.
  - `Header Actions`: Clear Queue, Shuffle, Close.

---

### 1.9 Settings & Customization Canvas
- **Purpose:** Deep hardware configuration, theme selection, layout creation, and plugin management.
- **Hierarchy:** Modal dialog with frosted glass overlay (`backdrop-filter: blur(20px)`).
- **Anatomy:**
  - `Sidebar Navigation`: Audio, Themes, Layouts, Lyrics, Plugins, Library.
  - `Audio Inspector`: Direct hardware sink selection (ALSA / WASAPI / CoreAudio), output mode, buffer size, ReplayGain settings.
  - `Live Preview`: Immediate token application without requiring app restart.

---

## 2. Visual Anti-Patterns Catalog

To guarantee Sonora never degenerates into a generic web dashboard, designers and engineers must strictly avoid these **10 Visual Anti-Patterns**:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                       SONORA VISUAL ANTI-PATTERNS                           │
├───────────────────────────────┬─────────────────────────────────────────────┤
│ Anti-Pattern                  │ Why It Is Forbidden in Sonora               │
├───────────────────────────────┼─────────────────────────────────────────────┤
│ 1. Nested Card Hell           │ Wrapping cards inside cards destroys visual │
│                               │ hierarchy and makes music feel like tickets.│
├───────────────────────────────┼─────────────────────────────────────────────┤
│ 2. Neon Wallpaper Overload    │ Full-screen saturated gradients make text   │
│                               │ unreadable and induce eye fatigue.          │
├───────────────────────────────┼─────────────────────────────────────────────┤
│ 3. Tiny Low-Contrast Metadata │ #555 gray text on #222 background fails WCAG│
│                               │ and frustrates users checking bit rates.    │
├───────────────────────────────┼─────────────────────────────────────────────┤
│ 4. Permanent Technical Clutter│ Displaying FFT graphs, sample counters, and │
│                               │ buffer stats permanently in casual browsing.│
├───────────────────────────────┼─────────────────────────────────────────────┤
│ 5. Bouncing / Jittery Motion  │ Over-animated bounce curves distract from   │
│                               │ album artwork and feel juvenile.            │
├───────────────────────────────┼─────────────────────────────────────────────┤
│ 6. Squished 1280px Sidebars   │ Keeping a 240px text sidebar on narrow      │
│                               │ laptop screens crushes track title columns. │
├───────────────────────────────┼─────────────────────────────────────────────┤
│ 7. Microscopic Album Artwork  │ Reducing artwork to 32px icons makes albums │
│                               │ unrecognizable; artwork must lead.          │
├───────────────────────────────┼─────────────────────────────────────────────┤
│ 8. Inconsistent Corner Radii  │ Mixing 2px, 8px, 24px, and 50px borders in  │
│                               │ adjacent controls creates visual dissonance.│
├───────────────────────────────┼─────────────────────────────────────────────┤
│ 9. Empty-State Desolation     │ Showing an empty black grid when 1-3 albums │
│                               │ exist instead of an intimate listening hero.│
├───────────────────────────────┼─────────────────────────────────────────────┤
│ 10. SaaS Pill-Button Sprawl   │ Turning every metadata tag into a clickable │
│                               │ tag cloud that distracts from music.        │
└───────────────────────────────┴─────────────────────────────────────────────┘
```
