export type PlaybackState = 'Stopped' | 'Playing' | 'Paused';

export interface QueueItem {
  track_id?: number | null;
  file_path: string;
  title: string;
  artist?: string | null;
  album?: string | null;
  duration_ms: number;
}

export interface PlaybackStatus {
  state: PlaybackState;
  current_track?: QueueItem | null;
  position_ms: number;
  duration_ms: number;
  volume: number;
  queue_length: number;
  current_queue_index?: number | null;
}

export interface SearchResult {
  track_id: number;
  file_path: string;
  title: string;
  artist_name?: string | null;
  album_title?: string | null;
  duration_ms: number;
  track_number?: number | null;
}

export interface AlbumDto {
  id: number;
  title: string;
  artist_id?: number | null;
  artist_name?: string | null;
  release_year?: number | null;
  track_count: number;
  total_duration_ms: number;
}

export interface ArtistDto {
  id: number;
  name: string;
  album_count: number;
  track_count: number;
}

export interface LibrarySummary {
  track_count: number;
  album_count: number;
  artist_count: number;
}

export interface ScanStats {
  scanned_files: number;
  indexed_tracks: number;
  skipped_unmodified: number;
  errors: number;
}

export type ActiveView = 
  | { type: 'albums' }
  | { type: 'artists' }
  | { type: 'tracks' }
  | { type: 'search'; query: string }
  | { type: 'album_detail'; albumId: number; albumTitle: string; artistName?: string }
  | { type: 'artist_detail'; artistId: number; artistName: string }
  | { type: 'marketplace' };

// Marketplace Types (mirrors sonora-registry DTOs)

export type ExtensionKind = 'plugin' | 'theme';

export interface CatalogEntry {
  kind: ExtensionKind;
  id: string;
  name: string;
  description: string;
  author: string;
  author_url?: string | null;
  category: string;
  homepage?: string | null;
  capabilities: string[];
  latest_version: string;
  min_sonora_version: string;
  installed_version?: string | null;
  update_available?: string | null;
}

export interface MarketplaceCatalog {
  plugins: CatalogEntry[];
  themes: CatalogEntry[];
  offline: boolean;
  stale: boolean;
  fetched_at: number;
  registry_url: string;
}

export interface InstalledEntry {
  id: string;
  kind: ExtensionKind;
  name: string;
  version: string;
  state: string;
  capabilities: string[];
}

export interface UpdateInfo {
  id: string;
  kind: ExtensionKind;
  name: string;
  current: string;
  available: string;
  changelog: string;
  compatible: boolean;
  min_sonora_version: string;
}

export interface InstallReport {
  id: string;
  kind: ExtensionKind;
  version: string;
  fresh_install: boolean;
  backed_up_version?: string | null;
  started: boolean;
  start_error?: string | null;
}

export interface RollbackReport {
  id: string;
  restored_version: string;
  previous_version: string;
}

export interface PluginInfo {
  id: string;
  name: string;
  version: string;
  state: string;
  capabilities: string[];
}

export type { ThemeDefinition } from './customization/theme/schema';

export interface InstalledTheme {
  id: string;
  name: string;
  version: string;
  mode: string;
  description?: string | null;
  author: string;
  definition: import('./customization/theme/schema').ThemeDefinition;
  has_css: boolean;
  active: boolean;
}

// Lyrics System Types
export type LyricsFormat = 'Plain' | 'Lrc' | 'EnhancedLrc' | 'Ttml';

export interface LyricSyllable {
  text: string;
  start_time_ms: number;
  end_time_ms: number;
}

export interface LyricLine {
  start_time_ms: number;
  end_time_ms?: number | null;
  text: string;
  syllables: LyricSyllable[];
}

export interface LyricsDocument {
  title?: string | null;
  artist?: string | null;
  album?: string | null;
  offset_ms: number;
  format: LyricsFormat;
  lines: LyricLine[];
}

export type LyricsDisplayMode = 'classic' | 'focused' | 'compact' | 'minimal' | 'dual';
export type LyricsFontSize = 'sm' | 'md' | 'lg' | 'xl';
export type LyricsFontWeight = 'normal' | 'medium' | 'bold';
export type LyricsAlignment = 'left' | 'center' | 'right';
export type LyricsLineSpacing = 'compact' | 'normal' | 'relaxed';
export type LyricsHighlightStyle = 'glow' | 'accent' | 'underline' | 'scale';
export type LyricsBackgroundMode = 'transparent' | 'surface' | 'blurred';

export interface LyricsPreferences {
  mode: LyricsDisplayMode;
  fontSize: LyricsFontSize;
  fontWeight: LyricsFontWeight;
  alignment: LyricsAlignment;
  lineSpacing: LyricsLineSpacing;
  activeOpacity: number;
  inactiveOpacity: number;
  highlightStyle: LyricsHighlightStyle;
  backgroundMode: LyricsBackgroundMode;
  autoScroll: boolean;
  manualOffsetMs: number;
}

