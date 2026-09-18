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

import { lyricsManager, DEFAULT_LYRICS_PREFERENCES } from '../src/lyricsManager.ts';
import type { LyricsPreferences } from '../src/types.ts';

test('Lyrics Manager: Default preferences and initial state', () => {
  lyricsManager.resetToDefaults();
  const prefs = lyricsManager.getPreferences();

  assert.strictEqual(prefs.mode, 'classic');
  assert.strictEqual(prefs.fontSize, 'md');
  assert.strictEqual(prefs.fontWeight, 'normal');
  assert.strictEqual(prefs.alignment, 'center');
  assert.strictEqual(prefs.lineSpacing, 'normal');
  assert.strictEqual(prefs.activeOpacity, 1.0);
  assert.strictEqual(prefs.inactiveOpacity, 0.45);
  assert.strictEqual(prefs.highlightStyle, 'glow');
  assert.strictEqual(prefs.backgroundMode, 'surface');
  assert.strictEqual(prefs.autoScroll, true);
  assert.strictEqual(prefs.manualOffsetMs, 0);
});

test('Lyrics Manager: Switching all 5 display modes', () => {
  lyricsManager.resetToDefaults();

  const modes: LyricsPreferences['mode'][] = ['classic', 'focused', 'compact', 'minimal', 'dual'];
  for (const mode of modes) {
    lyricsManager.setMode(mode);
    assert.strictEqual(lyricsManager.getPreferences().mode, mode);
  }
});

test('Lyrics Manager: Typography and styling customization', () => {
  lyricsManager.resetToDefaults();

  lyricsManager.setFontSize('xl');
  assert.strictEqual(lyricsManager.getPreferences().fontSize, 'xl');

  lyricsManager.setFontWeight('bold');
  assert.strictEqual(lyricsManager.getPreferences().fontWeight, 'bold');

  lyricsManager.setAlignment('left');
  assert.strictEqual(lyricsManager.getPreferences().alignment, 'left');

  lyricsManager.setLineSpacing('relaxed');
  assert.strictEqual(lyricsManager.getPreferences().lineSpacing, 'relaxed');

  lyricsManager.setHighlightStyle('accent');
  assert.strictEqual(lyricsManager.getPreferences().highlightStyle, 'accent');

  lyricsManager.setBackgroundMode('blurred');
  assert.strictEqual(lyricsManager.getPreferences().backgroundMode, 'blurred');

  lyricsManager.setAutoScroll(false);
  assert.strictEqual(lyricsManager.getPreferences().autoScroll, false);
});

test('Lyrics Manager: Manual timing offset adjustments and reset', () => {
  lyricsManager.resetToDefaults();

  assert.strictEqual(lyricsManager.getPreferences().manualOffsetMs, 0);

  lyricsManager.adjustManualOffset(100);
  assert.strictEqual(lyricsManager.getPreferences().manualOffsetMs, 100);

  lyricsManager.adjustManualOffset(100);
  assert.strictEqual(lyricsManager.getPreferences().manualOffsetMs, 200);

  lyricsManager.adjustManualOffset(-300);
  assert.strictEqual(lyricsManager.getPreferences().manualOffsetMs, -100);

  lyricsManager.resetManualOffset();
  assert.strictEqual(lyricsManager.getPreferences().manualOffsetMs, 0);
});

test('Lyrics Manager: Subscriber notification on update', () => {
  lyricsManager.resetToDefaults();

  let notified = false;
  let receivedMode = '';

  const unsubscribe = lyricsManager.subscribe((prefs) => {
    notified = true;
    receivedMode = prefs.mode;
  });

  lyricsManager.setMode('focused');
  assert.strictEqual(notified, true);
  assert.strictEqual(receivedMode, 'focused');

  unsubscribe();
  notified = false;
  lyricsManager.setMode('minimal');
  assert.strictEqual(notified, false);
  assert.strictEqual(lyricsManager.getPreferences().mode, 'minimal');
});

test('Lyrics Manager: LocalStorage persistence roundtrip', () => {
  lyricsManager.resetToDefaults();
  lyricsManager.updatePreferences({
    mode: 'dual',
    fontSize: 'lg',
    manualOffsetMs: 250,
  });

  const raw = localStorage.getItem('sonora_lyrics_preferences_v1');
  assert.ok(raw !== null);
  const parsed = JSON.parse(raw!);
  assert.strictEqual(parsed.mode, 'dual');
  assert.strictEqual(parsed.fontSize, 'lg');
  assert.strictEqual(parsed.manualOffsetMs, 250);
});
