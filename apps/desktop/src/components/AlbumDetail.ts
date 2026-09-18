import { api } from '../api';
import { artworkStyleManager } from '../customization';
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
            </div>
          </div>
        </div>

        <!-- Track List Table -->
        <div class="album-tracklist-section">
          <div class="tracklist-header">
            <span class="col-num">#</span>
            <span class="col-title">TITLE</span>
            <span class="col-artist">ARTIST</span>
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
                        <div class="col-duration">${formatDuration(t.duration_ms)}</div>
                        <div class="col-actions">
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

    const trackRows = this.container.querySelectorAll('.track-row');
    trackRows.forEach((row) => {
      const trackId = parseInt(row.getAttribute('data-track-id') || '0', 10);
      const playBtn = row.querySelector('.row-play-btn');
      const titleSpan = row.querySelector('.row-track-title');
      const queueBtn = row.querySelector('.row-queue-btn');

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
    });
  }
}
