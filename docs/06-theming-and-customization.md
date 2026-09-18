# Sonora: Theming, Customization & Visualizer Architecture

## 1. Theming Architecture & Design Token Hierarchy

Sonora's theming system is built on a layered semantic design token architecture. Themes define structural design tokens that dynamically cascade into layout containers, typography, visualizers, and interactive components.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           DESIGN TOKEN CASCADE MODEL                        │
│                                                                             │
│   ┌─────────────────────────────────────────────────────────────────────┐   │
│   │ Base Theme Tokens (Static System Palettes: Dark, Light, OLED, Custom│   │
│   └──────────────────────────────────┬──────────────────────────────────┘   │
│                                      │                                       │
│                                      ▼                                       │
│   ┌─────────────────────────────────────────────────────────────────────┐   │
│   │ Dynamic Album-Art Tokens (Real-time Extracted Dominant/Vibrant Swatch│  │
│   └──────────────────────────────────┬──────────────────────────────────┘   │
│                                      │                                       │
│                                      ▼                                       │
│   ┌─────────────────────────────────────────────────────────────────────┐   │
│   │ WCAG AA Mathematical Clamping Layer (Ensures 4.5:1 Contrast Ratio)  │   │
│   └──────────────────────────────────┬──────────────────────────────────┘   │
│                                      │                                       │
│                                      ▼                                       │
│   ┌─────────────────────────────────────────────────────────────────────┐   │
│   │ Semantic Runtime Tokens (Applied to CSS / WGPU Render Pipeline)     │   │
│   │ • --bg-canvas, --accent-primary, --text-primary, --glow-color       │   │
│   └─────────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 1.1 Core Semantic Token Hierarchy

| Token Name | Default Dark | Description & Usage |
| :--- | :--- | :--- |
| `--bg-base` | `#0e0f12` | Root window viewport background |
| `--bg-surface` | `#16181d` | Surface background for panels, sidebars, and tables |
| `--bg-elevated` | `#21242b` | Modals, dropdown menus, and floating tooltips |
| `--bg-canvas` | `rgba(...)` | Dynamic ambient background glow color |
| `--accent-primary` | `#6366f1` | Primary interactive buttons, active tab, scrub head |
| `--accent-secondary` | `#a855f7` | Secondary accents, visualizer peak highlights |
| `--text-primary` | `#f8fafc` | High-emphasis headers, active track title, active lyrics |
| `--text-secondary` | `#94a3b8` | Artist names, album titles, durations, column headers |
| `--text-muted` | `#64748b` | Disabled states, inactive lyrics, track numbers |
| `--border-subtle` | `#2e3440` | Subtle panel separators and grid dividers |
| `--glow-intensity` | `0.6` | Normalized ambient shader glow intensity (0.0 - 1.0) |

---

## 2. Dynamic Palette Extraction & WCAG Contrast Engine

### 2.1 Extraction Pipeline
When a new track is loaded, the player extracts color swatches from the cover art image using a 3-stage quantization pipeline executed on a background worker:

```
[Album Cover Bitmap] ──► [Octree / K-Means Quantization] ──► [Swatch Candidate Clustering]
                                                                      │
┌─────────────────────────────────────────────────────────────────────┘
▼
[Dominant, Vibrant, DarkVibrant, LightVibrant, Muted] ──► [Oklab / HCT Color Space]
                                                                      │
┌─────────────────────────────────────────────────────────────────────┘
▼
[WCAG 2.1 AA Contrast Math Verification] ──► [Lightness/Chroma Clamping] ──► [Token Cascade]
```

### 2.2 Mathematical Contrast Enforcement Algorithm
To guarantee legibility, text tokens are calculated relative to their background in the perceptual Oklab color space:

1. **Calculate Relative Luminance ($L$)**:
   $$L = 0.2126 R_{lin} + 0.7152 G_{lin} + 0.0722 B_{lin}$$
2. **Calculate Contrast Ratio ($CR$)**:
   $$CR = \frac{L_{lighter} + 0.05}{L_{darker} + 0.05}$$
3. **Automated Clamping**: If $CR < 4.5$, the text token's lightness in Oklab space is iteratively shifted along the lightness axis until $CR \ge 4.5$ is satisfied.

---

## 3. Ambient GPU Shaders & Canvas Glow

### 3.1 Shader Architecture (WGSL / GLSL)
The desktop GUI renders an ambient, living background canvas using a multi-pass GPU fragment shader.

```
Pass 1: Cover Texture ──► Downsample (32x32) ──► Dual-Kawase Blur Pass (16px)
Pass 2: Blurred Buffer + Dynamic Palette ──► Noise Perturbation & Fluid Mesh Warp
Pass 3: Final Composition ──► Vignette Clamping ──► Alpha Blended UI Canvas
```

#### Ambient Backdrop Shader Spec (WGSL)
- **Uniforms**:
  - `u_resolution`: Viewport dimensions in physical pixels.
  - `u_time`: Monotonically increasing playback time clock.
  - `u_palette`: 4 extracted RGBA color vectors (Dominant, Vibrant, DarkVibrant, Muted).
  - `u_audio_energy`: Normalized low-frequency bass energy extracted from audio tap (0.0 to 1.0) driving subtle rhythmic breathing pulses.

---

## 4. Modular Dockable Panel Layouts & Layout Studio

### 4.1 Layout Specification Schema
Workspaces are defined as recursive JSON layout trees composed of container nodes (splitters, tabs) and leaf nodes (view panels):

```json
{
  "$schema": "https://sonora.audio/schemas/v1/layout.json",
  "name": "Audiophile Studio",
  "root": {
    "type": "split",
    "orientation": "horizontal",
    "splitRatio": 0.22,
    "first": {
      "type": "panel",
      "panelId": "library-navigator"
    },
    "second": {
      "type": "split",
      "orientation": "vertical",
      "splitRatio": 0.70,
      "first": {
        "type": "tabs",
        "activeTabIndex": 0,
        "tabs": [
          { "title": "Tracks", "panelId": "track-table" },
          { "title": "Album Grid", "panelId": "album-grid" },
          { "title": "Visualizer", "panelId": "projectm-canvas" }
        ]
      },
      "second": {
        "type": "split",
        "orientation": "horizontal",
        "splitRatio": 0.50,
        "first": { "type": "panel", "panelId": "lyrics-view" },
        "second": { "type": "panel", "panelId": "parametric-eq-inspector" }
      }
    }
  }
}
```

---

## 5. Visualizer Subsystem Architecture

Sonora includes two native visualizer engines driven by the shared audio tap:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          VISUALIZER ENGINE SUBSYSTEMS                       │
│                                                                             │
│                        ┌────────────────────────────┐                       │
│                        │    Shared Audio Tap Ring   │                       │
│                        │ (2048 PCM / 256 FFT Bins)  │                       │
│                        └─────────────┬──────────────┘                       │
│                                      │                                      │
│                ┌─────────────────────┴─────────────────────┐                │
│                ▼                                           ▼                │
│   ┌───────────────────────────┐               ┌───────────────────────────┐ │
│   │    CAVA FFT Engine Core   │               │   projectM (Milkdrop 3)   │ │
│   ├───────────────────────────┤               ├───────────────────────────┤ │
│   │ • Logarithmic Frequency   │               │ • OpenGL / Vulkan Context │ │
│   │   Binning (20Hz - 20kHz)  │               │ • Classic .milk Presets   │ │
│   │ • Gravity & Integral Fall │               │ • Dynamic Texture Bindings│ │
│   │ • Renders to GUI & TUI    │               │ • 60/120/144 FPS Rendering│ │
│   └───────────────────────────┘               └───────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 5.1 CAVA FFT Engine Mathematics
Frequency bands are partitioned logarithmically across user-configured bars (16 to 256):

$$f_{low}(k) = 20 \times \left(\frac{20000}{20}\right)^{\frac{k}{N}}, \quad f_{high}(k) = f_{low}(k+1)$$

1. **Gravity Smoothing**: Peak heights drop according to simulated physical gravity:
   $$h_{new} = \max(h_{target}, h_{old} - g \cdot \Delta t)$$
2. **Integral Smoothing**: Bar heights blend with historical frames via an exponential moving average.

### 5.2 projectM (Milkdrop 3) Integration
- Embedded `libprojectM` C++ runtime wrapped via safe FFI bindings.
- Renders to an offscreen GPU texture directly composited into the main desktop window.
- Supports instant preset cycling (space/backspace), rating stars, and custom preset folder directories.

---

## 6. Theme Package Format (`.sonora-theme`)

Sonora themes are distributed as standard compressed archives (`.sonora-theme` / `.zip`) or directories containing:

```
cyberpunk-neon.sonora-theme/
├── theme.json          # Metadata, author, semantic token definitions
├── styles.css          # Optional CSS variable overrides & typography
├── background.wgsl     # Optional custom ambient backdrop fragment shader
├── layouts/            # Included custom workspace layouts
│   └── default.json
└── preview.png         # 1200x800 preview screenshot for marketplace
```

---

## 7. Reversibility & Customization Boundaries

> [!IMPORTANT]
> ### Reversible Architecture Decisions
> - **Shader Language**: Abstracted via standard shader bindings, supporting both WGSL and SPIR-V/GLSL.
> - **Palette Color Engine**: Can swap between ColorThief, Vibrant, and Google Material You HCT without altering the token schema.
> - **Layout Engine Serialization**: Tree-based JSON format is decoupled from UI framework rendering components.
>
> ### Invariable Boundaries
> - **Token Schema Stability**: Core token names (`--bg-base`, `--accent-primary`, etc.) are immutable across minor version updates to guarantee theme backward compatibility.
