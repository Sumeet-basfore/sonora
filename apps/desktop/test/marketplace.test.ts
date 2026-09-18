import test from 'node:test';
import assert from 'node:assert';

import {
  categoriesOf,
  compareSemver,
  describeCapabilities,
  filterCatalog,
  isNewerVersion,
  matchesQuery,
  orderUpdates,
  parseSemver,
  resolveThemeSelection,
  sortCatalog,
  timeAgoLabel,
} from '../src/marketplace.ts';
import type { CatalogEntry, UpdateInfo } from '../src/types.ts';

function entry(overrides: Partial<CatalogEntry> = {}): CatalogEntry {
  return {
    kind: 'plugin',
    id: 'org.sonora.lrclib',
    name: 'LRCLIB Lyrics Provider',
    description: 'Synced lyrics from LRCLIB.',
    author: 'Sonora Contributors',
    author_url: null,
    category: 'lyrics',
    homepage: 'https://lrclib.net',
    capabilities: ['lyrics:provider', 'network:fetch'],
    latest_version: '1.0.0',
    min_sonora_version: '0.1.0',
    installed_version: null,
    update_available: null,
    ...overrides,
  };
}

test('Marketplace: query matching is case-insensitive across fields', () => {
  const e = entry();
  assert.strictEqual(matchesQuery(e, ''), true);
  assert.strictEqual(matchesQuery(e, 'lrclib'), true);
  assert.strictEqual(matchesQuery(e, 'SYNCED'), true);
  assert.strictEqual(matchesQuery(e, 'contributors'), true);
  assert.strictEqual(matchesQuery(e, 'lyrics'), true);
  assert.strictEqual(matchesQuery(e, 'no-such-thing'), false);
});

test('Marketplace: filter by kind, category, and query', () => {
  const entries = [
    entry(),
    entry({ id: 'org.sonora.theme.night', kind: 'theme', name: 'Night', category: 'dark', capabilities: [] }),
    entry({ id: 'org.sonora.vis', name: 'Bars', category: 'visualizer' }),
  ];
  assert.strictEqual(filterCatalog(entries, { query: '', category: 'all', kind: 'plugin' }).length, 2);
  assert.strictEqual(filterCatalog(entries, { query: '', category: 'all', kind: 'theme' }).length, 1);
  assert.strictEqual(filterCatalog(entries, { query: '', category: 'dark', kind: 'all' }).length, 1);
  assert.strictEqual(filterCatalog(entries, { query: 'bars', category: 'all', kind: 'all' }).length, 1);
  assert.strictEqual(filterCatalog(entries, { query: 'bars', category: 'lyrics', kind: 'all' }).length, 0);
});

test('Marketplace: sort puts installed and updates first', () => {
  const entries = [
    entry({ id: 'b', name: 'Beta' }),
    entry({ id: 'a', name: 'Alpha', installed_version: '1.0.0' }),
    entry({ id: 'c', name: 'Gamma', installed_version: '1.0.0', update_available: '1.1.0' }),
  ];
  const sorted = sortCatalog(entries).map((e) => e.id);
  assert.deepStrictEqual(sorted, ['c', 'a', 'b']);
});

test('Marketplace: category listing is sorted and unique', () => {
  const entries = [entry({ category: 'visualizer' }), entry(), entry({ category: 'visualizer' })];
  assert.deepStrictEqual(categoriesOf(entries), ['lyrics', 'visualizer']);
});

test('Marketplace: semver parsing and comparison', () => {
  assert.deepStrictEqual(parseSemver('1.2.3'), [1, 2, 3]);
  assert.deepStrictEqual(parseSemver('10.0.1-rc.1'), [10, 0, 1]);
  assert.strictEqual(parseSemver('1.2'), null);
  assert.strictEqual(parseSemver('abc'), null);
  assert.strictEqual(compareSemver('1.0.0', '1.0.1'), -1);
  assert.strictEqual(compareSemver('1.10.0', '1.9.9'), 1);
  assert.strictEqual(compareSemver('2.0.0', '2.0.0'), 0);
  assert.strictEqual(compareSemver('nope', '1.0.0'), null);
  assert.strictEqual(isNewerVersion('1.1.0', '1.0.9'), true);
  assert.strictEqual(isNewerVersion('1.0.0', '1.0.0'), false);
  assert.strictEqual(isNewerVersion('0.9.0', '1.0.0'), false);
});

test('Marketplace: capability descriptions cover the closed set', () => {
  const e = entry();
  const lines = describeCapabilities(e);
  assert.strictEqual(lines.length, 2);
  assert.ok(lines.some((l) => l.includes('lyrics')));
  assert.ok(lines.some((l) => l.includes('allow-listed')));
  const theme = entry({ kind: 'theme', capabilities: [] });
  assert.deepStrictEqual(describeCapabilities(theme), ['No code runs: themes only change styling.']);
  const unknown = entry({ capabilities: ['filesystem:write'] });
  assert.ok(describeCapabilities(unknown)[0].includes('blocked'));
});

test('Marketplace: relative time labels', () => {
  const now = 1_000_000;
  assert.strictEqual(timeAgoLabel(now, now), 'just now');
  assert.strictEqual(timeAgoLabel(now - 120, now), '2m ago');
  assert.strictEqual(timeAgoLabel(now - 7200, now), '2h ago');
  assert.strictEqual(timeAgoLabel(now - 172800, now), '2d ago');
});

test('Marketplace: compatible updates sort first', () => {
  const updates: UpdateInfo[] = [
    { id: 'b', kind: 'plugin', name: 'Beta', current: '1.0.0', available: '2.0.0', changelog: 'x', compatible: false, min_sonora_version: '9.9.9' },
    { id: 'a', kind: 'theme', name: 'Alpha', current: '1.0.0', available: '1.1.0', changelog: 'y', compatible: true, min_sonora_version: '0.1.0' },
  ];
  assert.deepStrictEqual(orderUpdates(updates).map((u) => u.id), ['a', 'b']);
});

test('Marketplace: backend theme selection wins only when available', () => {
  const available = ['sonora-default-dark', 'org.marketplace.neon'];
  assert.strictEqual(
    resolveThemeSelection(available, 'org.marketplace.neon', 'sonora-default-dark'),
    'org.marketplace.neon'
  );
  // Dangling backend selection falls back to local, never to an unknown id.
  assert.strictEqual(
    resolveThemeSelection(available, 'org.marketplace.ghost', 'sonora-default-dark'),
    'sonora-default-dark'
  );
  assert.strictEqual(resolveThemeSelection(available, null, 'sonora-default-dark'), 'sonora-default-dark');
  assert.strictEqual(resolveThemeSelection([], 'org.marketplace.neon', 'sonora-default-dark'), 'sonora-default-dark');
});
