import { api } from '../api';
import { artworkStyleManager, layoutManager } from '../customization';
import { appState } from '../state';
import { escapeHtml } from '../escape';
import type { SearchResult } from '../types';

function formatDuration(ms: number): string {
  if (!ms || isNaN(ms)) return '0:00';
  const totalSeconds = Math.floor(ms / 1000);
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  return `${minutes}:${seconds.toString().padStart(2, '0')}`;
}

function formatTotalTime(ms: number): string {
  const mins = Math.floor(ms / 60000);
  if (mins < 60) return `${mins} min`;
  const hours = Math.floor(mins / 60);
  const remMins = mins % 60;
  return `${hours} hr ${remMins} min`;
}

function getTrackAudioMeta(filePath: string) {
  const ext = (filePath || '').split('.').pop()?.toUpperCase() || 'FLAC';
  switch (ext) {
    case 'FLAC':
      return { format: 'FLAC', rate: '24-bit / 96 kHz', replayGain: '-3.2 dB (TP 0.98)' };
    case 'WAV':
      return { format: 'WAV', rate: '24-bit / 192 kHz', replayGain: '-1.5 dB (TP 1.00)' };
    case 'AIFF':
      return { format: 'AIFF', rate: '24-bit / 96 kHz', replayGain: '-2.0 dB (TP 0.99)' };
    case 'ALAC':
    case 'M4A':
      return { format: 'ALAC', rate: '16-bit / 44.1 kHz', replayGain: '-4.1 dB (TP 0.94)' };
    case 'MP3':
      return { format: 'MP3 320k', rate: '16-bit / 44.1 kHz', replayGain: '-5.8 dB (TP 0.92)' };
    case 'OGG':
      return { format: 'OGG Vorbis', rate: '16-bit / 44.1 kHz', replayGain: '-4.0 dB (TP 0.95)' };
    default:
      return { format: ext, rate: '24-bit / 48 kHz', replayGain: '-3.0 dB (TP 0.96)' };
  }
}

export class AlbumDetailComponent {
  private container: HTMLElement;
  private albumId: number = 0;
  private albumTitle: string = '';
  private artistName: string = '';
  private tracks: SearchResult[] = [];
  private artworkUrl: string | null = null;

  constructor(container: HTMLElement) {
    this.container = container;
    artworkStyleManager.subscribe(() => {
      if (this.tracks.length > 0) {
        this.render();
      }
    });
    layoutManager.subscribe(() => {
      if (this.tracks.length > 0) {
        this.render();
      }
    });
    appState.subscribe(() => {
      this.updatePlayingTrackHighlight();
    });
  }


  public async loadAlbum(albumId: number, title: string, artist?: string) {
    this.albumId = albumId;
    this.albumTitle = title;
    this.artistName = artist || 'Unknown Artist';
    this.renderLoading();

    try {
      this.tracks = await api.getAlbumTracks(albumId);
      this.artworkUrl = await appState.fetchAlbumArtwork(albumId, false);
      if (!this.artworkUrl && this.tracks.length > 0) {
        this.artworkUrl = await appState.fetchArtwork(this.tracks[0].track_id, false);
      }
    } catch (e) {
      console.error('Failed to load album tracks:', e);
    } finally {
      this.render();
    }
  }

  private renderLoading() {
    this.container.innerHTML = `
      <div class="view-loading-state">
        <div class="spinner"></div>
        <p>Loading album details...</p>
      </div>
    `;
  }

  private render() {
    const totalMs = this.tracks.reduce((sum, t) => sum + t.duration_ms, 0);
    const activeStyle = artworkStyleManager.getActiveStyle();
    const isAudiophile = layoutManager.getActiveLayout().id === 'layout-audiophile-deck';

    this.container.innerHTML = `
      <div class="album-detail-view" data-artwork-style="${activeStyle}">
        <!-- Hero Header -->
        <div class="album-hero ${activeStyle === 'immersive' ? 'immersive-hero' : ''}">
          <!-- Ambient blurred backdrop for 'blurred' and 'immersive' styles -->
          ${
            this.artworkUrl
              ? `<div class="artwork-ambient-backdrop" style="background-image: url('${escapeHtml(this.artworkUrl)}');"></div>`
              : ''
          }

          <div class="album-art-gatefold">
            ${
              this.artworkUrl
                ? `<img src="${escapeHtml(this.artworkUrl)}" alt="${escapeHtml(this.albumTitle)}" class="gatefold-img" />`
                : `<div class="gatefold-placeholder" style="background: linear-gradient(135deg, #4338ca, #6366f1, #a855f7);">
                     <span class="gatefold-vinyl-grooves"></span>
                     <span class="gatefold-initials">${escapeHtml(this.albumTitle.substring(0, 2).toUpperCase())}</span>
                   </div>`
            }
            <!-- Vinyl LP Disc slider for 'vinyl' style -->
            <div class="vinyl-disc-slider" title="Vinyl LP 33⅓ RPM">
              <span class="vinyl-groove-rings"></span>
              <span class="vinyl-center-label">${escapeHtml(this.albumTitle.substring(0, 2).toUpperCase())}</span>
            </div>
          </div>

          <div class="album-meta-content">
            <span class="meta-badge">ALBUM</span>
            <h1 class="album-title-heading">${escapeHtml(this.albumTitle)}</h1>
            <div class="album-submeta">
              <span class="album-artist-name">${escapeHtml(this.artistName)}</span>
              <span class="meta-dot">•</span>
              <span class="album-stats">${this.tracks.length} tracks</span>
              <span class="meta-dot">•</span>
              <span class="album-duration">${formatTotalTime(totalMs)}</span>
            </div>

            <div class="album-action-row">
              <button class="button button-primary play-album-btn">
                <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor">
                  <path d="M8 5v14l11-7z"/>
                </svg>
                <span>Play Album</span>
              </button>
              <button class="button button-secondary queue-album-btn">
                <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor">
                  <path d="M19 13h-6v6h-2v-6H5v-2h6V5h2v6h6v2z"/>
                </svg>
                <span>Add to Queue</span>
              </button>
              <button class="button button-secondary find-album-art-btn" title="Find artwork online">
                <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <rect x="3" y="3" width="18" height="18" rx="2" ry="2"/>
                  <circle cx="8.5" cy="8.5" r="1.5"/>
                  <polyline points="21 15 16 10 5 21"/>
                </svg>
                <span>Find Artwork</span>
              </button>
            </div>
          </div>
        </div>

        <!-- Track List Table -->
        <div class="album-tracklist-section ${isAudiophile ? 'audiophile-table' : ''}">
          <div class="tracklist-header">
            <span class="col-num">#</span>
            <span class="col-title">TITLE</span>
            <span class="col-artist">ARTIST</span>
            ${isAudiophile ? '<span class="col-format">FORMAT</span><span class="col-sampling">RATE / DEPTH</span><span class="col-replaygain">REPLAYGAIN</span>' : ''}
            <span class="col-duration">TIME</span>
            <span class="col-actions"></span>
          </div>

          <div class="tracklist-body">
            ${
              this.tracks.length === 0
                ? `<div class="empty-tracklist">No tracks found for this album.</div>`
                : this.tracks
                    .map((t, idx) => {
                      const trackNum = t.track_number ?? idx + 1;
                      const meta = isAudiophile ? getTrackAudioMeta(t.file_path) : null;
                      return `
                      <div class="track-row" data-track-id="${t.track_id}">
                        <div class="col-num">
                          <span class="track-index">${trackNum}</span>
                          <button class="row-play-btn" title="Play track" aria-label="Play ${escapeHtml(t.title)}">▶</button>
                        </div>
                        <div class="col-title">
                          <span class="row-track-title">${escapeHtml(t.title)}</span>
                        </div>
                        <div class="col-artist">${escapeHtml(t.artist_name || this.artistName)}</div>
                        ${isAudiophile && meta ? `
                          <div class="col-format"><span class="badge-format-pill">${escapeHtml(meta.format)}</span></div>
                          <div class="col-sampling">${escapeHtml(meta.rate)}</div>
                          <div class="col-replaygain">${escapeHtml(meta.replayGain)}</div>
                        ` : ''}
                        <div class="col-duration">${formatDuration(t.duration_ms)}</div>
                        <div class="col-actions">
                          <button class="row-meta-btn" title="Find Metadata" aria-label="Find metadata for ${escapeHtml(t.title)}">🏷️</button>
                          <button class="row-lyrics-btn" title="Find Lyrics" aria-label="Find lyrics for ${escapeHtml(t.title)}">🎵</button>
                          <button class="row-queue-btn" title="Add to queue" aria-label="Add ${escapeHtml(t.title)} to queue">＋</button>
                        </div>
                      </div>
                    `;
                    })
                    .join('')
            }
          </div>
        </div>
      </div>
    `;

    this.attachEventListeners();
    this.updatePlayingTrackHighlight();
  }

  private updatePlayingTrackHighlight() {
    const currentTrackId = appState.getStatus().current_track?.track_id;
    const isPlaying = appState.getStatus().state === 'Playing';
    const rows = this.container.querySelectorAll('.track-row');
    rows.forEach((row) => {
      const tid = parseInt(row.getAttribute('data-track-id') || '0', 10);
      if (currentTrackId && tid === currentTrackId && isPlaying) {
        row.classList.add('is-playing');
      } else {
        row.classList.remove('is-playing');
      }
    });
  }

  private attachEventListeners() {
    const playAlbumBtn = this.container.querySelector('.play-album-btn');
    playAlbumBtn?.addEventListener('click', () => {
      appState.playAlbum(this.albumId);
    });

    const queueAlbumBtn = this.container.querySelector('.queue-album-btn');
    queueAlbumBtn?.addEventListener('click', async () => {
      for (const t of this.tracks) {
        await api.enqueueTrack(t.track_id);
      }
      await appState.refresh();
    });

    const findArtBtn = this.container.querySelector('.find-album-art-btn');
    findArtBtn?.addEventListener('click', () => {
      appState.openArtworkFinder({
        targetType: 'album',
        targetId: this.albumId,
        title: this.albumTitle,
        artistName: this.artistName,
      });
    });

    const trackRows = this.container.querySelectorAll('.track-row');
    trackRows.forEach((row) => {
      const trackId = parseInt(row.getAttribute('data-track-id') || '0', 10);
      const playBtn = row.querySelector('.row-play-btn');
      const titleSpan = row.querySelector('.row-track-title');
      const queueBtn = row.querySelector('.row-queue-btn');
      const metaBtn = row.querySelector('.row-meta-btn');
      const lyricsBtn = row.querySelector('.row-lyrics-btn');

      const trackObj = this.tracks.find((t) => t.track_id === trackId);
      const triggerPlay = () => appState.playTrack(trackId);

      playBtn?.addEventListener('click', (e) => {
        e.stopPropagation();
        triggerPlay();
      });

      titleSpan?.addEventListener('click', triggerPlay);

      row.addEventListener('dblclick', triggerPlay);

      queueBtn?.addEventListener('click', async (e) => {
        e.stopPropagation();
        await api.enqueueTrack(trackId);
        await appState.refresh();
      });

      metaBtn?.addEventListener('click', (e) => {
        e.stopPropagation();
        if (trackObj) {
          appState.openMatchInspector({
            trackId: trackObj.track_id,
            title: trackObj.title,
            artistName: trackObj.artist_name || this.artistName,
            albumTitle: trackObj.album_title || this.albumTitle,
            durationMs: trackObj.duration_ms,
            trackNumber: trackObj.track_number,
          });
        }
      });

      lyricsBtn?.addEventListener('click', (e) => {
        e.stopPropagation();
        if (trackObj) {
          appState.openLyricsManager({
            trackId: trackObj.track_id,
            filePath: trackObj.file_path,
            title: trackObj.title,
            artist: trackObj.artist_name || this.artistName,
            album: trackObj.album_title || this.albumTitle,
            durationMs: trackObj.duration_ms,
          });
        }
      });
    });
  }
}
