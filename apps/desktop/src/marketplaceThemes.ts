import { api } from './api';
import { themeEngine } from './customization';
import { resolveThemeSelection } from './marketplace';
import type { ThemeDefinition } from './customization/theme/schema';

// DOM-side glue between the backend marketplace theme store and the theme
// engine. Never imported by Node tests (it touches the singleton engine).

export interface MarketplaceThemeSync {
  registered: number;
  rejected: string[];
  backendActive: string | null;
}

/**
 * Pull installed themes from the backend, register them with the theme
 * engine (which re-validates), fetch supplemental CSS for the active
 * marketplace theme, and reconcile the persisted backend selection.
 */
export async function syncMarketplaceThemesFromBackend(): Promise<MarketplaceThemeSync> {
  const [installed, backendActive] = await Promise.all([
    api.marketplaceThemes(),
    api.marketplaceActiveTheme(),
  ]);
  const defs: ThemeDefinition[] = installed.map((t) => ({
    $schema: 'https://sonora.audio/schemas/v1/theme.json',
    id: t.id,
    name: t.name,
    description: t.description ?? undefined,
    version: t.version,
    author: t.author,
    mode: (t.mode === 'light' ? 'light' : 'dark') as 'dark' | 'light',
    isBuiltIn: false,
    tokens: t.definition.tokens,
  }));
  const rejected = themeEngine.syncMarketplaceThemes(defs);

  const active = themeEngine.getActiveTheme();
  if (themeEngine.isMarketplaceTheme(active.id)) {
    try {
      themeEngine.setMarketplaceCss(await api.marketplaceThemeCss(active.id));
    } catch {
      themeEngine.setMarketplaceCss(null);
    }
  } else {
    themeEngine.setMarketplaceCss(null);
  }

  // Backend selection wins when it names an available theme; otherwise make
  // sure a dangling local selection falls back to a built-in.
  const availableIds = themeEngine.getAvailableThemes().map((t) => t.id);
  const fallbackId = themeEngine.getActiveTheme().id;
  const resolved = resolveThemeSelection(availableIds, backendActive, fallbackId);
  if (resolved !== fallbackId) {
    themeEngine.setTheme(resolved);
  } else {
    themeEngine.ensureActiveAvailable();
  }

  return { registered: defs.length - rejected.length, rejected, backendActive };
}

/** Apply a marketplace theme persistently (engine + backend + CSS). */
export async function applyMarketplaceTheme(id: string): Promise<boolean> {
  const def = await api.marketplaceThemeDefinition(id);
  const rejected = themeEngine.syncMarketplaceThemes([
    ...themeEngine.getMarketplaceThemeDefinitions(),
    {
      $schema: 'https://sonora.audio/schemas/v1/theme.json',
      id: def.id,
      name: def.name,
      description: def.description ?? undefined,
      version: def.version,
      author: def.author,
      mode: def.mode === 'light' ? 'light' : 'dark',
      isBuiltIn: false,
      tokens: def.tokens,
    },
  ]);
  if (rejected.length > 0) return false;
  try {
    themeEngine.setMarketplaceCss(await api.marketplaceThemeCss(id));
  } catch {
    themeEngine.setMarketplaceCss(null);
  }
  if (!themeEngine.setTheme(id)) return false;
  await api.marketplaceSetActiveTheme(id);
  return true;
}

/** Preview without persisting; resolves false when the theme is unavailable. */
export async function previewMarketplaceTheme(id: string): Promise<boolean> {
  const ok = themeEngine.beginPreview(id);
  if (!ok) return false;
  try {
    themeEngine.setMarketplaceCss(await api.marketplaceThemeCss(id));
  } catch {
    themeEngine.setMarketplaceCss(null);
  }
  return true;
}

export function cancelMarketplacePreview() {
  themeEngine.endPreview(false);
  const active = themeEngine.getActiveTheme();
  if (themeEngine.isMarketplaceTheme(active.id)) {
    void api
      .marketplaceThemeCss(active.id)
      .then((css) => themeEngine.setMarketplaceCss(css))
      .catch(() => themeEngine.setMarketplaceCss(null));
  } else {
    themeEngine.setMarketplaceCss(null);
  }
}

/** Remove an installed theme; resets to built-in when it was active. */
export async function removeMarketplaceTheme(id: string): Promise<void> {
  await api.marketplaceUninstall(id);
  await syncMarketplaceThemesFromBackend();
}

/** Reset to the built-in default theme (backend selection cleared too). */
export async function resetToBuiltInTheme(): Promise<void> {
  themeEngine.resetToBuiltIn();
  themeEngine.setMarketplaceCss(null);
  await api.marketplaceSetActiveTheme(null);
}
