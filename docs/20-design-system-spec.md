# Sonora Design System & Token Specification

**Document ID:** `docs/20-design-system-spec.md`  
**Author:** Sonora Design Directorate  
**Target Platform:** Desktop Native (Linux / macOS / Windows)  
**Status:** Authoritative Design Specification  

---

## 1. Design Token Architecture

The Sonora Design System is built on a strict token hierarchy:
1. **Primitive Tokens**: Raw hex values, scales, and timing values.
2. **Semantic Tokens**: Contextual mappings for surface backgrounds, text hierarchy, border luminance, and accents.
3. **Component Tokens**: Scoped values for buttons, table rows, badges, and scrubbers.

---

## 2. Complete Token Table (CSS Custom Properties)

```css
:root {
  /* --- 2.1 Color Primitives & Foundations --- */
  --color-obsidian-950: #06070a;
  --color-obsidian-900: #0c0d12;
  --color-obsidian-800: #14161f;
  --color-obsidian-700: #1c1f2b;
  --color-obsidian-600: #262a3a;

  --color-slate-100: #f8fafc;
  --color-slate-200: #e2e8f0;
  --color-slate-300: #cbd5e1;
  --color-slate-400: #94a3b8;
  --color-slate-500: #64748b;
  --color-slate-600: #475569;

  --color-indigo-500: #6366f1;
  --color-indigo-400: #818cf8;
  --color-purple-500: #a855f7;
  --color-amber-400: #fbbf24;
  --color-emerald-400: #34d399;
  --color-rose-500: #f43f5e;

  /* --- 2.2 Semantic Surface Tokens --- */
  --bg-base: var(--color-obsidian-900);
  --bg-surface: var(--color-obsidian-800);
  --bg-elevated: var(--color-obsidian-700);
  --bg-card: var(--color-obsidian-800);
  --bg-card-hover: rgba(255, 255, 255, 0.05);
  --bg-glass-overlay: rgba(12, 13, 18, 0.88);

  /* --- 2.3 Semantic Typography Tokens --- */
  --text-primary: var(--color-slate-100);
  --text-secondary: var(--color-slate-300);
  --text-muted: var(--color-slate-400);
  --text-faint: var(--color-slate-500);

  /* --- 2.4 Semantic Accent Tokens --- */
  --accent-primary: var(--color-indigo-500);
  --accent-hover: var(--color-indigo-400);
  --accent-subtle: rgba(99, 102, 241, 0.15);
  --accent-glow: rgba(99, 102, 241, 0.35);

  /* --- 2.5 Semantic Borders & Dividers --- */
  --border-subtle: rgba(255, 255, 255, 0.07);
  --border-medium: rgba(255, 255, 255, 0.12);
  --border-focus: var(--color-indigo-400);

  /* --- 2.6 Spatial Grid Tokens (4px Modular Scale) --- */
  --space-0-5: 2px;
  --space-1: 4px;
  --space-1-5: 6px;
  --space-2: 8px;
  --space-3: 12px;
  --space-4: 16px;
  --space-5: 20px;
  --space-6: 24px;
  --space-8: 32px;
  --space-10: 40px;
  --space-12: 48px;
  --space-16: 64px;

  /* --- 2.7 Typography Scales & Weights --- */
  --font-sans: "Inter", -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
  --font-mono: "JetBrains Mono", "SF Mono", Menlo, Consolas, monospace;

  --text-3xl: 32px;
  --text-2xl: 24px;
  --text-xl: 20px;
  --text-lg: 16px;
  --text-md: 14px;
  --text-sm: 13px;
  --text-xs: 11px;
  --text-2xs: 9px;

  --weight-regular: 400;
  --weight-medium: 500;
  --weight-semibold: 600;
  --weight-bold: 700;

  /* --- 2.8 Radii & Corner Curvature --- */
  --radius-xs: 4px;
  --radius-sm: 6px;
  --radius-md: 8px;
  --radius-lg: 12px;
  --radius-xl: 16px;
  --radius-2xl: 24px;
  --radius-full: 9999px;

  /* --- 2.9 Elevation & Shadow Depth --- */
  --shadow-sm: 0 1px 3px rgba(0, 0, 0, 0.3);
  --shadow-md: 0 4px 12px rgba(0, 0, 0, 0.45);
  --shadow-lg: 0 12px 32px rgba(0, 0, 0, 0.6);
  --shadow-xl: 0 24px 64px rgba(0, 0, 0, 0.75);
  --shadow-artwork: 0 16px 40px -8px rgba(0, 0, 0, 0.7), 0 0 24px var(--accent-glow);

  /* --- 2.10 Motion Durations & Springs --- */
  --duration-fast: 120ms;
  --duration-normal: 220ms;
  --duration-slow: 400ms;
  --ease-spring: cubic-bezier(0.16, 1, 0.3, 1);
  --ease-standard: cubic-bezier(0.4, 0, 0.2, 1);

  /* --- 2.11 Layer Z-Index Hierarchy --- */
  --z-base: 0;
  --z-sidebar: 10;
  --z-dock: 20;
  --z-header: 30;
  --z-drawer: 40;
  --z-overlay: 50;
  --z-modal: 100;
  --z-tooltip: 200;

  /* --- 2.12 Layout Architectural Dimensions --- */
  --sidebar-width: 240px;
  --sidebar-rail-width: 68px;
  --now-playing-width: 340px;
  --playback-bar-height: 84px;
  --header-height: 56px;
}
```

---

## 3. Responsive Breakpoint Specification

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                       RESPONSIVE ADAPTATION MATRIX                          │
├───────────────────┬──────────────┬──────────────┬───────────────────────────┤
│ Target Resolution │ Sidebar Mode │ Right Stage  │ Main Library Grid Layout  │
├───────────────────┼──────────────┼──────────────┼───────────────────────────┤
│ 1. 1280 × 800     │ 68px Icon Rail│ Hidden (Dock)│ 3 Columns (180px art)     │
│ 2. 1440 × 900     │ 240px Tree   │ 300px Drawer │ 4 Columns (190px art)     │
│ 3. 1920 × 1080    │ 240px Tree   │ 360px Stage  │ 5–6 Columns (210px art)   │
└───────────────────┴──────────────┴──────────────┴───────────────────────────┘
```

### 3.1 1280 × 800 (Compact Laptop)
- **Sidebar:** Collapses to 68px Icon Rail. Text labels hidden; icons centered with 12px vertical spacing; hover triggers instant tooltip.
- **Right Stage:** Fully hidden from permanent grid; Now Playing context is surfaced via the Bottom Transport Bar and slide-out drawer on click.
- **Library Viewport:** Expands to 100% remaining width (`1212px`), preventing horizontal squishing of tracklists.

### 3.2 1440 × 900 (Standard Widescreen)
- **Sidebar:** 240px with full label navigation and library counts.
- **Right Stage:** 300px containing hero artwork, active format badge, and upcoming next track preview.
- **Library Viewport:** Balanced central column (`900px`).

### 3.3 1920 × 1080 (High-Resolution Studio)
- **Sidebar:** 240px standard tree.
- **Right Stage:** 360px dedicated stage with large 280px cover art, live spectrum visualizer, and artist biography.
- **Library Viewport:** Expansive multi-column grid (`1320px`).

---

## 4. Accessibility & Contrast Standard

- **WCAG AA Compliance (Strict 4.5:1 Minimum):**
  - Text Primary (`#f8fafc`) on Base (`#0c0d12`): **18.2:1 (AAA Pass)**
  - Text Secondary (`#cbd5e1`) on Base (`#0c0d12`): **12.6:1 (AAA Pass)**
  - Text Muted (`#94a3b8`) on Base (`#0c0d12`): **6.8:1 (AA Pass)**
- **Focus Indicators:** Explicit 2px border offset (`--border-focus`) for all keyboard navigators (`Tab` / Arrow keys).
- **Reduced Motion Support:** All transitions gracefully fall back to 0ms opacity steps under `prefers-reduced-motion`.
