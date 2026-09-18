import test from 'node:test';
import assert from 'node:assert';

// Mock browser globals for Node.js test environment
const storageStore = new Map<string, string>();
(globalThis as any).localStorage = {
  getItem: (key: string) => storageStore.get(key) ?? null,
  setItem: (key: string, val: string) => storageStore.set(key, String(val)),
  removeItem: (key: string) => storageStore.delete(key),
  clear: () => storageStore.clear(),
};

(globalThis as any).document = {
  documentElement: {
    setAttribute: () => {},
    style: {
      setProperty: () => {},
    },
  },
  querySelector: () => null,
  getElementById: () => null,
  createElement: () => ({}),
  head: {
    appendChild: () => {},
  },
};

import {
  validateThemeSchema,
  isThemeDefinition,
  THEME_SCHEMA_URI,
  type ThemeDefinition,
} from '../src/customization/theme/schema.ts';
import {
  BUILT_IN_THEMES,
  THEME_DEFAULT_DARK,
  THEME_OLED_OBSIDIAN,
  THEME_CYBERPUNK_NEON,
  THEME_NORD_FROST,
  THEME_NORDIC_DAY,
  THEME_SOLARIZED_LIGHT,
} from '../src/customization/theme/presets.ts';
import {
  hexToRgb,
  rgbToHex,
  hexToRgba,
  adjustBrightness,
  getContrastRatio,
  generateAccentVariants,
} from '../src/customization/theme/colorUtils.ts';
import { ThemeEngine } from '../src/customization/theme/engine.ts';
import {
  validateLayoutSchema,
  isLayoutDefinition,
  LAYOUT_SCHEMA_URI,
  type LayoutDefinition,
} from '../src/customization/layout/schema.ts';
import {
  BUILT_IN_LAYOUTS,
  LAYOUT_DEFAULT_STUDIO,
  LAYOUT_MINIMAL_PLAYER,
  LAYOUT_AUDIOPHILE_DECK,
  LAYOUT_LYRICS_STAGE,
} from '../src/customization/layout/presets.ts';
import { LayoutManager } from '../src/customization/layout/manager.ts';
import {
  ALBUM_ART_STYLES,
  ArtworkStyleManager,
} from '../src/customization/artwork/styles.ts';
import {
  DEFAULT_VISUALIZER_CONFIG,
  VisualizerConfigManager,
} from '../src/customization/visualizer/config.ts';

// ---------------------------------------------------------------------------
// 1. Theme Schema & Validation Tests
// ---------------------------------------------------------------------------

test('Theme Schema: All built-in themes pass strict schema validation', () => {
  for (const theme of BUILT_IN_THEMES) {
    const result = validateThemeSchema(theme);
    assert.strictEqual(result.valid, true, `Theme "${theme.name}" should be valid, errors: ${result.errors.join(', ')}`);
    assert.strictEqual(isThemeDefinition(theme), true);
    assert.strictEqual(theme.$schema, THEME_SCHEMA_URI);
  }
});

test('Theme Schema: Detects missing tokens or invalid mode', () => {
  const invalid1 = { id: 'test', name: 'Test', version: '1.0', author: 'Sonora', mode: 'invalid_mode', tokens: {} };
  const res1 = validateThemeSchema(invalid1);
  assert.strictEqual(res1.valid, false);
  assert.ok(res1.errors.some(e => e.includes('mode')));

  // Missing required design tokens
  const invalid2: any = { ...THEME_DEFAULT_DARK, tokens: { '--bg-base': '#000' } };
  const res2 = validateThemeSchema(invalid2);
  assert.strictEqual(res2.valid, false);
  assert.ok(res2.errors.length > 5);
});

test('Theme Serialization & Import/Export roundtrip', () => {
  storageStore.clear();
  const engine = new ThemeEngine();

  // Export default theme
  const exportedJson = engine.exportActiveThemeJson();
  assert.ok(typeof exportedJson === 'string');

  const parsed = JSON.parse(exportedJson);
  assert.strictEqual(parsed.id, THEME_DEFAULT_DARK.id);
  assert.strictEqual(parsed.name, THEME_DEFAULT_DARK.name);

  // Import custom theme
  const customTheme: ThemeDefinition = {
    $schema: THEME_SCHEMA_URI,
    id: 'theme-monokai-pro',
    name: 'Monokai Pro',
    version: '1.0.0',
    author: 'Test Community',
    mode: 'dark',
    tokens: {
      ...THEME_DEFAULT_DARK.tokens,
      '--bg-base': '#2d2a2e',
      '--bg-surface': '#221f22',
      '--accent-primary': '#ffd866',
    },
  };

  const importResult = engine.importThemeFromJson(JSON.stringify(customTheme));
  assert.strictEqual(importResult.success, true);
  assert.strictEqual(engine.getActiveTheme().id, 'theme-monokai-pro');

  // Verify persistence in localStorage
  assert.strictEqual(storageStore.get('sonora_theme_active_id'), 'theme-monokai-pro');
});

// ---------------------------------------------------------------------------
// 2. Color Utilities & Design Token Calculations
// ---------------------------------------------------------------------------

test('Color Utils: Hex and RGB conversions', () => {
  assert.deepStrictEqual(hexToRgb('#ffffff'), { r: 255, g: 255, b: 255 });
  assert.deepStrictEqual(hexToRgb('#000000'), { r: 0, g: 0, b: 0 });
  assert.deepStrictEqual(hexToRgb('#6366f1'), { r: 99, g: 102, b: 241 });
  assert.strictEqual(rgbToHex(99, 102, 241), '#6366f1');
  assert.strictEqual(hexToRgba('#6366f1', 0.25), 'rgba(99, 102, 241, 0.25)');
});

test('Color Utils: Brightness adjustments and WCAG contrast', () => {
  const darkened = adjustBrightness('#ffffff', -0.5);
  assert.strictEqual(darkened, '#808080');

  const lightened = adjustBrightness('#000000', 0.5);
  assert.strictEqual(lightened, '#808080');

  const contrast = getContrastRatio('#ffffff', '#000000');
  assert.ok(contrast > 20.0, 'White on black contrast should approach 21:1');

  const variants = generateAccentVariants('#6366f1');
  assert.strictEqual(variants['--accent-primary'], '#6366f1');
  assert.ok(variants['--accent-glow'].startsWith('rgba('));
  assert.ok(variants['--accent-subtle'].startsWith('rgba('));
  assert.ok(variants['--accent-primary-hover'].startsWith('#'));
});

// ---------------------------------------------------------------------------
// 3. Layout Schema & Regional Persistence Tests
// ---------------------------------------------------------------------------

test('Layout Schema: Built-in layouts pass validation', () => {
  for (const layout of BUILT_IN_LAYOUTS) {
    const res = validateLayoutSchema(layout);
    assert.strictEqual(res.valid, true, `Layout "${layout.name}" failed: ${res.errors.join(', ')}`);
    assert.strictEqual(isLayoutDefinition(layout), true);
  }
});

test('Layout Manager: Switch, custom layout creation, deletion and persistence', () => {
  storageStore.clear();
  const manager = new LayoutManager();

  assert.strictEqual(manager.getActiveLayout().id, LAYOUT_DEFAULT_STUDIO.id);

  // Switch to minimal player
  const switched = manager.setLayout(LAYOUT_MINIMAL_PLAYER.id);
  assert.strictEqual(switched, true);
  assert.strictEqual(manager.isRegionVisible('sidebar'), false);
  assert.strictEqual(manager.isRegionVisible('library'), true);

  // Toggle individual regions
  manager.setRegionVisible('lyrics', true);
  assert.strictEqual(manager.isRegionVisible('lyrics'), true);

  // Save current configuration as custom layout
  const custom = manager.saveCurrentAsCustom('My Minimal Lyrics Desk', 'Custom desk');
  assert.ok(custom.id.startsWith('layout-custom-'));
  assert.strictEqual(manager.getActiveLayout().id, custom.id);

  // Verify persistence
  const savedActive = storageStore.get('sonora_layout_active_id');
  assert.strictEqual(savedActive, custom.id);

  // Delete custom layout
  const deleted = manager.deleteCustomLayout(custom.id);
  assert.strictEqual(deleted, true);
  assert.strictEqual(manager.getActiveLayout().id, LAYOUT_DEFAULT_STUDIO.id);
});

// ---------------------------------------------------------------------------
// 4. Album Artwork Styles Tests
// ---------------------------------------------------------------------------

test('Album Art Styles: 5 required styles exist and can be selected', () => {
  storageStore.clear();
  const manager = new ArtworkStyleManager();

  const styleIds = manager.getAvailableStyles().map(s => s.id);
  assert.ok(styleIds.includes('classic'));
  assert.ok(styleIds.includes('minimal'));
  assert.ok(styleIds.includes('immersive'));
  assert.ok(styleIds.includes('blurred'));
  assert.ok(styleIds.includes('vinyl'));

  manager.setStyle('vinyl');
  assert.strictEqual(manager.getActiveStyle(), 'vinyl');
  assert.strictEqual(storageStore.get('sonora_artwork_style_active'), 'vinyl');

  manager.setStyle('blurred');
  assert.strictEqual(manager.getActiveStyle(), 'blurred');

  manager.resetToDefault();
  assert.strictEqual(manager.getActiveStyle(), 'classic');
});

// ---------------------------------------------------------------------------
// 5. Visualizer Configuration & Clamping Tests
// ---------------------------------------------------------------------------

test('Visualizer Config: Clamps values and persists configuration', () => {
  storageStore.clear();
  const manager = new VisualizerConfigManager();

  assert.strictEqual(manager.getConfig().style, 'bars');
  assert.strictEqual(manager.getConfig().fps, 60);

  // Update with out-of-range values to test clamping
  manager.updateConfig({
    sensitivity: 99.0, // should clamp to 3.0
    smoothing: 0.001,  // should clamp to 0.01
    height: 1000,      // should clamp to 320
    style: 'wave',
  });

  const cfg = manager.getConfig();
  assert.strictEqual(cfg.sensitivity, 3.0);
  assert.strictEqual(cfg.smoothing, 0.01);
  assert.strictEqual(cfg.height, 320);
  assert.strictEqual(cfg.style, 'wave');

  // Verify persistence in localStorage
  const storedJson = storageStore.get('sonora_visualizer_config');
  assert.ok(storedJson);
  const stored = JSON.parse(storedJson!);
  assert.strictEqual(stored.style, 'wave');

  // Reset to default
  manager.resetToDefault();
  assert.strictEqual(manager.getConfig().style, 'bars');
  assert.strictEqual(manager.getConfig().sensitivity, DEFAULT_VISUALIZER_CONFIG.sensitivity);
});

// ---------------------------------------------------------------------------
// 6. Marketplace Themes: registration, preview, persistence, safe removal
// ---------------------------------------------------------------------------

function marketplaceFixture(id: string, base: ThemeDefinition): ThemeDefinition {
  return {
    $schema: THEME_SCHEMA_URI,
    id,
    name: `Market ${id}`,
    version: '1.0.0',
    author: 'Marketplace',
    mode: 'dark',
    tokens: { ...base.tokens },
  };
}

test('Marketplace Themes: sync registers, validates, and never shadows built-ins', () => {
  storageStore.clear();
  const engine = new ThemeEngine();
  const good = marketplaceFixture('org.marketplace.neon', THEME_DEFAULT_DARK);
  const malformed = { id: 'org.marketplace.bad', name: 'Bad' };
  const shadow = marketplaceFixture(THEME_DEFAULT_DARK.id, THEME_DEFAULT_DARK);

  const rejected = engine.syncMarketplaceThemes([good, malformed, shadow]);
  assert.deepStrictEqual(rejected, ['org.marketplace.bad', THEME_DEFAULT_DARK.id]);

  const available = engine.getAvailableThemes();
  assert.ok(available.some((t) => t.id === 'org.marketplace.neon'));
  // Built-in entry with the same id is untouched (not replaced, not duplicated).
  assert.strictEqual(available.filter((t) => t.id === THEME_DEFAULT_DARK.id).length, 1);

  const sources = engine.getThemeSources();
  const neon = sources.find((s) => s.id === 'org.marketplace.neon')!;
  assert.strictEqual(neon.isMarketplace, true);
  assert.strictEqual(neon.isBuiltIn, false);
  assert.strictEqual(engine.isMarketplaceTheme('org.marketplace.neon'), true);
  assert.strictEqual(engine.isMarketplaceTheme(THEME_DEFAULT_DARK.id), false);
});

test('Marketplace Themes: preview does not persist, apply does', () => {
  storageStore.clear();
  const engine = new ThemeEngine();
  engine.syncMarketplaceThemes([marketplaceFixture('org.marketplace.neon', THEME_DEFAULT_DARK)]);

  assert.strictEqual(engine.beginPreview('org.marketplace.neon'), true);
  assert.strictEqual(engine.isPreviewing(), true);
  assert.strictEqual(engine.getActiveTheme().id, 'org.marketplace.neon');
  // Preview must not touch persisted selection.
  assert.notStrictEqual(storageStore.get('sonora_theme_active_id'), 'org.marketplace.neon');

  engine.endPreview(false);
  assert.strictEqual(engine.isPreviewing(), false);
  assert.strictEqual(engine.getActiveTheme().id, THEME_DEFAULT_DARK.id);

  assert.strictEqual(engine.beginPreview('org.marketplace.nope'), false);

  engine.beginPreview('org.marketplace.neon');
  engine.endPreview(true);
  assert.strictEqual(engine.getActiveTheme().id, 'org.marketplace.neon');
  assert.strictEqual(storageStore.get('sonora_theme_active_id'), 'org.marketplace.neon');
});

test('Marketplace Themes: removal falls back to built-in, reset restores default', () => {
  storageStore.clear();
  const engine = new ThemeEngine();
  engine.syncMarketplaceThemes([marketplaceFixture('org.marketplace.neon', THEME_DEFAULT_DARK)]);
  assert.strictEqual(engine.setTheme('org.marketplace.neon'), true);

  // Uninstall while active: next sync drops it and falls back safely.
  engine.syncMarketplaceThemes([]);
  assert.strictEqual(engine.getActiveTheme().id, THEME_DEFAULT_DARK.id);

  assert.strictEqual(engine.ensureActiveAvailable(), false);
  assert.strictEqual(engine.resetToBuiltIn(), true);
  assert.strictEqual(engine.getActiveTheme().id, THEME_DEFAULT_DARK.id);
});
