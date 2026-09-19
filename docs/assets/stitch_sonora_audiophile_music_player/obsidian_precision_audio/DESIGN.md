---
name: Obsidian Precision Audio
colors:
  surface: '#121318'
  surface-dim: '#121318'
  surface-bright: '#38393f'
  surface-container-lowest: '#0d0e13'
  surface-container-low: '#1a1b21'
  surface-container: '#1e1f25'
  surface-container-high: '#292a2f'
  surface-container-highest: '#34343a'
  on-surface: '#e3e1e9'
  on-surface-variant: '#c7c4d7'
  inverse-surface: '#e3e1e9'
  inverse-on-surface: '#2f3036'
  outline: '#908fa0'
  outline-variant: '#464554'
  surface-tint: '#c0c1ff'
  primary: '#c0c1ff'
  on-primary: '#1000a9'
  primary-container: '#8083ff'
  on-primary-container: '#0d0096'
  inverse-primary: '#494bd6'
  secondary: '#ffb95f'
  on-secondary: '#472a00'
  secondary-container: '#ee9800'
  on-secondary-container: '#5b3800'
  tertiary: '#4edea3'
  on-tertiary: '#003824'
  tertiary-container: '#00885d'
  on-tertiary-container: '#000703'
  error: '#ffb4ab'
  on-error: '#690005'
  error-container: '#93000a'
  on-error-container: '#ffdad6'
  primary-fixed: '#e1e0ff'
  primary-fixed-dim: '#c0c1ff'
  on-primary-fixed: '#07006c'
  on-primary-fixed-variant: '#2f2ebe'
  secondary-fixed: '#ffddb8'
  secondary-fixed-dim: '#ffb95f'
  on-secondary-fixed: '#2a1700'
  on-secondary-fixed-variant: '#653e00'
  tertiary-fixed: '#6ffbbe'
  tertiary-fixed-dim: '#4edea3'
  on-tertiary-fixed: '#002113'
  on-tertiary-fixed-variant: '#005236'
  background: '#121318'
  on-background: '#e3e1e9'
  surface-variant: '#34343a'
typography:
  headline-xl:
    fontFamily: Inter
    fontSize: 32px
    fontWeight: '800'
    lineHeight: 40px
    letterSpacing: -0.03em
  headline-lg:
    fontFamily: Inter
    fontSize: 24px
    fontWeight: '700'
    lineHeight: 32px
    letterSpacing: -0.02em
  headline-md:
    fontFamily: Inter
    fontSize: 18px
    fontWeight: '600'
    lineHeight: 24px
    letterSpacing: -0.01em
  headline-sm:
    fontFamily: Inter
    fontSize: 15px
    fontWeight: '600'
    lineHeight: 20px
    letterSpacing: 0em
  body-lg:
    fontFamily: Inter
    fontSize: 16px
    fontWeight: '500'
    lineHeight: 24px
    letterSpacing: -0.01em
  body-md:
    fontFamily: Inter
    fontSize: 14px
    fontWeight: '500'
    lineHeight: 20px
    letterSpacing: 0em
  body-sm:
    fontFamily: Inter
    fontSize: 12px
    fontWeight: '500'
    lineHeight: 16px
    letterSpacing: 0.01em
  label-lg:
    fontFamily: JetBrains Mono
    fontSize: 13px
    fontWeight: '600'
    lineHeight: 18px
    letterSpacing: 0.02em
  label-md:
    fontFamily: JetBrains Mono
    fontSize: 11px
    fontWeight: '500'
    lineHeight: 16px
    letterSpacing: 0.04em
  label-sm:
    fontFamily: JetBrains Mono
    fontSize: 10px
    fontWeight: '500'
    lineHeight: 12px
    letterSpacing: 0.06em
rounded:
  sm: 0.125rem
  DEFAULT: 0.25rem
  md: 0.375rem
  lg: 0.5rem
  xl: 0.75rem
  full: 9999px
spacing:
  gutter: 1rem
  gutter-compact: 0.5rem
  margin: 1.25rem
  margin-compact: 0.75rem
  space-xs: 0.25rem
  space-sm: 0.5rem
  space-md: 0.75rem
  space-lg: 1.25rem
  space-xl: 2rem
---

## Brand & Style

This design system targets critical audiophiles, mastering engineers, and technical sound purists who interface with lossless audio pipelines. The interface evokes the quiet authority of high-end studio rackmount hardware fused with modern computational precision. It creates an atmosphere of focused immersion, absolute signal integrity, and tactile feedback.

The aesthetic blends **Glassmorphic Precision** with **Tactile Instrument Architecture**:
- **Monolithic Substrates:** Deep obsidian backdrops anchored by structural dark slate frames, keeping glare minimal during late-night listening and monitoring sessions.
- **Translucent Acrylic Overlays:** Deeply blurred, frosted glass sheets layer information without obscuring background spectrum flows.
- **Studio Hardware Controls:** Tactile stepped attenuators, knurled dials, precise dual-axis sliders, and crisp vector telemetry emulate high-end analog laboratory equipment.
- **Instrument Telemetry:** High-density readouts, logarithmic frequency graphs, and status markers emphasize deterministic system state over decorative fluff.

## Colors

The palette operates under an uncompromising dark model engineered for extended low-light studio sessions, balancing structural depth with purposeful chromatic signaling.

### Base Canvases & Tiers
- **Void Canvas (`#090A0F`):** Root window background; deep obsidian black absorbing ambient light.
- **Structural Rack Frame (`#11131A`):** Opaque dark slate container defining sidebar boundaries, master rails, and transport panels.
- **Glass Panel Surface (`rgba(18, 20, 29, 0.75)`):** Dynamic analytical layers, modal popovers, and metering overlays paired with backdrop filtering.
- **Hairline Dividers (`rgba(255, 255, 255, 0.08)`):** Cold micro-borders isolating computational zones.
- **Active Structural Rim (`rgba(99, 102, 241, 0.5)`):** Highlights active signal paths and focused instrument modules.

### Functional Accents
- **Electric Indigo (`#6366F1`, Light Accent `#A5B4FC`, Glow `rgba(99, 102, 241, 0.35)`):** Primary brand accent; designates UI navigation, timeline scrub heads, cursor tracking, and active routing.
- **Audiophile Amber (`#F59E0B`, Light Accent `#FDE68A`, Glow `rgba(245, 158, 11, 0.3)`):** Signal telemetry, digital signal processing (DSP), equalizer parametric curves, and synchronized lyrics tracking.
- **Bit-Perfect Emerald (`#10B981`, Light Accent `#6EE7B7`, Glow `rgba(16, 185, 129, 0.35)`):** Lossless stream verification, DSD/MQA hardware pass-through validation, and nominal gain structures.
- **Signal Peak Rose (`#EF4444`, Glow `rgba(239, 68, 68, 0.4)`):** DAC clip detection, buffer underruns, signal phase errors, and destructive session operations.

### Typography Neutral Ramp
- **High Efficacy Text (`#F8FAFC`):** Primary data readouts, active track metadata, and key telemetry (meets WCAG AAA).
- **Secondary Readouts (`#94A3B8`):** Unfocused labels, standard track columns, and operational units.
- **Telemetry Ghost (`#64748B`):** Structural grid markers, scale indices, inactive control boundaries, and timestamps.

## Typography

The typographic hierarchy implements a dual-engine architecture:
1. **Structural Interface Core (Inter):** Leverages tightly tracked, medium-to-extrabold weights (500-800) for fast visual parsing of titles, navigation items, track rosters, and primary system controls.
2. **Instrument Telemetry & Code (JetBrains Mono):** Provides fixed-width precision for sample rates (e.g., `192.0 kHz`), quantization depths (`24-bit`), decibel calibrations (`-0.1 dBFS`), buffer latency, and timecode readouts. Numbers never jitter during active playback updates.

Use all-caps with generous tracking (`0.06em`) for `label-sm` when designating hardware status indicators, audio format badges (e.g., `FLAC`, `DSD256`), and spectrum analyzer frequency marks.

## Layout & Spacing

This design system targets fixed and fluid desktop screen real estate (Tauri runtime). It utilizes a dense modular dock layout designed around an absolute 4px baseline grid.

### Layout Model
- **Primary Host Window:** Fixed master-rail navigation (compact or full sidebar), dynamic workspace canvas (library table, logarithmic FFT spectrum, DSP rack), and an anchored bottom persistent transport deck.
- **Docking System:** Modules flex along structural horizontal rows with fixed-width parameter inspector panels (320px).
- **Spatial Densities:**
  - `space-xs` (4px): Parameter readout offsets, input micro-increments, tick offsets.
  - `space-sm` (8px): Control cluster internal padding, rotary dial groupings, badge internal padding.
  - `space-md` (12px): Inter-rack module spacing, standard list item row padding.
  - `space-lg` (20px): Structural panel padding, master channel separator spacing.
  - `space-xl` (32px): Major modal view padding, library section separations.

### Desktop Adaptability & Window Resizing
- **Expanded (Width > 1440px):** 3-column architecture (Source Navigation, Primary Lossless Stream Deck / Visualizer, Right Telemetry & DSP Matrix).
- **Compact Desktop (Width 1024px - 1439px):** Right telemetry panel folds into an overlay flyout; library columns drop secondary metadata (e.g., ReplayGain values).
- **Constrained Utility (Width < 1024px):** Single pane layout; sidebar collapses to an iconic slim dock (56px width), transport bar switches to compact knob mode.

## Elevation & Depth

Visual depth is achieved through optical filtration and physical rack layering rather than soft daylight shadows:

1. **Substrate Level 0 (Window Base):** Solid `#090A0F`. Non-reflective, housing background audio worker canvas renderers.
2. **Container Level 1 (Structural Bays):** Solid `#11131A` framed with a 1px inner stroke of `rgba(255, 255, 255, 0.05)`. Used for navigation trees and permanent playlists.
3. **Glassmorphic Level 2 (Floating Instrument Plates):** Fill `rgba(18, 20, 29, 0.75)` combined with `backdrop-filter: blur(24px)` and `border: 1px solid rgba(255, 255, 255, 0.08)`. Applied to audio routing trees, floating parameter modals, and active spectrum monitors.
4. **Focused Module Elevation:** When a DSP pod, fader channel, or EQ band is active, its boundary transitions to `rgba(99, 102, 241, 0.5)` with an electric indigo ambient back-glow: `box-shadow: 0 0 20px -4px rgba(99, 102, 241, 0.35)`.
5. **Hardware Skeuomorphic Recesses:** Track sliders and fader grooves utilize an inner shadow: `box-shadow: inset 0 2px 4px rgba(0, 0, 0, 0.8)` with a 1px bottom highlight edge (`rgba(255, 255, 255, 0.04)`) to simulate etched metal tracks.

## Shapes

The design system implements a tailored **Soft (1)** shape language to maintain a disciplined, high-grade instrument appearance. Excessive curvature is avoided to preserve maximum pixel area for dense tabular data and frequency visualizers.

- **Base Radius (`0.25rem` / 4px):** Standard interactive elements: telemetry tags, track row hover plates, buttons, toggles, text input boxes, and fader caps.
- **Medium Panels (`0.5rem` / 8px):** Glass floating decks, DSP modules, DSP graph containers, and context popovers.
- **Outer Shell (`0.75rem` / 12px):** Top-level Tauri window frame and master modal enclosures.
- **Special Geometry:** Dials, stepped attenuators, pan pots, and status LEDs remain absolute circles (`rounded-full`).

## Components

### Buttons & Transport Triggers
- **Primary Control (Play/Pause, Apply DSP):** Electric Indigo background (`#6366F1`), high-contrast text (`#F8FAFC`), 4px border radius. Hover introduces a subtle luminescence (`rgba(99, 102, 241, 0.35)` glow). Active state compresses by 1px scale.
- **Secondary Hardware Switch (Mute, Solo, Loop):** Dark Slate background (`#11131A`), 1px structural border (`rgba(255, 255, 255, 0.08)`). Inactive state text is `#94A3B8`. Active state shifts the border to Amber (`#F59E0B`) or Emerald (`#10B981`) with matching tinted typography.
- **Icon Utility Actions:** Transparent base with 1px hairline border hover reveals (`rgba(255, 255, 255, 0.1)`).

### Chips & Telemetry Badges
- **Format Indicators (e.g., `BIT-PERFECT`, `DSD 11.2MHz`, `FLAC 96/24`):** JetBrains Mono 10px uppercase (`label-sm`). 
- **Emerald Telemetry Tag:** Background `rgba(16, 185, 129, 0.1)`, text `#10B981`, border `rgba(16, 185, 129, 0.3)`.
- **Amber Processing Tag:** Background `rgba(245, 158, 11, 0.1)`, text `#F59E0B`, border `rgba(245, 158, 11, 0.3)`.

### Lists & Track Table
- **Container:** Structural Dark Slate with 0px inner dividers; alternating row striping is avoided.
- **Row Interaction:** Hover state activates a subtle surface wash (`rgba(255, 255, 255, 0.03)`) with an Electric Indigo 2px left border marker.
- **Active / Playing Row:** Background `rgba(99, 102, 241, 0.08)`, title typography jumps to `#F8FAFC` (weight 600), elapsed time displayed in JetBrains Mono (`#6366F1`).
- **Data Columns:** Numerical attributes (Bitrate, Sample Rate, Track #, Time) align right and strictly render in JetBrains Mono. Text fields (Title, Artist, Album) align left in Inter.

### Inputs & Field Controls
- **Precision Numeric Inputs:** Pitch black substrate (`#090A0F`), 1px perimeter line (`rgba(255, 255, 255, 0.08)`). Monospaced text with unit suffixes anchored in `#64748B`. Focused state applies `border: 1px solid rgba(99, 102, 241, 0.5)` and eliminates standard platform focus rings.
- **Search Fields:** Integrated keyboard shortcut badge (`Cmd+K`) rendered in `label-sm` with translucent inset background.

### Cards & Modular Rack Bays
- **Glassmorphic Instrument Card:** Translucent slate body (`rgba(18, 20, 29, 0.75)`), 24px backdrop blur, framed with hairline highlight borders (`rgba(255, 255, 255, 0.08)`).
- **Header Bay:** Houses module name in Inter 12px uppercase, module bypass rocker switch, and stereo mini-VU indicator.

### Domain-Specific Components: Audio Hardware Controls
- **Tactile Faders (Gain / Volume / EQ):** Recessed channel groove (4px wide, `#090A0F` background, inner shadow). Solid machined aluminum fader thumb (16px x 24px, 2px radius, subtle vertical grip knurling, `#F8FAFC` center indicator line).
- **Rotary Parameter Knobs:** Radial circular meter with an outer arc drawn via SVG stroke. Indigo or Amber gauge fill tracking value angle. Center dial contains physical knurling pattern and an LED dot pointer.
- **Spectrum Visualizer Box:** High-refresh canvas frame backed by a 12-line logarithmic grid colored with `rgba(255, 255, 255, 0.04)`. Dual renderers allow toggling between dynamic Amber bar graphs and Electric Indigo vector wireframes with active peak hold indicators.