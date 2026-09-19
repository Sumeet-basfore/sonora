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

// ---------------------------------------------------------------------------
// Online Metadata & Enrichment Types (Sonora v0.2)
// ---------------------------------------------------------------------------

export type ConfidenceTier = 'high' | 'medium' | 'low';

export interface ExternalLink {
  link_type: string;
  target_url: string;
}

export interface OnlineArtistCredit {
  artist_mbid: string;
  name: string;
  join_phrase?: string | null;
}

export interface OnlineArtist {
  mbid: string;
  name: string;
  sort_name?: string | null;
  country?: string | null;
  disambiguation?: string | null;
  biography?: string | null;
  external_links: ExternalLink[];
}

export interface OnlineReleaseGroup {
  mbid: string;
  title: string;
  primary_type?: string | null;
  secondary_types: string[];
  first_release_date?: string | null;
  artist_credits: OnlineArtistCredit[];
}

export interface OnlineRelease {
  mbid: string;
  release_group_mbid?: string | null;
  title: string;
  status?: string | null;
  date?: string | null;
  country?: string | null;
  barcode?: string | null;
  media_format?: string | null;
  track_count: number;
  artist_credits: OnlineArtistCredit[];
  label?: string | null;
  catalog_number?: string | null;
}

export interface OnlineTrack {
  recording_mbid: string;
  release_mbid?: string | null;
  release_group_mbid?: string | null;
  position?: number | null;
  number?: string | null;
  title: string;
  duration_ms?: number | null;
  artist_credits: OnlineArtistCredit[];
  isrcs: string[];
}

export interface MatchScoreBreakdown {
  total_score: number;
  confidence_tier: ConfidenceTier;
  title_score: number;
  artist_score: number;
  album_score: number;
  duration_score: number;
  track_number_score: number;
  is_exact_shortcut: boolean;
  shortcut_reason?: string | null;
}

export interface RankedCandidateMatch {
  candidate_track: OnlineTrack;
  candidate_release?: OnlineRelease | null;
  candidate_release_group?: OnlineReleaseGroup | null;
  score_breakdown: MatchScoreBreakdown;
}

// Artwork Types
export type ArtworkKind =
  | 'FrontCover'
  | 'BackCover'
  | 'Booklet'
  | 'Medium'
  | 'ArtistPortrait'
  | 'ArtistBackground'
  | 'ArtistBanner'
  | 'ArtistLogo'
  | 'Other';

export interface ArtworkCandidate {
  id: string;
  provider_name: string;
  source_type: Record<string, unknown>;
  kind: ArtworkKind;
  original_url: string;
  preview_thumbnail_url: string;
  width: number;
  height: number;
  format: string;
  size_bytes?: number | null;
  match_confidence: number;
  is_canonical: boolean;
}

export interface ArtworkQuery {
  release_mbid?: string | null;
  release_group_mbid?: string | null;
  artist_mbid?: string | null;
  artist_name?: string | null;
  album_title?: string | null;
}

export interface CachedArtworkAsset {
  key: string;
  full_path: string;
  thumbnail_path: string;
  width: number;
  height: number;
  mime_type: string;
  file_size_bytes: number;
}

// Lyrics Candidate Types
export type LyricsSyncType = 'SyllableSynced' | 'LineSynced' | 'PlainText';

export interface LyricsCandidate {
  candidate_id: string;
  source_kind: Record<string, unknown>;
  provider_name: string;
  sync_type: LyricsSyncType;
  track_name: string;
  artist_name: string;
  album_name?: string | null;
  duration_seconds: number;
  duration_delta_seconds: number;
  match_confidence: number;
  language_code?: string | null;
  is_instrumental: boolean;
  raw_content: string;
}

export interface LyricsCandidateQuery {
  track_name: string;
  artist_name?: string | null;
  album_name?: string | null;
  duration_seconds?: number | null;
}

