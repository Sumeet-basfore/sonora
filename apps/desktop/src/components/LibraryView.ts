import { api } from '../api';
import { appState } from '../state';
import { layoutManager } from '../customization';
import { AlbumDetailComponent } from './AlbumDetail';
import { escapeHtml } from '../escape';
import type { AlbumDto, SearchResult } from '../types';

function formatDuration(ms: number): string {
  if (!ms || isNaN(ms)) return '0:00';
  const totalSeconds = Math.floor(ms / 1000);
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  return `${minutes}:${seconds.toString().padStart(2, '0')}`;
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

export class LibraryViewComponent {
  private container: HTMLElement;
  private albumDetail: AlbumDetailComponent;
  private searchSequenceId: number = 0;
  private lastRenderedViewKey: string = '';
  private currentRenderId: number = 0;

  constructor(container: HTMLElement) {
    this.container = container;
    this.albumDetail = new AlbumDetailComponent(container);
    appState.subscribe(() => {
      this.renderCurrentView();
      this.updatePlayingTrackHighlight();
    });
    layoutManager.subscribe(() => {
      this.renderCurrentView(true);
    });
    window.addEventListener('sonora-library-updated', () => this.renderCurrentView(true));
    this.renderCurrentView();
  }


  public async renderCurrentView(force: boolean = false) {
    const activeView = appState.getActiveView();
    const activeLayoutId = layoutManager.getActiveLayout().id;
    const viewKey = JSON.stringify(activeView) + '::' + activeLayoutId;

    if (!force && viewKey === this.lastRenderedViewKey) {
      return;
    }
    this.lastRenderedViewKey = viewKey;
    const renderId = ++this.currentRenderId;

    if (activeView.type === 'marketplace') {
      // Owned by MarketplaceViewComponent; leave the container alone.
      return;
    }

    if (activeView.type === 'album_detail') {
      await this.albumDetail.loadAlbum(
        activeView.albumId,
        activeView.albumTitle,
        activeView.artistName
      );
      return;
    }

    this.renderLoading();

    try {
      if (activeView.type === 'albums') {
        await this.renderAlbums(renderId);
      } else if (activeView.type === 'artists') {
        await this.renderArtists(renderId);
      } else if (activeView.type === 'tracks') {
        await this.renderTracks(renderId);
      } else if (activeView.type === 'search') {
        await this.renderSearch(activeView.query, renderId);
      } else if (activeView.type === 'artist_detail') {
        await this.renderArtistDetail(activeView.artistId, activeView.artistName, renderId);
      }
    } catch (err: any) {
      if (this.currentRenderId === renderId) {
        this.renderError(err?.message || String(err));
      }
    }
  }

  private renderLoading() {
    this.container.innerHTML = `
      <div class="view-loading-state">
        <div class="spinner"></div>
        <p>Loading library content...</p>
      </div>
    `;
  }

  private renderError(message: string) {
    this.container.innerHTML = `
      <div class="view-error-state">
        <div class="error-icon">⚠️</div>
        <h3>Failed to load library</h3>
        <p class="error-message">${message}</p>
        <button class="button button-secondary retry-btn">Retry</button>
      </div>
    `;

    const retryBtn = this.container.querySelector('.retry-btn');
    retryBtn?.addEventListener('click', () => this.renderCurrentView());
  }

  private renderEmptyLibrary() {
    this.container.innerHTML = `
      <div class="empty-library-state">
        <div class="empty-icon-art">
          <svg width="64" height="64" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
            <path d="M9 18V5l12-2v13"/>
            <circle cx="6" cy="18" r="3"/>
            <circle cx="18" cy="16" r="3"/>
          </svg>
        </div>
        <h2 class="empty-title">Your Sonora Library is Empty</h2>
        <p class="empty-subtitle">
          Point Sonora to your local music directory to index your lossless and high-res audio files.
        </p>
        <button class="button button-primary empty-scan-trigger">
          <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor">
            <path d="M10 4H4c-1.1 0-1.99.9-1.99 2L2 18c0 1.1.9 2 2 2h16c1.1 0 2-.9 2-2V8c0-1.1-.9-2-2-2h-8l-2-2z"/>
          </svg>
          <span>Scan Music Directory</span>
        </button>
      </div>
    `;

    const scanBtn = this.container.querySelector('.empty-scan-trigger');
    scanBtn?.addEventListener('click', async () => {
      try {
        const folder = await api.pickDirectory();
        if (folder) {
          appState.toggleScanModal(true);
          const input = document.querySelector('#scan-path-input') as HTMLInputElement;
          if (input) input.value = folder;
          return;
        }
      } catch {}
      appState.toggleScanModal(true);
    });
  }

  // --- 1. Albums View ---
  private async renderAlbums(renderId: number) {
    const albums = await api.getAllAlbums();
    if (this.currentRenderId !== renderId) return;

    if (albums.length === 0) {
      this.renderEmptyLibrary();
      return;
    }

    // Small-library composition for 1-3 albums: Listening-focused spotlight composition
    if (albums.length <= 3) {
      await this.renderSpotlightLibrary(albums, renderId);
      return;
    }

    this.container.innerHTML = `
      <div class="view-header">
        <h2 class="view-title">Albums</h2>
        <span class="view-badge">${albums.length} releases</span>
      </div>
      <div class="album-grid">
        ${albums
          .map(
            (album) => `
            <div class="album-card" data-album-id="${album.id}" data-album-title="${escapeHtml(album.title)}" data-artist-name="${escapeHtml(album.artist_name || '')}">
              <div class="album-cover-wrapper">
                <div class="album-cover-placeholder">
                  <span class="cover-initials">${escapeHtml(album.title.substring(0, 2).toUpperCase())}</span>
                </div>
                <button class="card-play-btn" title="Play ${escapeHtml(album.title)}" aria-label="Play album ${escapeHtml(album.title)}">▶</button>
              </div>
              <div class="album-card-info">
                <div class="album-card-title" title="${escapeHtml(album.title)}">${escapeHtml(album.title)}</div>
                <div class="album-card-artist">${escapeHtml(album.artist_name || 'Unknown Artist')}</div>
                <div class="album-card-meta">${album.release_year ? album.release_year + ' • ' : ''}${album.track_count} tracks</div>
              </div>
            </div>
          `
          )
          .join('')}
      </div>
    `;

    this.attachAlbumCardListeners();
    this.loadAlbumThumbnails();
  }

  private async renderSpotlightLibrary(albums: AlbumDto[], renderId: number) {
    const primaryAlbum = albums[0];
    let previewTracks: SearchResult[] = [];
    try {
      previewTracks = await api.getAlbumTracks(primaryAlbum.id);
    } catch {}

    if (this.currentRenderId !== renderId) return;

    const otherAlbums = albums.slice(1);

    this.container.innerHTML = `
      <div class="curator-spotlight-container">
        <!-- Spotlight Hero -->
        <div class="curator-spotlight-hero" data-album-id="${primaryAlbum.id}">
          <div class="spotlight-badge-row">
            <span class="spotlight-curator-tag">LISTENING SPOTLIGHT</span>
            <span class="view-badge">${albums.length} ${albums.length === 1 ? 'album' : 'albums'} in library</span>
          </div>

          <div class="spotlight-card-content">
            <div class="spotlight-art-column">
              <div class="spotlight-art-card album-card" data-album-id="${primaryAlbum.id}" data-album-title="${escapeHtml(primaryAlbum.title)}" data-artist-name="${escapeHtml(primaryAlbum.artist_name || '')}">
                <div class="album-cover-wrapper spotlight-art-wrapper">
                  <div class="album-cover-placeholder">
                    <span class="cover-initials">${escapeHtml(primaryAlbum.title.substring(0, 2).toUpperCase())}</span>
                  </div>
                  <button class="card-play-btn spotlight-play-overlay" title="Play ${escapeHtml(primaryAlbum.title)}" aria-label="Play ${escapeHtml(primaryAlbum.title)}">▶</button>
                </div>
              </div>
            </div>

            <div class="spotlight-meta-column">
              <span class="spotlight-meta-eyebrow">FEATURED RECORDING</span>
              <h1 class="spotlight-title" title="${escapeHtml(primaryAlbum.title)}">${escapeHtml(primaryAlbum.title)}</h1>
              <div class="spotlight-artist">${escapeHtml(primaryAlbum.artist_name || 'Unknown Artist')}</div>
              <div class="spotlight-details">
                ${primaryAlbum.release_year ? `<span>${primaryAlbum.release_year}</span><span class="spotlight-dot">•</span>` : ''}
                <span>${primaryAlbum.track_count} Tracks</span>
                <span class="spotlight-dot">•</span>
                <span class="audio-badge badge-hires">Lossless Audio</span>
              </div>

              <div class="spotlight-action-row">
                <button class="button button-primary spotlight-play-album-btn" data-album-id="${primaryAlbum.id}">
                  <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor">
                    <path d="M8 5v14l11-7z"/>
                  </svg>
                  <span>Play Album</span>
                </button>
                <button class="button button-secondary spotlight-queue-album-btn" data-album-id="${primaryAlbum.id}">
                  <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor">
                    <path d="M19 13h-6v6h-2v-6H5v-2h6V5h2v6h6v2z"/>
                  </svg>
                  <span>Queue All</span>
                </button>
                <button class="button button-secondary spotlight-view-btn" data-album-id="${primaryAlbum.id}" data-album-title="${escapeHtml(primaryAlbum.title)}" data-artist-name="${escapeHtml(primaryAlbum.artist_name || '')}">
                  <span>View Details</span>
                </button>
              </div>

              <!-- Quick Track Preview -->
              ${
                previewTracks.length > 0
                  ? `
                <div class="spotlight-preview-tracklist">
                  <div class="spotlight-tracklist-title">TRACK PREVIEW</div>
                  <div class="spotlight-tracks-list">
                    ${previewTracks
                      .slice(0, 5)
                      .map(
                        (t, idx) => `
                      <div class="spotlight-track-item" data-track-id="${t.track_id}">
                        <span class="spotlight-track-idx">${idx + 1}</span>
                        <span class="spotlight-track-name" title="${escapeHtml(t.title)}">${escapeHtml(t.title)}</span>
                        <span class="spotlight-track-time">${formatDuration(t.duration_ms)}</span>
                        <button class="spotlight-track-play-btn" title="Play ${escapeHtml(t.title)}">▶</button>
                      </div>
                    `
                      )
                      .join('')}
                  </div>
                </div>
              `
                  : ''
              }
            </div>
          </div>
        </div>

        <!-- Other Albums in Small Library -->
        ${
          otherAlbums.length > 0
            ? `
          <div class="spotlight-other-section">
            <h3 class="spotlight-section-title">Other Releases in Library</h3>
            <div class="album-grid">
              ${otherAlbums
                .map(
                  (album) => `
                <div class="album-card" data-album-id="${album.id}" data-album-title="${escapeHtml(album.title)}" data-artist-name="${escapeHtml(album.artist_name || '')}">
                  <div class="album-cover-wrapper">
                    <div class="album-cover-placeholder">
                      <span class="cover-initials">${escapeHtml(album.title.substring(0, 2).toUpperCase())}</span>
                    </div>
                    <button class="card-play-btn" title="Play ${escapeHtml(album.title)}" aria-label="Play album ${escapeHtml(album.title)}">▶</button>
                  </div>
                  <div class="album-card-info">
                    <div class="album-card-title" title="${escapeHtml(album.title)}">${escapeHtml(album.title)}</div>
                    <div class="album-card-artist">${escapeHtml(album.artist_name || 'Unknown Artist')}</div>
                    <div class="album-card-meta">${album.release_year ? album.release_year + ' • ' : ''}${album.track_count} tracks</div>
                  </div>
                </div>
              `
                )
                .join('')}
            </div>
          </div>
        `
            : ''
        }
      </div>
    `;

    this.attachSpotlightListeners(primaryAlbum, previewTracks);
    this.attachAlbumCardListeners();
    this.loadAlbumThumbnails();
  }

  private attachSpotlightListeners(primaryAlbum: AlbumDto, previewTracks: SearchResult[]) {
    const playAlbumBtn = this.container.querySelector('.spotlight-play-album-btn');
    const queueAlbumBtn = this.container.querySelector('.spotlight-queue-album-btn');
    const viewBtn = this.container.querySelector('.spotlight-view-btn');

    playAlbumBtn?.addEventListener('click', () => {
      appState.playAlbum(primaryAlbum.id);
    });

    queueAlbumBtn?.addEventListener('click', async () => {
      for (const t of previewTracks) {
        await api.enqueueTrack(t.track_id);
      }
      await appState.refresh();
    });

    viewBtn?.addEventListener('click', () => {
      appState.setActiveView({
        type: 'album_detail',
        albumId: primaryAlbum.id,
        albumTitle: primaryAlbum.title,
        artistName: primaryAlbum.artist_name || undefined,
      });
    });

    const trackItems = this.container.querySelectorAll('.spotlight-track-item');
    trackItems.forEach((item) => {
      const trackId = parseInt(item.getAttribute('data-track-id') || '0', 10);
      const playBtn = item.querySelector('.spotlight-track-play-btn');
      const trigger = () => appState.playTrack(trackId);
      item.addEventListener('dblclick', trigger);
      playBtn?.addEventListener('click', (e) => {
        e.stopPropagation();
        trigger();
      });
    });
  }

  private async loadAlbumThumbnails() {
    const cards = this.container.querySelectorAll('.album-card');
    cards.forEach(async (card) => {
      const albumId = parseInt(card.getAttribute('data-album-id') || '0', 10);
      if (!albumId) return;
      try {
        const thumbUrl = await appState.fetchAlbumArtwork(albumId, true);
        if (thumbUrl) {
          const wrapper = card.querySelector('.album-cover-wrapper');
          const placeholder = card.querySelector('.album-cover-placeholder');
          if (wrapper && placeholder) {
            const img = document.createElement('img');
            img.className = 'album-cover-img';
            img.src = thumbUrl;
            img.alt = card.getAttribute('data-album-title') || 'Album Cover';
            img.loading = 'lazy';
            placeholder.replaceWith(img);
          }
        }
      } catch {}
    });
  }

  private attachAlbumCardListeners() {
    const cards = this.container.querySelectorAll('.album-card');
    cards.forEach((card) => {
      const albumId = parseInt(card.getAttribute('data-album-id') || '0', 10);
      const title = card.getAttribute('data-album-title') || '';
      const artist = card.getAttribute('data-artist-name') || '';

      card.addEventListener('click', () => {
        appState.setActiveView({
          type: 'album_detail',
          albumId,
          albumTitle: title,
          artistName: artist,
        });
      });

      const playBtn = card.querySelector('.card-play-btn');
      playBtn?.addEventListener('click', (e) => {
        e.stopPropagation();
        appState.playAlbum(albumId);
      });
    });
  }

  // --- 2. Artists View ---
  private async renderArtists(renderId: number) {
    const artists = await api.getAllArtists();
    if (this.currentRenderId !== renderId) return;

    if (artists.length === 0) {
      this.renderEmptyLibrary();
      return;
    }

    this.container.innerHTML = `
      <div class="view-header">
        <h2 class="view-title">Artists</h2>
        <span class="view-badge">${artists.length} artists</span>
      </div>
      <div class="artist-grid">
        ${artists
          .map(
            (artist) => `
            <div class="artist-card" data-artist-id="${artist.id}" data-artist-name="${escapeHtml(artist.name)}">
              <div class="artist-avatar-wrapper">
                <div class="artist-avatar-placeholder">
                  <span>${escapeHtml(artist.name.substring(0, 2).toUpperCase())}</span>
                </div>
              </div>
              <div class="artist-card-info">
                <div class="artist-card-name" title="${escapeHtml(artist.name)}">${escapeHtml(artist.name)}</div>
                <div class="artist-card-stats">${artist.album_count} albums • ${artist.track_count} tracks</div>
              </div>
            </div>
          `
          )
          .join('')}
      </div>
    `;

    const cards = this.container.querySelectorAll('.artist-card');
    cards.forEach((card) => {
      const artistId = parseInt(card.getAttribute('data-artist-id') || '0', 10);
      const name = card.getAttribute('data-artist-name') || '';
      card.addEventListener('click', () => {
        appState.setActiveView({ type: 'artist_detail', artistId, artistName: name });
      });
    });
  }

  // --- 3. Tracks View ---
  private async renderTracks(renderId: number) {
    const tracks = await api.getAllTracks(500);
    if (this.currentRenderId !== renderId) return;

    if (tracks.length === 0) {
      this.renderEmptyLibrary();
      return;
    }

    const isAudiophile = layoutManager.getActiveLayout().id === 'layout-audiophile-deck';

    this.container.innerHTML = `
      <div class="view-header">
        <h2 class="view-title">All Tracks</h2>
        <span class="view-badge">${tracks.length} songs ${isAudiophile ? '• Audiophile Studio' : ''}</span>
      </div>
      <div class="tracks-table-container ${isAudiophile ? 'audiophile-table' : ''}">
        <div class="tracks-table-header">
          <span class="col-num">#</span>
          <span class="col-title">TITLE</span>
          <span class="col-artist">ARTIST</span>
          <span class="col-album">ALBUM</span>
          ${isAudiophile ? '<span class="col-format">FORMAT</span><span class="col-sampling">RATE / DEPTH</span><span class="col-replaygain">REPLAYGAIN</span>' : ''}
          <span class="col-duration">TIME</span>
          <span class="col-actions"></span>
        </div>
        <div class="tracks-table-body">
          ${tracks
            .map(
              (t, idx) => {
                const meta = isAudiophile ? getTrackAudioMeta(t.file_path) : null;
                return `
              <div class="track-table-row" data-track-id="${t.track_id}">
                <div class="col-num">
                  <span class="row-num">${idx + 1}</span>
                  <button class="row-play-btn" title="Play" aria-label="Play ${escapeHtml(t.title)}">▶</button>
                </div>
                <div class="col-title">
                  <span class="row-title-text" title="${escapeHtml(t.title)}">${escapeHtml(t.title)}</span>
                </div>
                <div class="col-artist">${escapeHtml(t.artist_name || 'Unknown Artist')}</div>
                <div class="col-album">${escapeHtml(t.album_title || 'Unknown Album')}</div>
                ${isAudiophile && meta ? `
                  <div class="col-format"><span class="badge-format-pill">${escapeHtml(meta.format)}</span></div>
                  <div class="col-sampling">${escapeHtml(meta.rate)}</div>
                  <div class="col-replaygain">${escapeHtml(meta.replayGain)}</div>
                ` : ''}
                <div class="col-duration">${formatDuration(t.duration_ms)}</div>
                <div class="col-actions">
                  <button class="row-queue-btn" title="Add to queue" aria-label="Add ${escapeHtml(t.title)} to queue">＋</button>
                </div>
              </div>
            `;
              }
            )
            .join('')}
        </div>
      </div>
    `;

    this.attachTrackRowListeners();
    this.updatePlayingTrackHighlight();
  }

  private updatePlayingTrackHighlight() {
    const currentTrackId = appState.getStatus().current_track?.track_id;
    const isPlaying = appState.getStatus().state === 'Playing';
    const rows = this.container.querySelectorAll('.track-table-row');
    rows.forEach((row) => {
      const tid = parseInt(row.getAttribute('data-track-id') || '0', 10);
      if (currentTrackId && tid === currentTrackId && isPlaying) {
        row.classList.add('is-playing');
      } else {
        row.classList.remove('is-playing');
      }
    });
  }

  private attachTrackRowListeners() {

    const rows = this.container.querySelectorAll('.track-table-row');
    rows.forEach((row) => {
      const trackId = parseInt(row.getAttribute('data-track-id') || '0', 10);
      const playBtn = row.querySelector('.row-play-btn');
      const titleSpan = row.querySelector('.row-title-text');
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

  // --- 4. Search View ---
  private async renderSearch(query: string, renderId: number) {
    const currentSeq = ++this.searchSequenceId;
    const results = await api.searchLibrary(query, 100);

    // Stale search response protection against rapid repeated queries
    if (this.searchSequenceId !== currentSeq || this.currentRenderId !== renderId) return;

    if (results.length === 0) {
      this.container.innerHTML = `
        <div class="empty-search-state">
          <div class="search-empty-icon">🔍</div>
          <h3>No matches found for "${escapeHtml(query)}"</h3>
          <p>Check the spelling or try searching for a different track, artist, or album.</p>
        </div>
      `;
      return;
    }

    this.container.innerHTML = `
      <div class="view-header">
        <h2 class="view-title">Search Results</h2>
        <span class="view-badge">${results.length} found</span>
      </div>
      <div class="tracks-table-container">
        <div class="tracks-table-header">
          <span class="col-num">#</span>
          <span class="col-title">TITLE</span>
          <span class="col-artist">ARTIST</span>
          <span class="col-album">ALBUM</span>
          <span class="col-duration">TIME</span>
          <span class="col-actions"></span>
        </div>
        <div class="tracks-table-body">
          ${results
            .map(
              (t, idx) => `
              <div class="track-table-row" data-track-id="${t.track_id}">
                <div class="col-num">
                  <span class="row-num">${idx + 1}</span>
                  <button class="row-play-btn" title="Play" aria-label="Play ${escapeHtml(t.title)}">▶</button>
                </div>
                <div class="col-title">
                  <span class="row-title-text">${escapeHtml(t.title)}</span>
                </div>
                <div class="col-artist">${escapeHtml(t.artist_name || 'Unknown Artist')}</div>
                <div class="col-album">${escapeHtml(t.album_title || 'Unknown Album')}</div>
                <div class="col-duration">${formatDuration(t.duration_ms)}</div>
                <div class="col-actions">
                  <button class="row-queue-btn" title="Add to queue" aria-label="Add ${escapeHtml(t.title)} to queue">＋</button>
                </div>
              </div>
            `
            )
            .join('')}
        </div>
      </div>
    `;

    this.attachTrackRowListeners();
  }

  // --- 5. Artist Detail View ---
  private async renderArtistDetail(artistId: number, artistName: string, renderId: number) {
    const tracks = await api.getArtistTracks(artistId);
    if (this.currentRenderId !== renderId) return;

    this.container.innerHTML = `
      <div class="artist-detail-hero">
        <div class="artist-hero-avatar">
          <span>${escapeHtml(artistName.substring(0, 2).toUpperCase())}</span>
        </div>
        <div class="artist-hero-meta">
          <span class="meta-badge">ARTIST</span>
          <h1 class="artist-name-heading">${escapeHtml(artistName)}</h1>
          <div class="artist-meta-stats">${tracks.length} tracks in collection</div>
        </div>
      </div>

      <div class="tracks-table-container">
        <div class="tracks-table-header">
          <span class="col-num">#</span>
          <span class="col-title">TITLE</span>
          <span class="col-album">ALBUM</span>
          <span class="col-duration">TIME</span>
          <span class="col-actions"></span>
        </div>
        <div class="tracks-table-body">
          ${tracks
            .map(
              (t, idx) => `
              <div class="track-table-row" data-track-id="${t.track_id}">
                <div class="col-num">
                  <span class="row-num">${idx + 1}</span>
                  <button class="row-play-btn" title="Play" aria-label="Play ${escapeHtml(t.title)}">▶</button>
                </div>
                <div class="col-title">
                  <span class="row-title-text">${escapeHtml(t.title)}</span>
                </div>
                <div class="col-album">${escapeHtml(t.album_title || 'Unknown Album')}</div>
                <div class="col-duration">${formatDuration(t.duration_ms)}</div>
                <div class="col-actions">
                  <button class="row-queue-btn" title="Add to queue" aria-label="Add ${escapeHtml(t.title)} to queue">＋</button>
                </div>
              </div>
            `
            )
            .join('')}
        </div>
      </div>
    `;

    this.attachTrackRowListeners();
  }
}
