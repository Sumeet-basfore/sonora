# Sonora Visual Language & Design Philosophy

**Document ID:** `docs/19-visual-language.md`  
**Author:** Sonora Design Directorate  
**Target Platform:** Desktop Native (Linux / macOS / Windows)  
**Status:** Authoritative Design Specification  

---

## 1. The Core Philosophy

### 1.1 The Core Visual Principle
> **"Music is the visual subject. The UI is the instrument around it."**

In most software, the interface is a container for functional tools. In Sonora, the music itself—its album artwork, acoustic metadata, typography, poetic lyrics, and audio provenance—is the primary visual artwork. The application chrome is a high-precision, tactile physical instrument: sculpted, quiet, deliberate, and effortlessly responsive.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                            SONORA VISUAL SPECTRUM                           │
├───────────────────────────────┬─────────────────────────────────────────────┤
│ WHAT SONORA IS                │ WHAT SONORA IS NOT                          │
├───────────────────────────────┼─────────────────────────────────────────────┤
│ • Musical & Editorial         │ ✕ A SaaS dashboard / database manager       │
│ • Tactile & Physical          │ ✕ An administrative spreadsheet             │
│ • Premium & Atmospheric       │ ✕ A generic Spotify / Electron clone        │
│ • Technically Serious         │ ✕ A cluttered wall of neon gradients        │
│ • Compositionally Separated   │ ✕ A chaotic stack of nested cards & boxes   │
│ • Restrained in Motion        │ ✕ A restless circus of bouncing particles   │
└───────────────────────────────┴─────────────────────────────────────────────┘
```

---

## 2. Typography System

Sonora replaces arbitrary font scales with an **editorial 7-tier typographic hierarchy**. Typographic contrast is achieved through deliberate weight modulation, letter-spacing (tracking), and optical opacity rather than excessive font sizes.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          SONORA TYPOGRAPHIC SCALE                           │
├───────────────┬────────────┬──────────┬───────────┬─────────────────────────┤
│ Tier          │ Size (px)  │ Weight   │ Tracking  │ Primary Use             │
├───────────────┼────────────┼──────────┼───────────┼─────────────────────────┤
│ 1. Display    │ 32px / 2.0rem│ 700 / 800│ -0.035em  │ Album Title, Hero Hero  │
│ 2. Headline   │ 22px / 1.4rem│ 600 / 700│ -0.025em  │ View Headers, Modals    │
│ 3. Title      │ 15px / 0.95rem│ 600     │ -0.015em  │ Track Title, Section    │
│ 4. Body       │ 13px / 0.82rem│ 400 / 500│ +0.000em  │ Artist, Album, UI Labels│
│ 5. Caption    │ 11px / 0.70rem│ 500 / 600│ +0.030em  │ Timers, Badges, Context │
│ 6. Micro      │ 9px  / 0.58rem│ 700     │ +0.080em  │ Format Pills, Eyebrows  │
│ 7. Technical  │ 12px / 0.75rem│ 500     │ +0.020em  │ Monospace Audio Engine  │
└───────────────┴────────────┴──────────┴───────────┴─────────────────────────┘
```

### 2.1 Display & Album Typography
- **Font Stack:** Inter, -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif.
- **Rule:** Display headings feature tight negative tracking (`-0.035em`) to evoke prestige vinyl sleeve typography and magazine editorial layouts.

### 2.2 Track & Artist Typography
- **Track Title:** `15px`, Weight 600 (Semibold), Color `--text-primary` (`#f8fafc`).
- **Artist Name:** `13px`, Weight 500 (Medium), Color `--text-secondary` (`#cbd5e1`).
- **Metadata Separation:** Separated by optical center dots (`·`, opacity 0.4), avoiding comma soup.

### 2.3 Technical & Provenance Typography
- **Font Stack:** JetBrains Mono, SF Mono, Menlo, Consolas, monospace.
- **Rule:** Format, sample rates, bit depths, and ReplayGain values (`FLAC · 24-bit · 96.0 kHz · -4.5 dB`) always render in monospace with `font-variant-numeric: tabular-nums`.

### 2.4 Lyric Typography
- **Synchronized Active Line:** `24px` (Desktop) / `32px` (Theater), Weight 700, pure `#ffffff` with subtle ambient text-glow (`0 0 20px var(--accent-glow)`).
- **Inactive Lines:** `18px`, Weight 500, dimmed to 42% opacity (`#94a3b8`), scaling down optically to preserve focus on the active vocal phrase.

---

## 3. Spacing & Spatial Metrics

Sonora adheres strictly to a **4px modular baseline grid**. Every dimension, margin, padding, and gap is a multiple of 4px, creating harmonious optical rhythm.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           SPACING METRIC SYSTEM                             │
├───────────────┬────────────┬────────────────────────────────────────────────┤
│ Token         │ Size (px)  │ Architectural Context                          │
├───────────────┼────────────┼────────────────────────────────────────────────┤
│ `--space-1`   │ 4px        │ Micro gaps, badge internal padding             │
│ `--space-2`   │ 8px        │ Compact control padding, tracklist item gaps   │
│ `--space-3`   │ 12px       │ Standard item separation, icon-to-label gaps   │
│ `--space-4`   │ 16px       │ Container internal padding, toolbar gutters    │
│ `--space-6`   │ 24px       │ Section gutters, album grid gap                │
│ `--space-8`   │ 32px       │ Hero margins, modal interior gutters           │
│ `--space-12`  │ 48px       │ Major section dividers, stage breathing room   │
│ `--space-16`  │ 64px       │ Full-bleed theater margins, hero clearance     │
└───────────────┴────────────┴────────────────────────────────────────────────┘
```

### Optical Spacing Principles:
1. **No Dead Padding:** Avoid nested wrappers each adding 16px padding, which shrinks content into a tiny central box.
2. **Asymmetric Optical Balance:** Hero album artwork uses larger bottom clearance (32px) than top clearance (24px) to anchor visual weight downward toward the tracklist.
3. **Touch & Click Targets:** All interactive controls maintain a minimum interactive hit area of `32px × 32px` even when the visual icon glyph is 16px.

---

## 4. Surfaces & Compositional Separation

### 4.1 Eliminating "Card Bloat"
A major design defect in modern SaaS is wrapping every piece of text inside a bordered rectangle card. Sonora strictly forbids unnecessary containers.

```
  ✕ WRONG (SaaS Container Bloat):
  [App Background] ──► [Outer Card] ──► [Inner Card] ──► [Track Row Card] ──► Text

  ✓ CORRECT (Sonora Compositional Separation):
  [Atmospheric Canvas]
      │
      ├── Visual Whitespace & Typography Hierarchy
      └── Subtle Tonal Elevation (Only where interaction or grouping requires it)
```

### 4.2 The 5 Surface Tiers

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                             SURFACE LAYER MODEL                             │
├───────────────┬───────────────────────────────┬─────────────────────────────┤
│ Surface Tier  │ Color / Material Treatment    │ Application Context         │
├───────────────┼───────────────────────────────┼─────────────────────────────┤
│ 1. Canvas     │ `--bg-base` (`#090a0f`)       │ The infinite deep backdrop  │
│ 2. Recessed   │ `--bg-surface` (`#12151e`)    │ Sidebar, tracklist trough   │
│ 3. Elevated   │ `--bg-elevated` (`#1a1e2b`)   │ Playback bar, header, stage │
│ 4. Interactive│ `--bg-card` / Accent Pill     │ Buttons, active tabs, cards │
│ 5. Overlay    │ Glassmorphism (20px blur)     │ Drawers, flyouts, modals    │
└───────────────┴───────────────────────────────┴─────────────────────────────┘
```

---

## 5. Color Language & Ambient Artwork Lighting

### 5.1 The Neutral Foundation
- **Base Obsidian:** `#090a0f` (Deep near-black foundation with subtle blue-violet undertone)
- **Surface Dark:** `#12151e` (Quiet structural framing)
- **Elevated Slate:** `#1a1e2b` (Floating controls and playback dock)
- **Border Subtlety:** `rgba(255, 255, 255, 0.06)` (Barely perceptible structure)

### 5.2 Text Contrast & Luminosity
- **Text Primary (`#f8fafc`):** 100% opacity for track titles, active lyrics, and primary headers.
- **Text Secondary (`#cbd5e1`):** 80% opacity for artists, album titles, and active navigation.
- **Text Muted (`#94a3b8`):** 55% opacity for durations, technical data, and inactive lyric lines.
- **Text Faint (`#64748b`):** 35% opacity for disc numbers, table headers, and inactive hints.

### 5.3 Ambient Artwork Lighting (The Restrained Glow)
Album art generates an ambient color aura without turning the UI into an unreadable neon display:
- **Extraction:** Dual-color quantization selects 1 vibrant accent and 1 deep harmonic tone.
- **Backdrop Mask:** Applied as a low-opacity radial mesh (`opacity: 0.18` max) behind the stage.
- **Text Protection:** All text surfaces maintain a minimum WCAG contrast ratio of **7.0:1 (AAA)** against ambient backdrops by rendering above an opaque or blurred backdrop layer.

---

## 6. Artwork Hierarchy & Scaling

Artwork is Sonora's visual focal point. Rather than fixed arbitrary pixel squares, artwork obeys defined scale relationships:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                            ARTWORK SCALE MATRIX                             │
├─────────────────────┬──────────────┬──────────────┬─────────────────────────┤
│ Context             │ Dimensions   │ Corner Radius│ Shadow & Depth          │
├─────────────────────┼──────────────┼──────────────┼─────────────────────────┤
│ 1. Fullscreen Stage │ 360px–480px  │ 16px         │ 0 32px 80px rgba(0,0,0) │
│ 2. Spotlight Hero   │ 220px–260px  │ 12px         │ 0 16px 40px rgba(0,0,0) │
│ 3. Album Grid Card  │ 160px–200px  │ 8px          │ 0 8px 24px rgba(0,0,0)  │
│ 4. Track Row Thumb  │ 40px × 40px  │ 4px          │ 0 2px 6px rgba(0,0,0)   │
│ 5. Transport Mini   │ 48px × 48px  │ 6px          │ 0 4px 12px rgba(0,0,0)  │
└─────────────────────┴──────────────┴──────────────┴─────────────────────────┘
```

---

## 7. Control Language & Ergonomics

Controls are designed with the tactile precision of high-end hi-fi audio equipment:

```
       [Shuffle]   [Prev]    [ ▶ PLAY ]    [Next]   [Repeat]
          🔀         ⏮         ████         ⏭        🔁
                              ██████
```

1. **Play/Pause Center of Gravity:** Play/Pause is the dominant control, rendered as an elevated circular pill (`44px` diameter) with high-contrast accent fill and subtle depth shadow.
2. **Secondary Transport:** Previous, Next, Shuffle, and Repeat are sized at `28px × 28px` with subdued resting opacity (60%) that elevates to 100% on hover.
3. **Seek Waveform & Scrubber:** Features an interactive scrubber with a smooth rounded playhead indicator that expands from `0px` to `12px` on scrubber hover, displaying time scrub tooltips.
4. **Volume Fader:** Precision horizontal slider with instant tactile mute toggle by clicking the volume horn glyph.

---

## 8. Navigation & Chrome Simplification

The sidebar is reduced to essential primary navigation routes:

- **Primary Section:**
  - `Spotlight` (Curated listening & active listening hero)
  - `Albums` (Grid view with high-res cover art)
  - `Tracks` (Tabular view for power searching)
  - `Playlists` (User and smart curated playlists)
- **Secondary Section:**
  - `Marketplace` (Themes & Plugin extensions)
  - `Audio Settings` (DAC device & DSP controls)

On smaller screens ($\le 1280\text{px}$), the sidebar automatically collapses into a sleek **68px Icon Rail** with centered glyphs and tooltips.

---

## 9. Restrained Motion & Kinetic Physics

Motion in Sonora conveys physical mass and continuity, never distracting decoration:
- **Duration Scale:** Micro-interactions execute in `120ms - 180ms` (Fast). Page crossfades in `250ms` (Normal). Ambient lighting transitions in `800ms` (Slow).
- **Easing Curve:** Standard cubic bezier `cubic-bezier(0.16, 1, 0.3, 1)` (Fluid Apple-like spring deceleration).
- **Accessibility:** Reduced motion automatically disables pulse effects, ambient mesh shifts, and kinetic lyrics scaling.
