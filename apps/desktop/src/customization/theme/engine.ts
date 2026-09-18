/**
 * Sonora Theme Engine: Live Theme Switching, Custom Accent Injection,
 * Schema Validation, and Local Persistence.
 */

import { generateAccentVariants } from './colorUtils.ts';
import { BUILT_IN_THEMES, THEME_DEFAULT_DARK } from './presets.ts';
import { type ThemeDefinition, validateThemeSchema } from './schema.ts';

const STORAGE_KEY_ACTIVE_THEME = 'sonora_theme_active_id';
const STORAGE_KEY_CUSTOM_ACCENT = 'sonora_theme_custom_accent';
const STORAGE_KEY_USER_THEMES = 'sonora_theme_custom_definitions';

const MARKETPLACE_CSS_ELEMENT_ID = 'sonora-marketplace-theme-css';

export type ThemeChangeListener = (theme: ThemeDefinition) => void;

export interface ThemeSource {
  id: string;
  name: string;
  mode: 'dark' | 'light';
  isBuiltIn: boolean;
  isMarketplace: boolean;
}

export class ThemeEngine {
  private activeTheme: ThemeDefinition = THEME_DEFAULT_DARK;
  private customAccentHex: string | null = null;
  private customThemes: Map<string, ThemeDefinition> = new Map();
  private marketplaceThemes: Map<string, ThemeDefinition> = new Map();
  private marketplaceCss: string | null = null;
  private previewThemeId: string | null = null;
  private listeners: Set<ThemeChangeListener> = new Set();

  constructor() {
    this.loadFromStorage();
  }

  /**
   * Subscribe to live theme updates.
   */
  public subscribe(listener: ThemeChangeListener): () => void {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  }

  private notify() {
    for (const listener of this.listeners) {
      try {
        listener(this.activeTheme);
      } catch (e) {
        console.error('Error in theme listener:', e);
      }
    }
  }

  /**
   * Get list of all available themes (built-in + marketplace + user imported).
   * Built-ins always win id collisions; nothing can shadow or remove them.
   */
  public getAvailableThemes(): ThemeDefinition[] {
    const all = [...BUILT_IN_THEMES];
    for (const custom of this.marketplaceThemes.values()) {
      if (!all.some(t => t.id === custom.id)) {
        all.push(custom);
      }
    }
    for (const custom of this.customThemes.values()) {
      if (!all.some(t => t.id === custom.id)) {
        all.push(custom);
      }
    }
    return all;
  }

  /**
   * Available themes annotated with their source (for selector grouping).
   */
  public getThemeSources(): ThemeSource[] {
    return this.getAvailableThemes().map(t => ({
      id: t.id,
      name: t.name,
      mode: t.mode,
      isBuiltIn: t.isBuiltIn === true,
      isMarketplace: this.marketplaceThemes.has(t.id),
    }));
  }

  public isMarketplaceTheme(themeId: string): boolean {
    return this.marketplaceThemes.has(themeId);
  }

  public getMarketplaceThemeDefinitions(): ThemeDefinition[] {
    return [...this.marketplaceThemes.values()];
  }

  /**
   * Ensure the active theme still exists (e.g. after an uninstall); falls
   * back to the built-in default otherwise. Returns true when a fallback
   * happened.
   */
  public ensureActiveAvailable(): boolean {
    if (this.getAvailableThemes().some(t => t.id === this.activeTheme.id)) {
      return false;
    }
    this.activeTheme = THEME_DEFAULT_DARK;
    this.marketplaceCss = null;
    this.applyThemeToDom();
    this.saveToStorage();
    this.notify();
    return true;
  }

  public getActiveTheme(): ThemeDefinition {
    return this.activeTheme;
  }

  public getCustomAccent(): string | null {
    return this.customAccentHex;
  }

  /**
   * Switch the active theme by its unique ID.
   */
  public setTheme(themeId: string): boolean {
    const theme = this.getAvailableThemes().find(t => t.id === themeId);
    if (!theme) return false;

    this.activeTheme = theme;
    this.applyThemeToDom();
    this.saveToStorage();
    this.notify();
    return true;
  }

  /**
   * Register marketplace-installed themes (already validated by the backend
   * and re-validated here against the theme schema). Replaces the previous
   * marketplace set wholesale, so uninstalled themes disappear. If the
   * active theme was removed, falls back to the built-in default.
   * Returns the ids that were rejected.
   */
  public syncMarketplaceThemes(defs: unknown[]): string[] {
    const rejected: string[] = [];
    const next = new Map<string, ThemeDefinition>();
    for (const def of defs) {
      const validation = validateThemeSchema(def);
      if (!validation.valid) {
        const id = (def as Record<string, unknown>)?.id;
        rejected.push(typeof id === 'string' ? id : '<unknown>');
        continue;
      }
      const theme = def as ThemeDefinition;
      // Built-in ids are reserved: marketplace packages must never claim or
      // shadow them. Such entries are rejected, not registered.
      if (theme.isBuiltIn === true || BUILT_IN_THEMES.some(t => t.id === theme.id)) {
        rejected.push(theme.id);
        continue;
      }
      theme.isBuiltIn = false;
      next.set(theme.id, theme);
    }
    this.marketplaceThemes = next;
    if (!this.getAvailableThemes().some(t => t.id === this.activeTheme.id)) {
      this.activeTheme = THEME_DEFAULT_DARK;
      this.marketplaceCss = null;
      this.applyThemeToDom();
      this.saveToStorage();
      this.notify();
    }
    return rejected;
  }

  /**
   * Supplemental stylesheet for the active marketplace theme (extra raw CSS
   * shipped beside theme.json; validated by the backend, no remote fetches).
   */
  public setMarketplaceCss(css: string | null) {
    this.marketplaceCss = css;
    this.applyMarketplaceCss();
  }

  private applyMarketplaceCss() {
    if (typeof document === 'undefined') return;
    const existing = document.getElementById(MARKETPLACE_CSS_ELEMENT_ID);
    if (existing) existing.remove();
    if (!this.marketplaceCss || !this.isMarketplaceTheme(this.activeTheme.id)) return;
    const style = document.createElement('style');
    style.id = MARKETPLACE_CSS_ELEMENT_ID;
    style.textContent = this.marketplaceCss;
    document.head.appendChild(style);
  }

  /**
   * Preview a theme without persisting the selection. Call endPreview(false)
   * to revert or endPreview(true) to keep it (= apply).
   */
  public beginPreview(themeId: string): boolean {
    const theme = this.getAvailableThemes().find(t => t.id === themeId);
    if (!theme) return false;
    if (this.previewThemeId === null) {
      this.previewThemeId = this.activeTheme.id;
    }
    this.activeTheme = theme;
    this.applyThemeToDom();
    this.notify();
    return true;
  }

  public endPreview(commit: boolean) {
    if (this.previewThemeId === null) return;
    if (!commit) {
      const prev = this.getAvailableThemes().find(t => t.id === this.previewThemeId);
      this.activeTheme = prev ?? THEME_DEFAULT_DARK;
      this.applyThemeToDom();
      this.notify();
    } else {
      this.saveToStorage();
    }
    this.previewThemeId = null;
  }

  public isPreviewing(): boolean {
    return this.previewThemeId !== null;
  }

  /**
   * Reset to the built-in default theme (built-ins are always available).
   */
  public resetToBuiltIn(): boolean {
    return this.setTheme(THEME_DEFAULT_DARK.id);
  }

  /**
   * Set user-selected custom accent color (or null to revert to theme default).
   */
  public setCustomAccent(hex: string | null) {
    this.customAccentHex = hex;
    this.applyThemeToDom();
    this.saveToStorage();
    this.notify();
  }

  /**
   * Live apply active tokens directly to document root CSS variables.
   */
  public applyThemeToDom() {
    const root = document.documentElement;
    if (!root) return;

    // Set data attribute for container / mode styling
    root.setAttribute('data-theme-mode', this.activeTheme.mode);
    root.setAttribute('data-theme-id', this.activeTheme.id);

    // Apply base tokens from theme definition
    for (const [token, value] of Object.entries(this.activeTheme.tokens)) {
      if (value !== undefined) {
        root.style.setProperty(token, value);
      }
    }

    // If a custom accent color is set, override accent-related design tokens
    if (this.customAccentHex) {
      const variants = generateAccentVariants(this.customAccentHex);
      for (const [token, value] of Object.entries(variants)) {
        root.style.setProperty(token, value);
      }
    }

    // Supplemental marketplace stylesheet (only for marketplace themes).
    this.applyMarketplaceCss();
  }

  /**
   * Import and validate a theme definition from JSON.
   */
  public importThemeFromJson(jsonString: string): { success: boolean; theme?: ThemeDefinition; errors?: string[] } {
    try {
      const parsed = JSON.parse(jsonString);
      const validation = validateThemeSchema(parsed);
      if (!validation.valid) {
        return { success: false, errors: validation.errors };
      }

      const imported = parsed as ThemeDefinition;
      imported.isBuiltIn = false;

      this.customThemes.set(imported.id, imported);
      this.activeTheme = imported;
      this.applyThemeToDom();
      this.saveToStorage();
      this.notify();

      return { success: true, theme: imported };
    } catch (e) {
      return { success: false, errors: [`JSON parse failure: ${e instanceof Error ? e.message : String(e)}`] };
    }
  }

  /**
   * Export the current theme (including any custom accent) as a formatted JSON string.
   */
  public exportActiveThemeJson(): string {
    const exported: ThemeDefinition = {
      ...this.activeTheme,
      tokens: { ...this.activeTheme.tokens },
    };

    if (this.customAccentHex) {
      const variants = generateAccentVariants(this.customAccentHex);
      Object.assign(exported.tokens, variants);
    }

    return JSON.stringify(exported, null, 2);
  }

  /**
   * Reset all themes and accents to factory default.
   */
  public resetToDefault() {
    this.activeTheme = THEME_DEFAULT_DARK;
    this.customAccentHex = null;
    this.applyThemeToDom();
    this.saveToStorage();
    this.notify();
  }

  private saveToStorage() {
    try {
      if (typeof localStorage === 'undefined') return;
      localStorage.setItem(STORAGE_KEY_ACTIVE_THEME, this.activeTheme.id);
      if (this.customAccentHex) {
        localStorage.setItem(STORAGE_KEY_CUSTOM_ACCENT, this.customAccentHex);
      } else {
        localStorage.removeItem(STORAGE_KEY_CUSTOM_ACCENT);
      }

      const customArray = Array.from(this.customThemes.values());
      localStorage.setItem(STORAGE_KEY_USER_THEMES, JSON.stringify(customArray));
    } catch (e) {
      console.warn('Failed to save theme settings to storage:', e);
    }
  }

  private loadFromStorage() {
    try {
      if (typeof localStorage === 'undefined') return;

      // Load custom user themes
      const customStr = localStorage.getItem(STORAGE_KEY_USER_THEMES);
      if (customStr) {
        try {
          const parsed = JSON.parse(customStr);
          if (Array.isArray(parsed)) {
            for (const item of parsed) {
              if (validateThemeSchema(item).valid) {
                this.customThemes.set(item.id, item);
              }
            }
          }
        } catch {
          // ignore corrupted custom themes
        }
      }

      // Load custom accent
      this.customAccentHex = localStorage.getItem(STORAGE_KEY_CUSTOM_ACCENT);

      // Load active theme
      const savedId = localStorage.getItem(STORAGE_KEY_ACTIVE_THEME);
      if (savedId) {
        const found = this.getAvailableThemes().find(t => t.id === savedId);
        if (found) {
          this.activeTheme = found;
        }
      }
    } catch (e) {
      console.warn('Failed to load theme settings from storage:', e);
    }

    this.applyThemeToDom();
  }
}

export const themeEngine = new ThemeEngine();
