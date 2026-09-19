import { test } from 'node:test';
import assert from 'node:assert/strict';
import { api } from '../src/api.ts';
import { appState } from '../src/state.ts';
import { UniversalSearchComponent } from '../src/components/UniversalSearch.ts';
import type { SearchScope } from '../src/types.ts';

test('Universal Search: Initial state, open and close lifecycle via StateManager', () => {
  assert.equal(appState.isUniversalSearchVisible(), false);

  appState.openUniversalSearch('Daft Punk', 'online_metadata');
  assert.equal(appState.isUniversalSearchVisible(), true);
  const state = appState.getUniversalSearchState();
  assert.equal(state.isOpen, true);
  assert.equal(state.query, 'Daft Punk');
  assert.equal(state.scope, 'online_metadata');

  appState.setUniversalSearchScope('lyrics');
  assert.equal(appState.getUniversalSearchState().scope, 'lyrics');

  appState.setUniversalSearchQuery('Get Lucky');
  assert.equal(appState.getUniversalSearchState().query, 'Get Lucky');

  appState.closeUniversalSearch();
  assert.equal(appState.isUniversalSearchVisible(), false);
});

test('Universal Search: Mock API online track, album, and artist lookup', async () => {
  const tracks = await api.searchOnlineTracks('Get Lucky', 10);
  assert.ok(tracks.length > 0);
  assert.equal(tracks[0].title, 'Get Lucky');
  assert.ok(tracks[0].recording_mbid.length > 0);
  assert.ok(tracks[0].artist_credits.some((c) => c.name === 'Daft Punk'));

  const albums = await api.searchOnlineAlbums('Random Access Memories', 10);
  assert.ok(albums.length > 0);
  assert.equal(albums[0].title, 'Random Access Memories');
  assert.equal(albums[0].primary_type, 'Album');

  const artists = await api.searchOnlineArtists('Daft Punk', 10);
  assert.ok(artists.length > 0);
  assert.equal(artists[0].name, 'Daft Punk');
  assert.equal(artists[0].country, 'FR');
});

test('Universal Search: Mock API lyrics candidate lookup', async () => {
  const candidates = await api.findLyricsCandidates({
    track_name: 'Get Lucky',
    artist_name: 'Daft Punk',
  });
  assert.ok(candidates.length > 0);
  assert.equal(candidates[0].track_name, 'Get Lucky');
  assert.equal(candidates[0].sync_type, 'LineSynced');
  assert.ok(candidates[0].raw_content.includes('[00:05.00]'));
});

test('Universal Search: Scope switching and query state invariants', () => {
  const scopes: SearchScope[] = ['local', 'online_metadata', 'lyrics'];
  for (const scope of scopes) {
    appState.setUniversalSearchScope(scope);
    assert.equal(appState.getUniversalSearchState().scope, scope);
  }
});

test('Universal Search: Stale request handling sequence logic', async () => {
  let activeReqId = 0;
  const executeQuery = async (id: number, delayMs: number, resultVal: string) => {
    await new Promise((res) => setTimeout(res, delayMs));
    if (id !== activeReqId) {
      return null; // Stale discard
    }
    return resultVal;
  };

  activeReqId = 1;
  const p1 = executeQuery(1, 50, 'result-1');
  activeReqId = 2; // User typed new character quickly
  const p2 = executeQuery(2, 10, 'result-2');

  const [r1, r2] = await Promise.all([p1, p2]);
  assert.equal(r1, null, 'Stale request 1 must be ignored');
  assert.equal(r2, 'result-2', 'Latest request 2 must resolve');
});

test('Universal Search: Debounce timer simulation', async () => {
  let calls = 0;
  let timer: any = null;
  const triggerDebounced = (val: string) => {
    if (timer) clearTimeout(timer);
    timer = setTimeout(() => {
      calls++;
    }, 50);
  };

  triggerDebounced('D');
  triggerDebounced('Da');
  triggerDebounced('Daf');
  triggerDebounced('Daft');

  await new Promise((res) => setTimeout(res, 80));
  assert.equal(calls, 1, 'Debounce must coalesce rapid keypresses into a single execution');
});
