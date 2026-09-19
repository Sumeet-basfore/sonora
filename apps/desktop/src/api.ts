import type {
  AlbumDto,
  ArtistDto,
  LibrarySummary,
  PlaybackStatus,
  QueueItem,
  ScanStats,
  SearchResult,
} from './types';

function checkIsTauri(): boolean {
  return typeof window !== 'undefined' && ('__TAURI_INTERNALS__' in window || '__TAURI__' in window);
}

async function invokeTauri<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (checkIsTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<T>(cmd, args);
  }
  return mockInvoke<T>(cmd, args);
}

export const api = {
  getSystemStatus: () => invokeTauri<string>('get_system_status'),
  scanDirectory: (path: string) => invokeTauri<ScanStats>('scan_directory', { path }),
  searchLibrary: (query: string, limit?: number) =>
    invokeTauri<SearchResult[]>('search_library', { query, limit }),
  getLibrarySummary: () => invokeTauri<LibrarySummary>('get_library_summary'),
  getAllTracks: (limit?: number) => invokeTauri<SearchResult[]>('get_all_tracks', { limit }),
  getAllAlbums: () => invokeTauri<AlbumDto[]>('get_all_albums'),
  getAllArtists: () => invokeTauri<ArtistDto[]>('get_all_artists'),
  getAlbumTracks: (albumId: number) => invokeTauri<SearchResult[]>('get_album_tracks', { albumId }),
  getArtistTracks: (artistId: number) =>
    invokeTauri<SearchResult[]>('get_artist_tracks', { artistId }),
  getTrackArtwork: (trackId: number, thumbnail: boolean = false) =>
    invokeTauri<string | null>('get_track_artwork', { trackId, thumbnail }),
  getAlbumArtwork: (albumId: number, thumbnail: boolean = false) =>
    invokeTauri<string | null>('get_album_artwork', { albumId, thumbnail }),
  pickDirectory: () => invokeTauri<string | null>('pick_directory'),
  pickAudioFile: () => invokeTauri<string | null>('pick_audio_file'),

  playTrack: (trackId: number) => invokeTauri<void>('play_track', { trackId }),
  playFile: (path: string) => invokeTauri<void>('play_file', { path }),
  playAlbum: (albumId: number) => invokeTauri<void>('play_album', { albumId }),
  pausePlayback: () => invokeTauri<void>('pause_playback'),
  resumePlayback: () => invokeTauri<void>('resume_playback'),
  stopPlayback: () => invokeTauri<void>('stop_playback'),
  seekPlayback: (positionMs: number) =>
    invokeTauri<void>('seek_playback', { positionMs }),
  setVolume: (volume: number) => invokeTauri<void>('set_volume', { volume }),

  getPlaybackStatus: () => invokeTauri<PlaybackStatus>('get_playback_status'),
  getQueue: () => invokeTauri<QueueItem[]>('get_queue'),
  enqueueTrack: (trackId: number) => invokeTauri<void>('enqueue_track', { trackId }),
  removeFromQueue: (index: number) =>
    invokeTauri<QueueItem | null>('remove_from_queue', { index }),
  moveQueueItem: (from: number, to: number) =>
    invokeTauri<boolean>('move_queue_item', { from, to }),
  playQueueIndex: (index: number) => invokeTauri<void>('play_queue_index', { index }),
  clearQueue: () => invokeTauri<void>('clear_queue'),
  queueNext: () => invokeTauri<boolean>('queue_next'),
  queuePrevious: () => invokeTauri<boolean>('queue_previous'),

  getVisualizerData: () => invokeTauri<number[]>('get_visualizer_data'),

  getLyrics: (
    trackId?: number | null,
    filePath?: string | null,
    title: string = '',
    artist?: string | null,
    album?: string | null,
    durationMs?: number | null
  ) =>
    invokeTauri<import('./types').LyricsDocument | null>('get_lyrics', {
      trackId,
      filePath,
      title,
      artist,
      album,
      durationMs,
    }),

  saveLyricsOffset: (
    trackId: number | null | undefined,
    filePath: string | null | undefined,
    offsetMs: number
  ) =>
    invokeTauri<void>('save_lyrics_offset', {
      trackId,
      filePath,
      offsetMs,
    }),

  pickLrcFile: () => invokeTauri<string | null>('pick_lrc_file'),

  loadLrcFile: (
    path: string,
    trackId?: number | null,
    filePath?: string | null,
    title?: string | null,
    artist?: string | null
  ) =>
    invokeTauri<import('./types').LyricsDocument>('load_lrc_file', {
      path,
      trackId,
      filePath,
      title,
      artist,
    }),

  marketplaceCatalog: () =>
    invokeTauri<import('./types').MarketplaceCatalog>('marketplace_catalog'),
  marketplaceRefresh: () =>
    invokeTauri<import('./types').MarketplaceCatalog>('marketplace_refresh'),
  marketplaceInstalled: () =>
    invokeTauri<import('./types').InstalledEntry[]>('marketplace_installed'),
  marketplaceUpdates: () =>
    invokeTauri<import('./types').UpdateInfo[]>('marketplace_updates'),
  marketplaceInstall: (id: string, version?: string | null) =>
    invokeTauri<import('./types').InstallReport>('marketplace_install', { id, version }),
  marketplaceUpdate: (id: string) =>
    invokeTauri<import('./types').InstallReport>('marketplace_update', { id }),
  marketplaceRollback: (id: string, version?: string | null) =>
    invokeTauri<import('./types').RollbackReport>('marketplace_rollback', { id, version }),
  marketplaceUninstall: (id: string) => invokeTauri<void>('marketplace_uninstall', { id }),

  marketplaceThemes: () =>
    invokeTauri<import('./types').InstalledTheme[]>('marketplace_themes'),
  marketplaceThemeDefinition: (id: string) =>
    invokeTauri<import('./types').ThemeDefinition>('marketplace_theme_definition', { id }),
  marketplaceThemeCss: (id: string) =>
    invokeTauri<string | null>('marketplace_theme_css', { id }),
  marketplaceActiveTheme: () =>
    invokeTauri<string | null>('marketplace_active_theme'),
  marketplaceSetActiveTheme: (id: string | null) =>
    invokeTauri<void>('marketplace_set_active_theme', { id }),

  // Online Enrichment (Sonora v0.2 Phase 3)
  findMetadataCandidates: (trackId: number) =>
    invokeTauri<import('./types').RankedCandidateMatch[]>('enrichment_find_metadata', { trackId }),

  applyMetadata: (trackId: number, candidate: import('./types').RankedCandidateMatch) =>
    invokeTauri<void>('enrichment_apply_metadata', { trackId, candidate }),

  findArtworkCandidates: (query: import('./types').ArtworkQuery) =>
    invokeTauri<import('./types').ArtworkCandidate[]>('enrichment_find_artwork', { query }),

  applyArtwork: (targetType: string, targetId: number, imageUrl: string) =>
    invokeTauri<import('./types').CachedArtworkAsset>('enrichment_apply_artwork', {
      targetType,
      targetId,
      imageUrl,
    }),

  findLyricsCandidates: (query: import('./types').LyricsCandidateQuery) =>
    invokeTauri<import('./types').LyricsCandidate[]>('enrichment_find_lyrics', { query }),

  applyLyricsCandidate: (
    candidate: import('./types').LyricsCandidate,
    trackId?: number | null,
    filePath?: string | null
  ) =>
    invokeTauri<void>('enrichment_apply_lyrics', {
      candidate,
      trackId,
      filePath,
    }),

  exportLrcSidecar: (
    lrcContent: string,
    trackId?: number | null,
    filePath?: string | null
  ) =>
    invokeTauri<string>('enrichment_export_lrc', {
      lrcContent,
      trackId,
      filePath,
    }),
};

// In-memory mock for web preview/development
let mockStatus: PlaybackStatus = {
  state: 'Stopped',
  current_track: null,
  position_ms: 0,
  duration_ms: 0,
  volume: 1.0,
  queue_length: 0,
  current_queue_index: null,
};

let mockQueue: QueueItem[] = [];

async function mockInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  console.debug(`[Web Mock API] invoke: ${cmd}`, args);
  switch (cmd) {
    case 'get_system_status':
      return 'Sonora Core v0.1.0 Ready (Web Preview Mode)' as T;
    case 'get_library_summary':
      return { track_count: 2, album_count: 1, artist_count: 1 } as T;
    case 'get_all_albums':
      return [
        {
          id: 1,
          title: 'Random Access Memories',
          artist_id: 1,
          artist_name: 'Daft Punk',
          release_year: 2013,
          track_count: 2,
          total_duration_ms: 450000,
        },
      ] as T;
    case 'get_all_artists':
      return [
        {
          id: 1,
          name: 'Daft Punk',
          album_count: 1,
          track_count: 2,
        },
      ] as T;
    case 'get_all_tracks':
    case 'get_album_tracks':
    case 'get_artist_tracks':
      return [
        {
          track_id: 1,
          file_path: '/music/get_lucky.flac',
          title: 'Get Lucky',
          artist_name: 'Daft Punk',
          album_title: 'Random Access Memories',
          duration_ms: 248000,
          track_number: 1,
        },
        {
          track_id: 2,
          file_path: '/music/instant_crush.flac',
          title: 'Instant Crush',
          artist_name: 'Daft Punk',
          album_title: 'Random Access Memories',
          duration_ms: 337000,
          track_number: 2,
        },
      ] as T;
    case 'get_playback_status':
      return mockStatus as T;
    case 'get_queue':
      return mockQueue as T;
    case 'play_track': {
      mockStatus = {
        ...mockStatus,
        state: 'Playing',
        position_ms: 0,
        duration_ms: 248000,
        current_track: {
          track_id: 1,
          file_path: '/music/get_lucky.flac',
          title: 'Get Lucky',
          artist: 'Daft Punk',
          album: 'Random Access Memories',
          duration_ms: 248000,
        },
      };
      return undefined as T;
    }
    case 'pause_playback':
      mockStatus = { ...mockStatus, state: 'Paused' };
      return undefined as T;
    case 'resume_playback':
      mockStatus = { ...mockStatus, state: 'Playing' };
      return undefined as T;
    case 'stop_playback':
      mockStatus = { ...mockStatus, state: 'Stopped', position_ms: 0 };
      return undefined as T;
    case 'set_volume':
      mockStatus = { ...mockStatus, volume: (args?.volume as number) ?? 1.0 };
      return undefined as T;
    case 'seek_playback':
      mockStatus = { ...mockStatus, position_ms: (args?.positionMs as number) ?? 0 };
      return undefined as T;
    case 'get_visualizer_data': {
      if (mockStatus.state !== 'Playing') return new Array(64).fill(0) as T;
      const t = Date.now() / 200;
      return Array.from({ length: 64 }, (_, i) => {
        return Math.sin(t + i * 0.2) * 0.5 + 0.5;
      }) as T;
    }
    case 'get_lyrics': {
      return {
        title: 'Get Lucky',
        artist: 'Daft Punk',
        album: 'Random Access Memories',
        offset_ms: 0,
        format: 'Lrc',
        lines: [
          { start_time_ms: 0, end_time_ms: 12000, text: '(Intro - Synthesizer Groove)', syllables: [] },
          { start_time_ms: 12000, end_time_ms: 16500, text: 'Like the legend of the phoenix', syllables: [] },
          { start_time_ms: 16500, end_time_ms: 20000, text: 'All ends with beginnings', syllables: [] },
          { start_time_ms: 20000, end_time_ms: 24000, text: 'What keeps the planet turning (uh)', syllables: [] },
          { start_time_ms: 24000, end_time_ms: 28000, text: 'The force from the beginning', syllables: [] },
          { start_time_ms: 28000, end_time_ms: 32000, text: "We've come too far to give up who we are", syllables: [] },
          { start_time_ms: 32000, end_time_ms: 36000, text: "So let's raise the bar and our cups to the stars", syllables: [] },
          { start_time_ms: 36000, end_time_ms: 40000, text: "She's up all night 'til the sun", syllables: [] },
          { start_time_ms: 40000, end_time_ms: 44000, text: "I'm up all night to get some", syllables: [] },
          { start_time_ms: 44000, end_time_ms: 48000, text: "She's up all night for good fun", syllables: [] },
          { start_time_ms: 48000, end_time_ms: 55000, text: "I'm up all night to get lucky", syllables: [] },
          { start_time_ms: 55000, end_time_ms: 60000, text: "We're up all night 'til the sun", syllables: [] },
          { start_time_ms: 60000, end_time_ms: 64000, text: "We're up all night to get some", syllables: [] },
          { start_time_ms: 64000, end_time_ms: 68000, text: "We're up all night for good fun", syllables: [] },
          { start_time_ms: 68000, end_time_ms: 76000, text: "We're up all night to get lucky", syllables: [] },
        ],
      } as T;
    }
    case 'save_lyrics_offset':
      return undefined as T;
    case 'pick_lrc_file':
      return '/music/lyrics/get_lucky.lrc' as T;
    case 'marketplace_catalog':
    case 'marketplace_refresh':
      return {
        plugins: [
          {
            kind: 'plugin',
            id: 'org.sonora.lrclib',
            name: 'LRCLIB Lyrics Provider',
            description: 'Synced lyrics from the LRCLIB public API.',
            author: 'Sonora Contributors',
            author_url: null,
            category: 'lyrics',
            homepage: 'https://lrclib.net',
            capabilities: ['lyrics:provider', 'network:fetch'],
            latest_version: '1.0.0',
            min_sonora_version: '0.1.0',
            installed_version: '1.0.0',
            update_available: null,
          },
        ],
        themes: [
          {
            kind: 'theme',
            id: 'org.sonora.theme.neon_night',
            name: 'Neon Night',
            description: 'Dark theme with neon accents.',
            author: 'Sonora Contributors',
            author_url: null,
            category: 'dark',
            homepage: null,
            capabilities: [],
            latest_version: '1.0.0',
            min_sonora_version: '0.1.0',
            installed_version: null,
            update_available: null,
          },
        ],
        offline: false,
        stale: false,
        fetched_at: Math.floor(Date.now() / 1000),
        registry_url: 'https://registry.sonora.audio/v1',
      } as T;
    case 'marketplace_installed':
      return [
        {
          id: 'org.sonora.lrclib',
          kind: 'plugin',
          name: 'LRCLIB Lyrics Provider',
          version: '1.0.0',
          state: 'running',
          capabilities: ['lyrics:provider', 'network:fetch'],
        },
      ] as T;
    case 'marketplace_updates':
      return [] as T;
    case 'marketplace_install':
      return {
        id: args?.id,
        kind: 'plugin',
        version: '1.0.0',
        fresh_install: true,
        backed_up_version: null,
        started: true,
        start_error: null,
      } as T;
    case 'marketplace_update':
      return {
        id: args?.id,
        kind: 'plugin',
        version: '1.1.0',
        fresh_install: false,
        backed_up_version: '1.0.0',
        started: true,
        start_error: null,
      } as T;
    case 'marketplace_rollback':
      return { id: args?.id, restored_version: '1.0.0', previous_version: '1.1.0' } as T;
    case 'marketplace_uninstall':
      return undefined as T;
    case 'marketplace_themes':
      return [] as T;
    case 'marketplace_theme_definition':
      return null as T;
    case 'marketplace_theme_css':
      return null as T;
    case 'marketplace_active_theme':
      return null as T;
    case 'marketplace_set_active_theme':
      return undefined as T;
    case 'load_lrc_file':
      return {
        title: 'Custom Local Lyric',
        artist: 'Daft Punk',
        album: null,
        offset_ms: 0,
        format: 'Lrc',
        lines: [
          { start_time_ms: 0, end_time_ms: 5000, text: 'Custom Sidecar Lyric Line 1', syllables: [] },
          { start_time_ms: 5000, end_time_ms: 10000, text: 'Custom Sidecar Lyric Line 2', syllables: [] },
        ],
      } as T;
    case 'get_track_artwork':
    case 'get_album_artwork':
      return null as T;
    case 'enrichment_find_metadata':
      return [
        {
          candidate_track: {
            recording_mbid: 'mbid-rec-001',
            release_mbid: 'mbid-rel-001',
            release_group_mbid: 'mbid-rg-001',
            position: 1,
            number: '1',
            title: 'Get Lucky (feat. Pharrell Williams & Nile Rodgers)',
            duration_ms: 248000,
            artist_credits: [
              { artist_mbid: 'mbid-art-001', name: 'Daft Punk', join_phrase: ' feat. ' },
              { artist_mbid: 'mbid-art-002', name: 'Pharrell Williams', join_phrase: ' & ' },
              { artist_mbid: 'mbid-art-003', name: 'Nile Rodgers', join_phrase: null },
            ],
            isrcs: ['USQX91300105'],
          },
          candidate_release: {
            mbid: 'mbid-rel-001',
            release_group_mbid: 'mbid-rg-001',
            title: 'Random Access Memories',
            status: 'Official',
            date: '2013-05-17',
            country: 'US',
            barcode: '888837168627',
            media_format: 'CD',
            track_count: 13,
            artist_credits: [{ artist_mbid: 'mbid-art-001', name: 'Daft Punk', join_phrase: null }],
            label: 'Columbia',
            catalog_number: '88883716862',
          },
          candidate_release_group: {
            mbid: 'mbid-rg-001',
            title: 'Random Access Memories',
            primary_type: 'Album',
            secondary_types: [],
            first_release_date: '2013-05-17',
            artist_credits: [{ artist_mbid: 'mbid-art-001', name: 'Daft Punk', join_phrase: null }],
          },
          score_breakdown: {
            total_score: 0.96,
            confidence_tier: 'high',
            title_score: 0.95,
            artist_score: 1.0,
            album_score: 1.0,
            duration_score: 0.98,
            track_number_score: 1.0,
            is_exact_shortcut: false,
            shortcut_reason: null,
          },
        },
      ] as T;
    case 'enrichment_apply_metadata':
      return undefined as T;
    case 'enrichment_find_artwork':
      return [
        {
          id: 'caa-front-001',
          provider_name: 'Cover Art Archive',
          source_type: { type: 'CoverArtArchive', details: { release_mbid: 'mbid-rel-001' } },
          kind: 'FrontCover',
          original_url: 'https://coverartarchive.org/release/mbid-rel-001/front.jpg',
          preview_thumbnail_url: 'https://coverartarchive.org/release/mbid-rel-001/front-250.jpg',
          width: 1200,
          height: 1200,
          format: 'JPEG',
          size_bytes: 524288,
          match_confidence: 0.98,
          is_canonical: true,
        },
      ] as T;
    case 'enrichment_apply_artwork':
      return {
        key: 'art-mock-001',
        full_path: '/cache/artwork/art-mock-001.webp',
        thumbnail_path: '/cache/artwork/art-mock-001_thumb.webp',
        width: 1200,
        height: 1200,
        mime_type: 'image/webp',
        file_size_bytes: 262144,
      } as T;
    case 'enrichment_find_lyrics':
      return [
        {
          candidate_id: 'lrclib-001',
          source_kind: { type: 'LrclibPublicApi', details: { id: 101 } },
          provider_name: 'LRCLIB',
          sync_type: 'LineSynced',
          track_name: 'Get Lucky',
          artist_name: 'Daft Punk',
          album_name: 'Random Access Memories',
          duration_seconds: 248.0,
          duration_delta_seconds: 0.0,
          match_confidence: 0.98,
          language_code: 'en',
          is_instrumental: false,
          raw_content: '[00:05.00] Like the legend of the phoenix\n[00:09.00] All ends with beginnings\n[00:13.00] What keeps the planet spinning\n[00:17.00] The force from the beginning\n[00:21.00] We\'ve come too far to give up who we are\n[00:25.00] So let\'s raise the bar and our cups to the stars\n[00:29.00] She\'s up all night \'til the sun\n[00:31.00] I\'m up all night to get some\n[00:33.00] She\'s up all night for good fun\n[00:35.00] I\'m up all night to get lucky',
        },
      ] as T;
    case 'enrichment_apply_lyrics':
      return undefined as T;
    case 'enrichment_export_lrc':
      return '/music/get_lucky.lrc' as T;
    default:
      return [] as unknown as T;
  }
}
