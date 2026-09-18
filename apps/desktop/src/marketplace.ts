import type { CatalogEntry, ExtensionKind, UpdateInfo } from './types';

// Pure marketplace helpers: filtering, search, version math. DOM-free so the
// Node test suite can exercise them without a browser.

export type MarketplaceTab = 'browse' | 'installed' | 'updates';

export interface BrowseFilter {
  query: string;
  category: string; // 'all' or a category id
  kind: ExtensionKind | 'all';
}

/** Case-insensitive substring search over id, name, description, author. */
export function matchesQuery(entry: CatalogEntry, query: string): boolean {
  const q = query.trim().toLowerCase();
  if (!q) return true;
  return [entry.id, entry.name, entry.description, entry.author, entry.category]
    .join('\n')
    .toLowerCase()
    .includes(q);
}

export function filterCatalog(entries: CatalogEntry[], filter: BrowseFilter): CatalogEntry[] {
  return entries.filter((e) => {
    if (filter.kind !== 'all' && e.kind !== filter.kind) return false;
    if (filter.category !== 'all' && e.category !== filter.category) return false;
    return matchesQuery(e, filter.query);
  });
}

export function sortCatalog(entries: CatalogEntry[]): CatalogEntry[] {
  return [...entries].sort((a, b) => {
    // Installed first, then updates available, then alphabetical.
    const aRank = (a.installed_version ? 0 : 2) + (a.update_available ? -1 : 0);
    const bRank = (b.installed_version ? 0 : 2) + (b.update_available ? -1 : 0);
    if (aRank !== bRank) return aRank - bRank;
    return a.name.localeCompare(b.name);
  });
}

export function categoriesOf(entries: CatalogEntry[]): string[] {
  const seen = new Set(entries.map((e) => e.category));
  return [...seen].sort();
}

/** Parse "major.minor.patch" (ignoring pre-release/build) into a tuple. */
export function parseSemver(version: string): [number, number, number] | null {
  const m = /^(\d+)\.(\d+)\.(\d+)(?:[-+].*)?$/.exec(version.trim());
  if (!m) return null;
  return [Number(m[1]), Number(m[2]), Number(m[3])];
}

/** -1 | 0 | 1, or null when either side is not semver. */
export function compareSemver(a: string, b: string): number | null {
  const pa = parseSemver(a);
  const pb = parseSemver(b);
  if (!pa || !pb) return null;
  for (let i = 0; i < 3; i++) {
    if (pa[i] !== pb[i]) return pa[i] < pb[i] ? -1 : 1;
  }
  return 0;
}

export function isNewerVersion(candidate: string, current: string): boolean {
  return compareSemver(candidate, current) === 1;
}

/** Human-readable capability summary for the consent screen. */
export function describeCapabilities(entry: CatalogEntry): string[] {
  if (entry.kind === 'theme') return ['No code runs: themes only change styling.'];
  return entry.capabilities.map((cap) => {
    switch (cap) {
      case 'lyrics:provider':
        return 'Provide lyrics for tracks you play.';
      case 'metadata:read':
        return 'Read track metadata to enrich lookups.';
      case 'library:read':
        return 'Read library statistics (no file access).';
      case 'visualizer:tap':
        return 'Read the audio visualizer feed (no audio capture).';
      case 'ui:widget':
        return 'Show a small UI widget in the app.';
      case 'storage:cache':
        return 'Keep a small private cache (1 MiB max).';
      case 'network:fetch':
        return 'Contact the network (allow-listed origins only).';
      default:
        return `Unknown capability (blocked): ${cap}`;
    }
  });
}

/** Relative "x ago" label for the cache timestamp (seconds epoch). */
export function timeAgoLabel(fetchedAtSecs: number, nowSecs?: number): string {
  const now = nowSecs ?? Math.floor(Date.now() / 1000);
  const delta = Math.max(0, now - fetchedAtSecs);
  if (delta < 60) return 'just now';
  if (delta < 3600) return `${Math.floor(delta / 60)}m ago`;
  if (delta < 86400) return `${Math.floor(delta / 3600)}h ago`;
  return `${Math.floor(delta / 86400)}d ago`;
}

/** Updates the UI may offer (compatible ones first). */
export function orderUpdates(updates: UpdateInfo[]): UpdateInfo[] {
  return [...updates].sort((a, b) => {
    if (a.compatible !== b.compatible) return a.compatible ? -1 : 1;
    return a.name.localeCompare(b.name);
  });
}

/**
 * Resolve which theme id should be active: the persisted backend selection
 * wins when it names an available theme, otherwise the local fallback.
 * Never returns an id outside `availableIds`.
 */
export function resolveThemeSelection(
  availableIds: string[],
  backendActiveId: string | null,
  fallbackId: string
): string {
  if (backendActiveId && availableIds.includes(backendActiveId)) return backendActiveId;
  if (availableIds.includes(fallbackId)) return fallbackId;
  return availableIds[0] ?? fallbackId;
}
