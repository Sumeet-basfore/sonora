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
(globalThis as any).window = {
  setInterval: () => 1,
  clearInterval: () => {},
  addEventListener: () => {},
  dispatchEvent: () => true,
};

import { appState } from '../src/state.ts';
import { api } from '../src/api.ts';
import type {
  RankedCandidateMatch,
  ArtworkCandidate,
  LyricsCandidate,
  ConfidenceTier,
} from '../src/types.ts';

test('Enrichment API: Mock metadata candidate lookup and score breakdown', async () => {
  const results = await api.findMetadataCandidates(1);
  assert.ok(Array.isArray(results));
  assert.strictEqual(results.length, 1);

  const match: RankedCandidateMatch = results[0];
  assert.strictEqual(match.candidate_track.title, 'Get Lucky (feat. Pharrell Williams & Nile Rodgers)');
  assert.strictEqual(match.score_breakdown.confidence_tier, 'high');
  assert.ok(match.score_breakdown.total_score >= 0.9);
  assert.strictEqual(match.candidate_release?.title, 'Random Access Memories');
  assert.strictEqual(match.candidate_track.recording_mbid, 'mbid-rec-001');
});

test('Enrichment API: Mock artwork discovery across providers', async () => {
  const artResults = await api.findArtworkCandidates({
    album_title: 'Random Access Memories',
    artist_name: 'Daft Punk',
  });

  assert.ok(Array.isArray(artResults));
  assert.strictEqual(artResults.length, 1);

  const art: ArtworkCandidate = artResults[0];
  assert.strictEqual(art.provider_name, 'Cover Art Archive');
  assert.strictEqual(art.kind, 'FrontCover');
  assert.strictEqual(art.is_canonical, true);
  assert.strictEqual(art.width, 1200);
  assert.strictEqual(art.height, 1200);
  assert.strictEqual(art.format, 'JPEG');
  assert.ok(art.match_confidence > 0.9);
});

test('Enrichment API: Mock lyrics discovery and duration alignment', async () => {
  const lyricsResults = await api.findLyricsCandidates({
    track_name: 'Get Lucky',
    artist_name: 'Daft Punk',
    duration_seconds: 248.0,
  });

  assert.ok(Array.isArray(lyricsResults));
  assert.strictEqual(lyricsResults.length, 1);

  const lyric: LyricsCandidate = lyricsResults[0];
  assert.strictEqual(lyric.track_name, 'Get Lucky');
  assert.strictEqual(lyric.artist_name, 'Daft Punk');
  assert.strictEqual(lyric.sync_type, 'LineSynced');
  assert.strictEqual(lyric.duration_seconds, 248.0);
  assert.strictEqual(lyric.duration_delta_seconds, 0.0);
  assert.ok(lyric.raw_content.includes('[00:05.00]'));
});

test('Enrichment State: Match Inspector open, track payload, and close flow', () => {
  assert.strictEqual(appState.isMatchInspectorVisible(), false);
  assert.strictEqual(appState.getActiveMatchTrack(), null);

  appState.openMatchInspector({
    trackId: 10,
    title: 'Instant Crush',
    artistName: 'Daft Punk',
    albumTitle: 'Random Access Memories',
    durationMs: 337000,
    trackNumber: 2,
  });

  assert.strictEqual(appState.isMatchInspectorVisible(), true);
  const active = appState.getActiveMatchTrack();
  assert.ok(active);
  assert.strictEqual(active.trackId, 10);
  assert.strictEqual(active.title, 'Instant Crush');

  appState.closeMatchInspector();
  assert.strictEqual(appState.isMatchInspectorVisible(), false);
  assert.strictEqual(appState.getActiveMatchTrack(), null);
});

test('Enrichment State: Artwork Finder open, target payload, and close flow', () => {
  assert.strictEqual(appState.isArtworkFinderVisible(), false);
  assert.strictEqual(appState.getActiveArtworkTarget(), null);

  appState.openArtworkFinder({
    targetType: 'album',
    targetId: 5,
    title: 'Discovery',
    artistName: 'Daft Punk',
    mbid: 'mbid-disc-001',
  });

  assert.strictEqual(appState.isArtworkFinderVisible(), true);
  const target = appState.getActiveArtworkTarget();
  assert.ok(target);
  assert.strictEqual(target.targetId, 5);
  assert.strictEqual(target.title, 'Discovery');
  assert.strictEqual(target.targetType, 'album');

  appState.closeArtworkFinder();
  assert.strictEqual(appState.isArtworkFinderVisible(), false);
  assert.strictEqual(appState.getActiveArtworkTarget(), null);
});

test('Enrichment State: Lyrics Manager open, track payload, and close flow', () => {
  assert.strictEqual(appState.isLyricsManagerVisible(), false);
  assert.strictEqual(appState.getActiveLyricsTrack(), null);

  appState.openLyricsManager({
    trackId: 2,
    filePath: '/music/instant_crush.flac',
    title: 'Instant Crush',
    artist: 'Daft Punk',
    album: 'Random Access Memories',
    durationMs: 337000,
  });

  assert.strictEqual(appState.isLyricsManagerVisible(), true);
  const lTrack = appState.getActiveLyricsTrack();
  assert.ok(lTrack);
  assert.strictEqual(lTrack.trackId, 2);
  assert.strictEqual(lTrack.title, 'Instant Crush');

  appState.closeLyricsManager();
  assert.strictEqual(appState.isLyricsManagerVisible(), false);
  assert.strictEqual(appState.getActiveLyricsTrack(), null);
});

test('Enrichment Operations: Metadata field diff computation invariants', () => {
  function computeFieldDiff(
    localVal: string | number | null | undefined,
    candidateVal: string | number | null | undefined
  ): { status: 'exact' | 'fuzzy' | 'conflict' | 'missing'; label: string } {
    const l = localVal !== null && localVal !== undefined ? String(localVal).trim() : '';
    const c = candidateVal !== null && candidateVal !== undefined ? String(candidateVal).trim() : '';

    if (!l && !c) {
      return { status: 'missing', label: 'Empty' };
    }
    if (!l && c) {
      return { status: 'fuzzy', label: 'New Data' };
    }
    if (l && !c) {
      return { status: 'missing', label: 'Missing Online' };
    }
    if (l.toLowerCase() === c.toLowerCase()) {
      return { status: 'exact', label: 'Exact Match' };
    }
    return { status: 'conflict', label: 'Conflict / Update' };
  }

  // Exact Match
  assert.strictEqual(computeFieldDiff('Get Lucky', 'Get Lucky').status, 'exact');
  assert.strictEqual(computeFieldDiff('Daft Punk', 'daft punk').status, 'exact');

  // Conflict / Update
  assert.strictEqual(
    computeFieldDiff('Get Lucky', 'Get Lucky (feat. Pharrell Williams)').status,
    'conflict'
  );

  // New Data (Fuzzy)
  assert.strictEqual(computeFieldDiff('', '2013').status, 'fuzzy');
  assert.strictEqual(computeFieldDiff(null, 'Columbia').status, 'fuzzy');

  // Missing
  assert.strictEqual(computeFieldDiff(null, null).status, 'missing');
  assert.strictEqual(computeFieldDiff('Custom Tag', null).status, 'missing');
});
