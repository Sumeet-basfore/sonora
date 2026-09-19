import { api } from './api.ts';
import type { ActiveView, PlaybackStatus, QueueItem } from './types.ts';

type Listener = () => void;

class StateManager {
  private status: PlaybackStatus = {
    state: 'Stopped',
    current_track: null,
    position_ms: 0,
    duration_ms: 0,
    volume: 1.0,
    queue_length: 0,
    current_queue_index: null,
  };

  private queue: QueueItem[] = [];
  private activeView: ActiveView = { type: 'albums' };
  private isQueueOpen: boolean = false;
  private isVisualizerOpen: boolean = false;
  private isScanModalOpen: boolean = false;
  private pollIntervalId: number | null = null;
  private listeners: Set<Listener> = new Set();
  private artworkCache: Map<string, string | null> = new Map();

  constructor() {
    if (typeof window !== 'undefined') {
      this.startPolling();
    }
  }

  public subscribe(listener: Listener): () => void {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  }

  private notify() {
    for (const listener of this.listeners) {
      try {
        listener();
      } catch (err) {
        console.error('Listener error in StateManager:', err);
      }
    }
  }

  public getStatus(): PlaybackStatus {
    return this.status;
  }

  public getQueue(): QueueItem[] {
    return this.queue;
  }

  public getActiveView(): ActiveView {
    return this.activeView;
  }

  public setActiveView(view: ActiveView) {
    this.activeView = view;
    this.notify();
  }

  public isQueueVisible(): boolean {
    return this.isQueueOpen;
  }

  public toggleQueue(show?: boolean) {
    this.isQueueOpen = show !== undefined ? show : !this.isQueueOpen;
    this.notify();
  }

  public isVisualizerVisible(): boolean {
    return this.isVisualizerOpen;
  }

  public toggleVisualizer(show?: boolean) {
    this.isVisualizerOpen = show !== undefined ? show : !this.isVisualizerOpen;
    this.notify();
  }

  public isScanModalVisible(): boolean {
    return this.isScanModalOpen;
  }

  public toggleScanModal(show?: boolean) {
    this.isScanModalOpen = show !== undefined ? show : !this.isScanModalOpen;
    this.notify();
  }

  public async fetchArtwork(trackId: number, thumbnail: boolean = true): Promise<string | null> {
    const key = `track_${trackId}_${thumbnail ? 'thumb' : 'full'}`;
    if (this.artworkCache.has(key)) {
      return this.artworkCache.get(key) ?? null;
    }
    try {
      const art = await api.getTrackArtwork(trackId, thumbnail);
      this.artworkCache.set(key, art);
      return art;
    } catch {
      this.artworkCache.set(key, null);
      return null;
    }
  }

  public async fetchAlbumArtwork(albumId: number, thumbnail: boolean = true): Promise<string | null> {
    const key = `album_${albumId}_${thumbnail ? 'thumb' : 'full'}`;
    if (this.artworkCache.has(key)) {
      return this.artworkCache.get(key) ?? null;
    }
    try {
      const art = await api.getAlbumArtwork(albumId, thumbnail);
      this.artworkCache.set(key, art);
      return art;
    } catch {
      this.artworkCache.set(key, null);
      return null;
    }
  }

  public startPolling() {
    if (this.pollIntervalId !== null) return;
    this.refresh();
    const setIntervalFn = typeof window !== 'undefined' ? window.setInterval : globalThis.setInterval;
    if (setIntervalFn) {
      this.pollIntervalId = setIntervalFn(() => {
        this.refreshStatusOnly();
      }, 250) as unknown as number;
    }
  }

  public stopPolling() {
    if (this.pollIntervalId !== null) {
      const clearIntervalFn = typeof window !== 'undefined' ? window.clearInterval : globalThis.clearInterval;
      if (clearIntervalFn) {
        clearIntervalFn(this.pollIntervalId as any);
      }
      this.pollIntervalId = null;
    }
  }

  public async refresh() {
    try {
      const [status, queue] = await Promise.all([
        api.getPlaybackStatus(),
        api.getQueue(),
      ]);
      this.status = status;
      this.queue = queue;
      this.notify();
    } catch (e) {
      console.warn('Status refresh error:', e);
    }
  }

  private async refreshStatusOnly() {
    try {
      const status = await api.getPlaybackStatus();
      const statusChanged =
        this.status.state !== status.state ||
        this.status.position_ms !== status.position_ms ||
        this.status.duration_ms !== status.duration_ms ||
        this.status.queue_length !== status.queue_length ||
        this.status.current_queue_index !== status.current_queue_index ||
        this.status.current_track?.title !== status.current_track?.title;

      this.status = status;
      if (statusChanged) {
        // Also sync queue if queue length or current track changed
        if (this.isQueueOpen) {
          this.queue = await api.getQueue();
        }
        this.notify();
      }
    } catch (e) {
      // Backend maybe initializing
    }
  }

  // --- Actions ---

  public async playTrack(trackId: number) {
    await api.playTrack(trackId);
    await this.refresh();
  }

  public async playAlbum(albumId: number) {
    await api.playAlbum(albumId);
    await this.refresh();
  }

  public async togglePlayPause() {
    if (this.status.state === 'Playing') {
      await api.pausePlayback();
    } else if (this.status.state === 'Paused') {
      await api.resumePlayback();
    } else if (this.queue.length > 0) {
      await api.playQueueIndex(0);
    }
    await this.refresh();
  }

  public async next() {
    await api.queueNext();
    await this.refresh();
  }

  public async previous() {
    await api.queuePrevious();
    await this.refresh();
  }

  public async seek(positionMs: number) {
    await api.seekPlayback(positionMs);
    await this.refresh();
  }

  public async setVolume(volume: number) {
    await api.setVolume(volume);
    this.status.volume = volume;
    this.notify();
  }

  public async playQueueIndex(index: number) {
    await api.playQueueIndex(index);
    await this.refresh();
  }

  public async removeFromQueue(index: number) {
    await api.removeFromQueue(index);
    await this.refresh();
  }

  public async moveQueueItem(from: number, to: number) {
    await api.moveQueueItem(from, to);
    await this.refresh();
  }

  public async clearQueue() {
    await api.clearQueue();
    await this.refresh();
  }

  // --- Enrichment Contextual UI State ---

  private isMatchInspectorOpen: boolean = false;
  private activeMatchTrack: {
    trackId: number;
    title: string;
    artistName?: string | null;
    albumTitle?: string | null;
    durationMs?: number;
    trackNumber?: number | null;
  } | null = null;

  public isMatchInspectorVisible(): boolean {
    return this.isMatchInspectorOpen;
  }

  public getActiveMatchTrack() {
    return this.activeMatchTrack;
  }

  public openMatchInspector(track: {
    trackId: number;
    title: string;
    artistName?: string | null;
    albumTitle?: string | null;
    durationMs?: number;
    trackNumber?: number | null;
  }) {
    this.activeMatchTrack = track;
    this.isMatchInspectorOpen = true;
    this.notify();
  }

  public closeMatchInspector() {
    this.isMatchInspectorOpen = false;
    this.activeMatchTrack = null;
    this.notify();
  }

  private isArtworkFinderOpen: boolean = false;
  private activeArtworkTarget: {
    targetType: 'album' | 'artist' | 'track';
    targetId: number;
    title: string;
    artistName?: string | null;
    mbid?: string | null;
  } | null = null;

  public isArtworkFinderVisible(): boolean {
    return this.isArtworkFinderOpen;
  }

  public getActiveArtworkTarget() {
    return this.activeArtworkTarget;
  }

  public openArtworkFinder(target: {
    targetType: 'album' | 'artist' | 'track';
    targetId: number;
    title: string;
    artistName?: string | null;
    mbid?: string | null;
  }) {
    this.activeArtworkTarget = target;
    this.isArtworkFinderOpen = true;
    this.notify();
  }

  public closeArtworkFinder() {
    this.isArtworkFinderOpen = false;
    this.activeArtworkTarget = null;
    this.notify();
  }

  private isLyricsManagerOpen: boolean = false;
  private activeLyricsTrack: {
    trackId?: number | null;
    filePath?: string | null;
    title: string;
    artist?: string | null;
    album?: string | null;
    durationMs?: number | null;
  } | null = null;

  public isLyricsManagerVisible(): boolean {
    return this.isLyricsManagerOpen;
  }

  public getActiveLyricsTrack() {
    return this.activeLyricsTrack;
  }

  public openLyricsManager(track?: {
    trackId?: number | null;
    filePath?: string | null;
    title: string;
    artist?: string | null;
    album?: string | null;
    durationMs?: number | null;
  }) {
    if (track) {
      this.activeLyricsTrack = track;
    } else if (this.status.current_track) {
      this.activeLyricsTrack = {
        trackId: this.status.current_track.track_id,
        filePath: this.status.current_track.file_path,
        title: this.status.current_track.title,
        artist: this.status.current_track.artist,
        album: this.status.current_track.album,
        durationMs: this.status.current_track.duration_ms,
      };
    }
    this.isLyricsManagerOpen = true;
    this.notify();
  }

  public closeLyricsManager() {
    this.isLyricsManagerOpen = false;
    this.activeLyricsTrack = null;
    this.notify();
  }
}

export const appState = new StateManager();
