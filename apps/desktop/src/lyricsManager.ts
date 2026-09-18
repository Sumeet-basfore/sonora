/**
 * Sonora Lyrics Manager.
 * Manages user lyrics preferences, display modes, font styling,
 * manual timing offsets, and persistence to LocalStorage.
 */

import type { LyricsPreferences } from './types.ts';

const STORAGE_KEY = 'sonora_lyrics_preferences_v1';

export const DEFAULT_LYRICS_PREFERENCES: LyricsPreferences = {
  mode: 'classic',
  fontSize: 'md',
  fontWeight: 'normal',
  alignment: 'center',
  lineSpacing: 'normal',
  activeOpacity: 1.0,
  inactiveOpacity: 0.45,
  highlightStyle: 'glow',
  backgroundMode: 'surface',
  autoScroll: true,
  manualOffsetMs: 0,
};

export type LyricsPreferencesListener = (prefs: LyricsPreferences) => void;

class LyricsManager {
  private prefs: LyricsPreferences;
  private listeners: Set<LyricsPreferencesListener> = new Set();

  constructor() {
    this.prefs = this.loadPreferences();
  }

  private loadPreferences(): LyricsPreferences {
    if (typeof localStorage === 'undefined') {
      return { ...DEFAULT_LYRICS_PREFERENCES };
    }
    try {
      const raw = localStorage.getItem(STORAGE_KEY);
      if (raw) {
        const parsed = JSON.parse(raw);
        return {
          ...DEFAULT_LYRICS_PREFERENCES,
          ...parsed,
        };
      }
    } catch (e) {
      console.warn('Failed to parse lyrics preferences from storage:', e);
    }
    return { ...DEFAULT_LYRICS_PREFERENCES };
  }

  private persist() {
    if (typeof localStorage === 'undefined') return;
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(this.prefs));
    } catch (e) {
      console.warn('Failed to save lyrics preferences to storage:', e);
    }
  }

  public getPreferences(): LyricsPreferences {
    return { ...this.prefs };
  }

  public updatePreferences(partial: Partial<LyricsPreferences>) {
    this.prefs = {
      ...this.prefs,
      ...partial,
    };
    this.persist();
    this.notify();
  }

  public setMode(mode: LyricsPreferences['mode']) {
    this.updatePreferences({ mode });
  }

  public setFontSize(fontSize: LyricsPreferences['fontSize']) {
    this.updatePreferences({ fontSize });
  }

  public setFontWeight(fontWeight: LyricsPreferences['fontWeight']) {
    this.updatePreferences({ fontWeight });
  }

  public setAlignment(alignment: LyricsPreferences['alignment']) {
    this.updatePreferences({ alignment });
  }

  public setLineSpacing(lineSpacing: LyricsPreferences['lineSpacing']) {
    this.updatePreferences({ lineSpacing });
  }

  public setHighlightStyle(highlightStyle: LyricsPreferences['highlightStyle']) {
    this.updatePreferences({ highlightStyle });
  }

  public setBackgroundMode(backgroundMode: LyricsPreferences['backgroundMode']) {
    this.updatePreferences({ backgroundMode });
  }

  public setAutoScroll(autoScroll: boolean) {
    this.updatePreferences({ autoScroll });
  }

  public adjustManualOffset(deltaMs: number) {
    this.updatePreferences({ manualOffsetMs: this.prefs.manualOffsetMs + deltaMs });
  }

  public resetManualOffset() {
    this.updatePreferences({ manualOffsetMs: 0 });
  }

  public setManualOffset(offsetMs: number) {
    this.updatePreferences({ manualOffsetMs: offsetMs });
  }

  public resetToDefaults() {
    this.prefs = { ...DEFAULT_LYRICS_PREFERENCES };
    this.persist();
    this.notify();
  }

  public subscribe(listener: LyricsPreferencesListener): () => void {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  }

  private notify() {
    for (const listener of this.listeners) {
      listener(this.getPreferences());
    }
  }
}

export const lyricsManager = new LyricsManager();
