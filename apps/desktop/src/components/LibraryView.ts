import { api } from '../api';
import { appState } from '../state';
import { AlbumDetailComponent } from './AlbumDetail';
import { escapeHtml } from '../escape';

function formatDuration(ms: number): string {
  if (!ms || isNaN(ms)) return '0:00';
  const totalSeconds = Math.floor(ms / 1000);
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  return `${minutes}:${seconds.toString().padStart(2, '0')}`;
}

export class LibraryViewComponent {
  private container: HTMLElement;
  private albumDetail: AlbumDetailComponent;
  private searchSequenceId: number = 0;
  private lastRenderedViewKey: string = '';

  constructor(container: HTMLElement) {
    this.container = container;
    this.albumDetail = new AlbumDetailComponent(container);
    appState.subscribe(() => {
      this.renderCurrentView();
      this.updatePlayingTrackHighlight();
    });
    window.addEventListener('sonora-library-updated', () => this.renderCurrentView(true));
    this.renderCurrentView();
  }


  public async renderCurrentView(force: boolean = false) {
    const activeView = appState.getActiveView();
    const viewKey = JSON.stringify(activeView);

    if (!force && viewKey === this.lastRenderedViewKey) {
      return;
    }
    this.lastRenderedViewKey = viewKey;

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
        await this.renderAlbums();
      } else if (activeView.type === 'artists') {
        await this.renderArtists();
      } else if (activeView.type === 'tracks') {
        await this.renderTracks();
      } else if (activeView.type === 'search') {
        await this.renderSearch(activeView.query);
      } else if (activeView.type === 'artist_detail') {
        await this.renderArtistDetail(activeView.artistId, activeView.artistName);
      }
    } catch (err: any) {
      this.renderError(err?.message || String(err));
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
  private async renderAlbums() {
    const albums = await api.getAllAlbums();
    if (albums.length === 0) {
      this.renderEmptyLibrary();
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
  private async renderArtists() {
    const artists = await api.getAllArtists();
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
  private async renderTracks() {
    const tracks = await api.getAllTracks(500);
    if (tracks.length === 0) {
      this.renderEmptyLibrary();
      return;
    }

    this.container.innerHTML = `
      <div class="view-header">
        <h2 class="view-title">All Tracks</h2>
        <span class="view-badge">${tracks.length} songs</span>
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
          ${tracks
            .map(
              (t, idx) => `
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
  private async renderSearch(query: string) {
    const currentSeq = ++this.searchSequenceId;
    const results = await api.searchLibrary(query, 100);

    // Stale search response protection against rapid repeated queries
    if (this.searchSequenceId !== currentSeq) return;

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
  private async renderArtistDetail(artistId: number, artistName: string) {
    const tracks = await api.getArtistTracks(artistId);

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
