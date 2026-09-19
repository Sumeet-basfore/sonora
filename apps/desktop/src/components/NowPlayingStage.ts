/**
 * Sonora Now Playing & Context Stage Component.
 * 
 * Provides a strong visual center of gravity for the currently playing track:
 * - High-resolution artwork display with vinyl styling and ambient glow
 * - Track, artist, and album identity with deep library navigation links
 * - Stream format badges (Format, Sample Rate, Bit Depth, Bit-Perfect status)
 * - Live synchronized mini-lyrics snippet with glowing kinetic text fill
 * - Up Next queue preview card with quick skip
 * - Quick action controls (Heart, View Album, View Artist, Open Lyrics, Open Visualizer)
 * - Inspiring vinyl showcase empty state when idle
 */

import { api } from '../api';
import { appState } from '../state';
import { escapeHtml } from '../escape';
import { lyricsManager } from '../lyricsManager';
import type { LyricsDocument } from '../types';

export class NowPlayingStageComponent {
  private container: HTMLElement;
  private currentTrackKey: string | null = null;
  private lyricsDoc: LyricsDocument | null = null;
  private currentLyricLine: string = '';
  private nextLyricLine: string = '';

  constructor(container: HTMLElement) {
    this.container = container;
    this.render();
    appState.subscribe(() => this.update());
    lyricsManager.subscribe(() => this.updateLyrics());
  }

  private render() {
    this.container.innerHTML = `
      <div class="now-playing-stage-inner">
        <!-- Stage Header -->
        <div class="stage-header">
          <span class="stage-title">Now Playing</span>
          <div class="stage-badges" id="stage-stream-badges">
            <span class="audio-badge badge-format">Lossless</span>
          </div>
        </div>

        <!-- Stage Artwork Showcase -->
        <div class="stage-artwork-card">
          <div class="stage-vinyl-groove"></div>
          <div class="stage-artwork-wrapper">
            <div class="stage-artwork-placeholder">
              <span class="stage-vinyl-center">♪</span>
            </div>
            <img class="stage-artwork-img" src="" alt="Album Cover" style="display: none;" />
          </div>
        </div>

        <!-- Track Metadata & Identity -->
        <div class="stage-track-meta">
          <div class="stage-track-title" title="No track playing">Ready for Music</div>
          <div class="stage-track-artist">Select an album or track to start</div>
          <div class="stage-track-album"></div>
        </div>

        <!-- Stage Actions -->
        <div class="stage-actions-row">
          <button class="stage-action-btn stage-heart-btn" title="Save to Favorites" aria-label="Favorite">
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M20.84 4.61a5.5 5.5 0 0 0-7.78 0L12 5.67l-1.06-1.06a5.5 5.5 0 0 0-7.78 7.78l1.06 1.06L12 21.23l7.78-7.78 1.06-1.06a5.5 5.5 0 0 0 0-7.78z"/>
            </svg>
          </button>
          <button class="stage-action-btn stage-view-album-btn" title="View Album in Library" aria-label="View Album">
            <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor">
              <path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm0 14.5c-2.49 0-4.5-2.01-4.5-4.5S9.51 7.5 12 7.5s4.5 2.01 4.5 4.5-2.01 4.5-4.5 4.5zm0-5.5c-.55 0-1 .45-1 1s.45 1 1 1 1-.45 1-1-.45-1-1-1z"/>
            </svg>
          </button>
          <button class="stage-action-btn stage-enrich-btn" title="Find Metadata (MusicBrainz)" aria-label="Find Metadata">
            <span style="font-size: 13px;">🏷️</span>
          </button>
          <button class="stage-action-btn stage-find-lyrics-btn" title="Find Lyrics (Cmd+L)" aria-label="Find Lyrics">
            <span style="font-size: 13px;">🎵</span>
          </button>
          <button class="stage-action-btn stage-lyrics-btn" title="Open Full Screen Lyrics (l)" aria-label="Lyrics">
            <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor">
              <path d="M12 3v10.55c-.59-.34-1.27-.55-2-.55-2.21 0-4 1.79-4 4s1.79 4 4 4 4-1.79 4-4V7h4V3h-6z"/>
            </svg>
          </button>
          <button class="stage-action-btn stage-vis-btn" title="Toggle Spectrum Visualizer (v)" aria-label="Visualizer">
            <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor">
              <path d="M4 9h2v6H4zm4-4h2v14H8zm4 7h2v7h-2zm4-6h2v13h-2zm4 3h2v10h-2z"/>
            </svg>
          </button>
        </div>

        <!-- Mini Contextual Lyrics Box -->
        <div class="stage-mini-lyrics" id="stage-mini-lyrics-box">
          <div class="mini-lyrics-header">
            <span class="mini-lyrics-title">LYRICS</span>
            <div style="display: flex; gap: var(--space-2);">
              <button class="mini-lyrics-find-btn text-button" title="Find Lyrics Online (Cmd+L)">Find</button>
              <button class="mini-lyrics-expand-btn text-button" title="Expand Full Lyrics">Expand</button>
            </div>
          </div>
          <div class="mini-lyrics-body">
            <p class="mini-lyric-active" id="mini-lyric-active-line">Sonora High-Fidelity Audio</p>
            <p class="mini-lyric-next" id="mini-lyric-next-line">Play a track to view synchronized lyrics</p>
          </div>
        </div>

        <!-- Up Next Queue Preview Card -->
        <div class="stage-up-next-card" id="stage-up-next-box" style="display: none;">
          <div class="up-next-header">
            <span class="up-next-label">UP NEXT</span>
            <button class="up-next-queue-btn text-button" title="Open full queue">View Queue</button>
          </div>
          <div class="up-next-track-info">
            <div class="up-next-title" id="up-next-title">Next Track</div>
            <div class="up-next-artist" id="up-next-artist">Next Artist</div>
          </div>
        </div>
      </div>
    `;

    this.attachEventListeners();
  }

  private attachEventListeners() {
    const heartBtn = this.container.querySelector('.stage-heart-btn') as HTMLButtonElement;
    const viewAlbumBtn = this.container.querySelector('.stage-view-album-btn') as HTMLButtonElement;
    const enrichBtn = this.container.querySelector('.stage-enrich-btn') as HTMLButtonElement;
    const findLyricsBtn = this.container.querySelector('.stage-find-lyrics-btn') as HTMLButtonElement;
    const miniFindBtn = this.container.querySelector('.mini-lyrics-find-btn') as HTMLButtonElement;
    const lyricsBtn = this.container.querySelector('.stage-lyrics-btn') as HTMLButtonElement;
    const visBtn = this.container.querySelector('.stage-vis-btn') as HTMLButtonElement;
    const expandLyricsBtn = this.container.querySelector('.mini-lyrics-expand-btn') as HTMLButtonElement;
    const viewQueueBtn = this.container.querySelector('.up-next-queue-btn') as HTMLButtonElement;

    heartBtn?.addEventListener('click', () => {
      heartBtn.classList.toggle('active');
    });

    viewAlbumBtn?.addEventListener('click', () => {
      const track = appState.getStatus().current_track;
      if (track?.album) {
        appState.setActiveView({
          type: 'search',
          query: track.album,
        });
      }
    });

    enrichBtn?.addEventListener('click', () => {
      const track = appState.getStatus().current_track;
      if (track && track.track_id) {
        appState.openMatchInspector({
          trackId: track.track_id,
          title: track.title,
          artistName: track.artist,
          albumTitle: track.album,
          durationMs: track.duration_ms,
        });
      }
    });

    const triggerFindLyrics = () => {
      const track = appState.getStatus().current_track;
      if (track) {
        appState.openLyricsManager({
          trackId: track.track_id,
          filePath: track.file_path,
          title: track.title,
          artist: track.artist,
          album: track.album,
          durationMs: track.duration_ms,
        });
      }
    };

    findLyricsBtn?.addEventListener('click', triggerFindLyrics);
    miniFindBtn?.addEventListener('click', triggerFindLyrics);

    lyricsBtn?.addEventListener('click', () => {
      window.dispatchEvent(new CustomEvent('sonora-toggle-lyrics'));
    });

    expandLyricsBtn?.addEventListener('click', () => {
      window.dispatchEvent(new CustomEvent('sonora-toggle-lyrics'));
    });

    visBtn?.addEventListener('click', () => {
      appState.toggleVisualizer();
    });

    viewQueueBtn?.addEventListener('click', () => {
      appState.toggleQueue(true);
    });
  }

  private async update() {
    const status = appState.getStatus();
    const track = status.current_track;

    const titleEl = this.container.querySelector('.stage-track-title') as HTMLElement;
    const artistEl = this.container.querySelector('.stage-track-artist') as HTMLElement;
    const albumEl = this.container.querySelector('.stage-track-album') as HTMLElement;
    const badgesContainer = this.container.querySelector('#stage-stream-badges') as HTMLElement;
    const imgThumb = this.container.querySelector('.stage-artwork-img') as HTMLImageElement;
    const placeholderThumb = this.container.querySelector('.stage-artwork-placeholder') as HTMLElement;
    const upNextBox = this.container.querySelector('#stage-up-next-box') as HTMLElement;
    const upNextTitle = this.container.querySelector('#up-next-title') as HTMLElement;
    const upNextArtist = this.container.querySelector('#up-next-artist') as HTMLElement;

    if (!titleEl || !artistEl) return;

    if (track) {
      titleEl.textContent = track.title || 'Untitled Track';
      titleEl.setAttribute('title', track.title || '');
      artistEl.textContent = track.artist || 'Unknown Artist';
      if (albumEl) {
        albumEl.textContent = track.album || '';
      }

      // Format badges
      if (badgesContainer) {
        const ext = (track.file_path || '').split('.').pop()?.toUpperCase() || 'AUDIO';
        const isLossless = ['FLAC', 'WAV', 'AIFF', 'ALAC'].includes(ext);

        badgesContainer.innerHTML = `
          <span class="audio-badge badge-format ${isLossless ? 'badge-hires' : ''}">${escapeHtml(ext)}</span>
          <span class="audio-badge badge-rate">${isLossless ? 'Lossless' : 'Compressed'}</span>
          <span class="audio-badge badge-depth">Bit-Perfect</span>
        `;
      }

      // Artwork
      if (track.track_id) {
        const art = await appState.fetchArtwork(track.track_id);
        if (art && imgThumb && placeholderThumb) {
          imgThumb.src = art;
          imgThumb.style.display = 'block';
          placeholderThumb.style.display = 'none';
          this.extractAmbientColors(art);
        } else if (imgThumb && placeholderThumb) {
          imgThumb.style.display = 'none';
          placeholderThumb.style.display = 'flex';
          this.resetAmbientColors();
        }
      } else {
        this.resetAmbientColors();
      }

      // Load lyrics if track changed
      const trackKey = `${track.title}::${track.artist}`;
      if (trackKey !== this.currentTrackKey) {
        this.currentTrackKey = trackKey;
        this.fetchLyrics(track);
      }
    } else {
      this.resetAmbientColors();
      titleEl.textContent = 'Sonora Music Player';
      titleEl.removeAttribute('title');
      artistEl.textContent = 'Select an album or track to begin playback';
      if (albumEl) albumEl.textContent = '';
      if (badgesContainer) {
        badgesContainer.innerHTML = `<span class="audio-badge badge-format">Standby</span>`;
      }
      if (imgThumb && placeholderThumb) {
        imgThumb.style.display = 'none';
        placeholderThumb.style.display = 'flex';
      }
      this.currentTrackKey = null;
      this.lyricsDoc = null;
      this.updateLyricsText('Drop the needle on a record', 'Browse your albums or search your collection');
    }

    // Up next track preview
    const queue = appState.getQueue();
    const currentIndex = status.current_queue_index;
    if (queue.length > 0 && typeof currentIndex === 'number' && currentIndex < queue.length - 1) {
      const nextTrack = queue[currentIndex + 1];
      if (upNextBox && upNextTitle && upNextArtist && nextTrack) {
        upNextTitle.textContent = nextTrack.title || 'Next Track';
        upNextArtist.textContent = nextTrack.artist || 'Artist';
        upNextBox.style.display = 'flex';
      }
    } else if (upNextBox) {
      upNextBox.style.display = 'none';
    }

    this.updateLyrics();
  }

  private async fetchLyrics(track: any) {
    try {
      const doc = await api.getLyrics(track.track_id, track.file_path, track.title || '', track.artist, track.album, track.duration_ms);
      this.lyricsDoc = doc;
      this.updateLyrics();
    } catch {
      this.lyricsDoc = null;
      this.updateLyricsText('Instrumental / No Lyrics Found', 'Enjoy the high-fidelity acoustic performance');
    }
  }

  private updateLyrics() {
    if (!this.lyricsDoc || !this.lyricsDoc.lines || this.lyricsDoc.lines.length === 0) {
      return;
    }

    const status = appState.getStatus();
    const prefs = lyricsManager.getPreferences();
    const effectivePosMs = status.position_ms + prefs.manualOffsetMs;

    const lines = this.lyricsDoc.lines;
    let activeIdx = -1;
    for (let i = 0; i < lines.length; i++) {
      if (lines[i].start_time_ms <= effectivePosMs) {
        activeIdx = i;
      } else {
        break;
      }
    }

    if (activeIdx >= 0) {
      const current = lines[activeIdx]?.text || '';
      const next = lines[activeIdx + 1]?.text || '♪ ♪ ♪';
      this.updateLyricsText(current, next);
    } else {
      const first = lines[0]?.text || '';
      this.updateLyricsText('♪ ♪ ♪', first);
    }
  }

  private updateLyricsText(active: string, next: string) {
    const activeEl = this.container.querySelector('#mini-lyric-active-line') as HTMLElement;
    const nextEl = this.container.querySelector('#mini-lyric-next-line') as HTMLElement;
    if (activeEl && active !== this.currentLyricLine) {
      this.currentLyricLine = active;
      activeEl.textContent = active;
      activeEl.classList.remove('pulse-glow');
      void activeEl.offsetWidth; // trigger reflow
      activeEl.classList.add('pulse-glow');
    }
    if (nextEl && next !== this.nextLyricLine) {
      this.nextLyricLine = next;
      nextEl.textContent = next;
    }
  }

  private extractAmbientColors(imgSrc: string) {
    const img = new Image();
    img.crossOrigin = 'Anonymous';
    img.onload = () => {
      try {
        const canvas = document.createElement('canvas');
        canvas.width = 16;
        canvas.height = 16;
        const ctx = canvas.getContext('2d');
        if (!ctx) return;
        ctx.drawImage(img, 0, 0, 16, 16);
        const imgData = ctx.getImageData(0, 0, 16, 16).data;
        let r1 = 0, g1 = 0, b1 = 0, count1 = 0;
        let r2 = 0, g2 = 0, b2 = 0, count2 = 0;
        for (let i = 0; i < imgData.length; i += 4) {
          const r = imgData[i];
          const g = imgData[i + 1];
          const b = imgData[i + 2];
          const a = imgData[i + 3];
          if (a < 128) continue;
          if (i < imgData.length / 2) {
            r1 += r; g1 += g; b1 += b; count1++;
          } else {
            r2 += r; g2 += g; b2 += b; count2++;
          }
        }
        if (count1 > 0 && count2 > 0) {
          const c1R = Math.round(r1 / count1);
          const c1G = Math.round(g1 / count1);
          const c1B = Math.round(b1 / count1);
          const c2R = Math.round(r2 / count2);
          const c2G = Math.round(g2 / count2);
          const c2B = Math.round(b2 / count2);
          document.documentElement.style.setProperty('--ambient-color-1', `rgba(${c1R}, ${c1G}, ${c1B}, 0.55)`);
          document.documentElement.style.setProperty('--ambient-color-2', `rgba(${c2R}, ${c2G}, ${c2B}, 0.4)`);
        }
      } catch {
        this.resetAmbientColors();
      }
    };
    img.onerror = () => {
      this.resetAmbientColors();
    };
    img.src = imgSrc;
  }

  private resetAmbientColors() {
    document.documentElement.style.setProperty('--ambient-color-1', 'rgba(99, 102, 241, 0.25)');
    document.documentElement.style.setProperty('--ambient-color-2', 'rgba(168, 85, 247, 0.2)');
  }
}
