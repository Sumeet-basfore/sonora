import { api } from '../api.ts';
import { appState } from '../state.ts';
import { esc } from '../escape.ts';
import type {
  AlbumDto,
  ArtistDto,
  LyricsCandidate,
  OnlineArtist,
  OnlineReleaseGroup,
  OnlineTrack,
  SearchResult,
  SearchScope,
} from '../types.ts';

export class UniversalSearchComponent {
  private container: HTMLElement;
  private currentQuery: string = '';
  private currentScope: SearchScope = 'local';
  private debounceTimer: number | null = null;
  private requestId: number = 0;
  private selectedIndex: number = -1;

  // Cached results
  private localResults: {
    tracks: SearchResult[];
    albums: AlbumDto[];
    artists: ArtistDto[];
  } = { tracks: [], albums: [], artists: [] };

  private onlineResults: {
    recordings: OnlineTrack[];
    releaseGroups: OnlineReleaseGroup[];
    artists: OnlineArtist[];
  } = { recordings: [], releaseGroups: [], artists: [] };

  private lyricsResults: LyricsCandidate[] = [];

  // Status & error states
  private isLoading: boolean = false;
  private errorMessage: string | null = null;
  private isRateLimited: boolean = false;
  private isOffline: boolean = false;

  constructor(container: HTMLElement) {
    this.container = container;
    this.render();
    appState.subscribe(() => this.onStateChange());

    // Monitor online/offline events
    if (typeof window !== 'undefined') {
      window.addEventListener('online', () => {
        this.isOffline = false;
        if (appState.isUniversalSearchVisible()) this.executeSearch(true);
      });
      window.addEventListener('offline', () => {
        this.isOffline = true;
        if (appState.isUniversalSearchVisible()) this.render();
      });
    }
  }

  private onStateChange() {
    const isVisible = appState.isUniversalSearchVisible();
    const searchState = appState.getUniversalSearchState();

    if (!isVisible) {
      this.container.innerHTML = '';
      this.container.style.display = 'none';
      return;
    }

    this.container.style.display = 'flex';
    if (this.currentScope !== searchState.scope || this.currentQuery !== searchState.query) {
      this.currentScope = searchState.scope;
      this.currentQuery = searchState.query;
      this.render();
      if (this.currentQuery.trim()) {
        this.executeSearch(true);
      }
    } else if (!this.container.querySelector('.universal-search-dialog')) {
      this.render();
    }
  }

  public render() {
    const isVisible = appState.isUniversalSearchVisible();
    if (!isVisible) {
      this.container.style.display = 'none';
      return;
    }

    this.container.style.display = 'flex';
    this.container.innerHTML = `
      <div class="universal-search-backdrop">
        <div class="universal-search-dialog" role="dialog" aria-modal="true" aria-label="Universal Search">
          <!-- Search Header -->
          <div class="universal-search-header">
            <div class="search-input-row">
              <svg class="search-bar-icon" width="18" height="18" viewBox="0 0 24 24" fill="currentColor">
                <path d="M15.5 14h-.79l-.28-.27C15.41 12.59 16 11.11 16 9.5 16 5.91 13.09 3 9.5 3S3 5.91 3 9.5 5.91 16 9.5 16c1.61 0 3.09-.59 4.23-1.57l.27.28v.79l5 4.99L20.49 19l-4.99-5zm-6 0C7.01 14 5 11.99 5 9.5S7.01 5 9.5 5 14 7.01 14 9.5 11.99 14 9.5 14z"/>
              </svg>
              <input
                type="search"
                class="universal-search-input"
                placeholder="Search library, online metadata, lyrics..."
                value="${esc(this.currentQuery)}"
                aria-label="Universal Search Input"
                autofocus
              />
              ${
                this.currentQuery
                  ? `<button class="universal-search-clear-btn" title="Clear query" aria-label="Clear">✕</button>`
                  : ''
              }
              <kbd class="search-shortcut-kbd">ESC</kbd>
            </div>

            <!-- Mode Selector Tabs -->
            <div class="search-mode-tabs" role="tablist">
              <button class="mode-tab ${this.currentScope === 'local' ? 'active' : ''}" data-scope="local" role="tab">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor">
                  <path d="M12 3v10.55c-.59-.34-1.27-.55-2-.55-2.21 0-4 1.79-4 4s1.79 4 4 4 4-1.79 4-4V7h4V3h-6z"/>
                </svg>
                Local Library
              </button>
              <button class="mode-tab ${this.currentScope === 'online_metadata' ? 'active' : ''}" data-scope="online_metadata" role="tab">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor">
                  <path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm-1 17.93c-3.95-.49-7-3.85-7-7.93 0-.62.08-1.21.21-1.79L9 15v1c0 1.1.9 2 2 2v1.93zm6.9-2.54c-.26-.81-1-1.39-1.9-1.39h-1v-3c0-.55-.45-1-1-1H8v-2h2c.55 0 1-.45 1-1V7h2c1.1 0 2-.9 2-2v-.41c2.93 1.19 5 4.06 5 7.41 0 2.08-.8 3.97-2.1 5.39z"/>
                </svg>
                Online Metadata
              </button>
              <button class="mode-tab ${this.currentScope === 'lyrics' ? 'active' : ''}" data-scope="lyrics" role="tab">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor">
                  <path d="M15 4H5v16h14V8h-4V4zm-2 13.5c0 .83-.67 1.5-1.5 1.5S10 18.33 10 17.5s.67-1.5 1.5-1.5c.24 0 .46.06.66.16V13h3v2h-2.16c-.01.16-.04.33-.04.5z"/>
                </svg>
                Lyrics
              </button>
            </div>
          </div>

          <!-- Network & Rate Limit Banners -->
          ${
            this.isOffline
              ? `<div class="search-status-banner offline-banner">
                  <span class="status-icon">⚡</span>
                  <span>Network Unavailable — Showing Local Results Only</span>
                </div>`
              : ''
          }
          ${
            this.isRateLimited
              ? `<div class="search-status-banner rate-limit-banner">
                  <span class="status-icon">⏳</span>
                  <span>Provider Rate Limit Exceeded — Retrying shortly...</span>
                </div>`
              : ''
          }
          ${
            this.errorMessage
              ? `<div class="search-status-banner error-banner">
                  <span class="status-icon">⚠️</span>
                  <span>${esc(this.errorMessage)}</span>
                  <button class="retry-search-btn button-link">Retry</button>
                </div>`
              : ''
          }

          <!-- Results Scroll Area -->
          <div class="universal-search-results" id="search-results-viewport">
            ${this.renderResultsBody()}
          </div>

          <!-- Footer Shortcut Hints -->
          <div class="universal-search-footer">
            <div class="shortcut-hints">
              <span><kbd>↑</kbd><kbd>↓</kbd> Navigate</span>
              <span><kbd>↵</kbd> Select</span>
              <span><kbd>Tab</kbd> Switch Scope</span>
              <span><kbd>Esc</kbd> Close</span>
            </div>
            <div class="search-attribution">
              ${
                this.currentScope === 'online_metadata'
                  ? 'Powered by MusicBrainz & Cover Art Archive'
                  : this.currentScope === 'lyrics'
                  ? 'Powered by LRCLIB'
                  : 'Instant Local FTS5'
              }
            </div>
          </div>
        </div>
      </div>
    `;

    this.attachEventListeners();
    const input = this.container.querySelector('.universal-search-input') as HTMLInputElement;
    if (input) {
      input.focus();
      input.setSelectionRange(input.value.length, input.value.length);
    }
  }

  private renderResultsBody(): string {
    if (this.isLoading) {
      return `
        <div class="search-loading-state">
          <div class="skeleton-pulse-row"></div>
          <div class="skeleton-pulse-row"></div>
          <div class="skeleton-pulse-row"></div>
          <div class="skeleton-pulse-row"></div>
        </div>
      `;
    }

    if (!this.currentQuery.trim()) {
      return `
        <div class="search-empty-state">
          <div class="empty-icon">🔍</div>
          <h3>Type to search ${
            this.currentScope === 'local'
              ? 'your local library'
              : this.currentScope === 'online_metadata'
              ? 'MusicBrainz database'
              : 'synced & plain lyrics'
          }</h3>
          <p>Instant keyboard-driven discovery and non-destructive enrichment.</p>
        </div>
      `;
    }

    switch (this.currentScope) {
      case 'local':
        return this.renderLocalResults();
      case 'online_metadata':
        return this.renderOnlineMetadataResults();
      case 'lyrics':
        return this.renderLyricsResults();
    }
  }

  // --- Local Library Results ---
  private renderLocalResults(): string {
    const { tracks, albums, artists } = this.localResults;
    const hasAny = tracks.length > 0 || albums.length > 0 || artists.length > 0;

    if (!hasAny) {
      return `
        <div class="search-empty-state">
          <div class="empty-icon">📁</div>
          <h3>No local library matches found</h3>
          <p>Try switching to <strong>Online Metadata</strong> to find this release on MusicBrainz.</p>
          <button class="button button-secondary switch-to-online-btn">Search Online Metadata</button>
        </div>
      `;
    }

    let html = '';

    // Tracks Section
    if (tracks.length > 0) {
      html += `
        <div class="result-section">
          <div class="section-title">Tracks (${tracks.length})</div>
          <div class="result-list">
            ${tracks
              .map(
                (track, idx) => `
              <div class="result-row track-result ${this.selectedIndex === idx ? 'active-result' : ''}" data-type="local-track" data-id="${track.track_id}" data-index="${idx}">
                <div class="result-main">
                  <div class="result-title">${esc(track.title)}</div>
                  <div class="result-sub">${esc(track.artist_name || 'Unknown Artist')} — ${esc(track.album_title || 'Unknown Album')}</div>
                </div>
                <div class="result-meta">
                  <span class="duration-badge">${this.formatDuration(track.duration_ms)}</span>
                  <button class="button button-ghost play-track-btn" data-id="${track.track_id}" title="Play Track">▶</button>
                </div>
              </div>
            `
              )
              .join('')}
          </div>
        </div>
      `;
    }

    // Albums Section
    if (albums.length > 0) {
      html += `
        <div class="result-section">
          <div class="section-title">Albums (${albums.length})</div>
          <div class="result-list">
            ${albums
              .map(
                (album) => `
              <div class="result-row album-result" data-type="local-album" data-id="${album.id}" data-title="${esc(album.title)}" data-artist="${esc(album.artist_name || '')}">
                <div class="result-main">
                  <div class="result-title">${esc(album.title)}</div>
                  <div class="result-sub">${esc(album.artist_name || 'Unknown Artist')} ${album.release_year ? `(${album.release_year})` : ''}</div>
                </div>
                <div class="result-meta">
                  <button class="button button-ghost view-album-btn" data-id="${album.id}" data-title="${esc(album.title)}" data-artist="${esc(album.artist_name || '')}">Open</button>
                </div>
              </div>
            `
              )
              .join('')}
          </div>
        </div>
      `;
    }

    // Artists Section
    if (artists.length > 0) {
      html += `
        <div class="result-section">
          <div class="section-title">Artists (${artists.length})</div>
          <div class="result-list">
            ${artists
              .map(
                (artist) => `
              <div class="result-row artist-result" data-type="local-artist" data-id="${artist.id}" data-name="${esc(artist.name)}">
                <div class="result-main">
                  <div class="result-title">${esc(artist.name)}</div>
                  <div class="result-sub">${artist.track_count || 0} tracks</div>
                </div>
                <div class="result-meta">
                  <button class="button button-ghost view-artist-btn" data-id="${artist.id}">Open</button>
                </div>
              </div>
            `
              )
              .join('')}
          </div>
        </div>
      `;
    }

    return html;
  }

  // --- Online Metadata Results ---
  private renderOnlineMetadataResults(): string {
    const { recordings, releaseGroups, artists } = this.onlineResults;
    const hasAny = recordings.length > 0 || releaseGroups.length > 0 || artists.length > 0;

    if (!hasAny) {
      return `
        <div class="search-empty-state">
          <div class="empty-icon">🌐</div>
          <h3>No MusicBrainz matches found</h3>
          <p>Check your spelling or try searching by canonical artist / album name.</p>
        </div>
      `;
    }

    let html = '';

    // Recordings / Tracks
    if (recordings.length > 0) {
      html += `
        <div class="result-section">
          <div class="section-title">Recordings (${recordings.length})</div>
          <div class="result-list">
            ${recordings
              .map(
                (rec, idx) => `
              <div class="result-row online-track-result ${this.selectedIndex === idx ? 'active-result' : ''}" data-type="online-track" data-mbid="${rec.recording_mbid}" data-index="${idx}">
                <div class="result-main">
                  <div class="result-title-row">
                    <span class="result-title">${esc(rec.title)}</span>
                    <span class="badge badge-mbid">MBID</span>
                  </div>
                  <div class="result-sub">
                    ${esc(rec.artist_credits.map((c) => c.name + (c.join_phrase || '')).join(''))}
                    ${rec.duration_ms ? ` • ${this.formatDuration(rec.duration_ms)}` : ''}
                    ${rec.isrcs.length > 0 ? ` • ISRC: ${rec.isrcs[0]}` : ''}
                  </div>
                </div>
                <div class="result-actions">
                  <button class="button button-secondary inspect-online-btn" data-mbid="${rec.recording_mbid}" data-title="${esc(rec.title)}" data-artist="${esc(rec.artist_credits.map((c) => c.name).join(', '))}" title="Inspect with Match Inspector">
                    Inspect
                  </button>
                  <button class="button button-ghost find-lyrics-btn" data-title="${esc(rec.title)}" data-artist="${esc(rec.artist_credits.map((c) => c.name).join(', '))}" title="Find Lyrics">
                    Lyrics
                  </button>
                  <button class="button button-ghost find-art-btn" data-rel="${rec.release_mbid || ''}" data-rg="${rec.release_group_mbid || ''}" data-artist="${esc(rec.artist_credits.map((c) => c.name).join(', '))}" title="Find Artwork">
                    Artwork
                  </button>
                </div>
              </div>
            `
              )
              .join('')}
          </div>
        </div>
      `;
    }

    // Release Groups / Albums
    if (releaseGroups.length > 0) {
      html += `
        <div class="result-section">
          <div class="section-title">Release Groups / Albums (${releaseGroups.length})</div>
          <div class="result-list">
            ${releaseGroups
              .map(
                (rg) => `
              <div class="result-row online-album-result" data-type="online-album" data-mbid="${rg.mbid}">
                <div class="result-main">
                  <div class="result-title-row">
                    <span class="result-title">${esc(rg.title)}</span>
                    <span class="badge badge-type">${esc(rg.primary_type || 'Album')}</span>
                  </div>
                  <div class="result-sub">
                    ${esc(rg.artist_credits.map((c) => c.name + (c.join_phrase || '')).join(''))}
                    ${rg.first_release_date ? ` • Released: ${rg.first_release_date}` : ''}
                  </div>
                </div>
                <div class="result-actions">
                  <button class="button button-secondary find-art-btn" data-rg="${rg.mbid}" data-title="${esc(rg.title)}" data-artist="${esc(rg.artist_credits.map((c) => c.name).join(', '))}" title="Find Artwork">
                    Artwork
                  </button>
                </div>
              </div>
            `
              )
              .join('')}
          </div>
        </div>
      `;
    }

    // Artists
    if (artists.length > 0) {
      html += `
        <div class="result-section">
          <div class="section-title">Artists (${artists.length})</div>
          <div class="result-list">
            ${artists
              .map(
                (artist) => `
              <div class="result-row online-artist-result" data-type="online-artist" data-mbid="${artist.mbid}">
                <div class="result-main">
                  <div class="result-title-row">
                    <span class="result-title">${esc(artist.name)}</span>
                    ${artist.country ? `<span class="badge badge-country">${esc(artist.country)}</span>` : ''}
                  </div>
                  <div class="result-sub">
                    ${artist.disambiguation ? esc(artist.disambiguation) : 'MusicBrainz Artist'}
                  </div>
                </div>
                <div class="result-actions">
                  <button class="button button-secondary find-artist-art-btn" data-mbid="${artist.mbid}" data-name="${esc(artist.name)}" title="Find Artist Artwork">
                    Artwork
                  </button>
                </div>
              </div>
            `
              )
              .join('')}
          </div>
        </div>
      `;
    }

    return html;
  }

  // --- Lyrics Results ---
  private renderLyricsResults(): string {
    if (this.lyricsResults.length === 0) {
      return `
        <div class="search-empty-state">
          <div class="empty-icon">📜</div>
          <h3>No lyrics candidates found</h3>
          <p>Try refining your query with the track name and artist name.</p>
        </div>
      `;
    }

    return `
      <div class="result-section">
        <div class="section-title">Lyrics Candidates (${this.lyricsResults.length})</div>
        <div class="result-list">
          ${this.lyricsResults
            .map(
              (candidate, idx) => `
            <div class="result-row lyrics-result ${this.selectedIndex === idx ? 'active-result' : ''}" data-type="lyrics-candidate" data-id="${candidate.candidate_id}" data-index="${idx}">
              <div class="result-main">
                <div class="result-title-row">
                  <span class="result-title">${esc(candidate.track_name)}</span>
                  <span class="badge ${candidate.sync_type === 'LineSynced' || candidate.sync_type === 'SyllableSynced' ? 'badge-synced' : 'badge-plain'}">
                    ${candidate.sync_type === 'LineSynced' ? 'Synced (LRC)' : candidate.sync_type === 'SyllableSynced' ? 'Syllable' : 'Plain Text'}
                  </span>
                  <span class="badge badge-provider">${esc(candidate.provider_name)}</span>
                </div>
                <div class="result-sub">
                  ${esc(candidate.artist_name)} ${candidate.album_name ? `• ${esc(candidate.album_name)}` : ''}
                  • Duration: ${this.formatDuration(candidate.duration_seconds * 1000)}
                  ${
                    Math.abs(candidate.duration_delta_seconds) > 0.5
                      ? ` (Δ ${candidate.duration_delta_seconds > 0 ? '+' : ''}${candidate.duration_delta_seconds.toFixed(1)}s)`
                      : ' (Exact alignment)'
                  }
                </div>
              </div>
              <div class="result-actions">
                <button class="button button-secondary preview-lyrics-btn" data-index="${idx}" title="Preview in Lyrics Manager">
                  Preview
                </button>
                <button class="button button-primary use-lyrics-btn" data-index="${idx}" title="Assign to track">
                  Use
                </button>
              </div>
            </div>
          `
            )
            .join('')}
        </div>
      </div>
    `;
  }

  private attachEventListeners() {
    const dialog = this.container.querySelector('.universal-search-dialog');
    const input = this.container.querySelector('.universal-search-input') as HTMLInputElement;
    const clearBtn = this.container.querySelector('.universal-search-clear-btn');
    const modeTabs = this.container.querySelectorAll('.mode-tab');
    const switchToOnlineBtn = this.container.querySelector('.switch-to-online-btn');
    const retryBtn = this.container.querySelector('.retry-search-btn');

    // Prevent backdrop click closing when clicking dialog
    dialog?.addEventListener('click', (e) => e.stopPropagation());

    // Backdrop click closes search
    this.container.querySelector('.universal-search-backdrop')?.addEventListener('click', () => {
      appState.closeUniversalSearch();
    });

    // Input query typing with 350ms debounce
    input?.addEventListener('input', () => {
      this.currentQuery = input.value;
      appState.setUniversalSearchQuery(this.currentQuery);
      if (this.debounceTimer !== null) {
        clearTimeout(this.debounceTimer);
      }
      if (this.currentScope === 'local') {
        this.executeSearch(false);
      } else {
        const setTimer = typeof window !== 'undefined' ? window.setTimeout : globalThis.setTimeout;
        this.debounceTimer = setTimer(() => {
          this.executeSearch(false);
        }, 350) as unknown as number;
      }
    });

    // Keyboard navigation (Up/Down/Enter/Escape/Tab)
    input?.addEventListener('keydown', (e: KeyboardEvent) => {
      if (e.key === 'ArrowDown') {
        e.preventDefault();
        this.moveSelection(1);
      } else if (e.key === 'ArrowUp') {
        e.preventDefault();
        this.moveSelection(-1);
      } else if (e.key === 'Enter') {
        e.preventDefault();
        this.triggerPrimarySelection();
      } else if (e.key === 'Escape') {
        e.preventDefault();
        appState.closeUniversalSearch();
      } else if (e.key === 'Tab') {
        e.preventDefault();
        this.cycleScope(e.shiftKey ? -1 : 1);
      }
    });

    // Clear Button
    clearBtn?.addEventListener('click', () => {
      this.currentQuery = '';
      appState.setUniversalSearchQuery('');
      this.render();
    });

    // Switch Scope Tabs
    modeTabs.forEach((tab) => {
      tab.addEventListener('click', () => {
        const scope = tab.getAttribute('data-scope') as SearchScope;
        if (scope && scope !== this.currentScope) {
          this.currentScope = scope;
          appState.setUniversalSearchScope(scope);
          this.selectedIndex = -1;
          this.render();
          if (this.currentQuery.trim()) {
            this.executeSearch(true);
          }
        }
      });
    });

    // Switch to Online shortcut button from empty local results
    switchToOnlineBtn?.addEventListener('click', () => {
      this.currentScope = 'online_metadata';
      appState.setUniversalSearchScope('online_metadata');
      this.render();
      this.executeSearch(true);
    });

    // Retry Button
    retryBtn?.addEventListener('click', () => {
      this.errorMessage = null;
      this.executeSearch(true);
    });

    // Delegate row and action button clicks
    this.attachResultRowActions();
  }

  private attachResultRowActions() {
    const resultsViewport = this.container.querySelector('#search-results-viewport');
    if (!resultsViewport) return;

    // Play local track
    resultsViewport.querySelectorAll('.play-track-btn').forEach((btn) => {
      btn.addEventListener('click', (e) => {
        e.stopPropagation();
        const id = Number(btn.getAttribute('data-id'));
        if (id) {
          appState.playTrack(id);
          appState.closeUniversalSearch();
        }
      });
    });

    // View local album
    resultsViewport.querySelectorAll('.view-album-btn, .album-result').forEach((el) => {
      el.addEventListener('click', () => {
        const id = Number(el.getAttribute('data-id'));
        const title = el.getAttribute('data-title') || 'Album';
        const artist = el.getAttribute('data-artist') || undefined;
        if (id) {
          appState.setActiveView({ type: 'album_detail', albumId: id, albumTitle: title, artistName: artist });
          appState.closeUniversalSearch();
        }
      });
    });

    // View local artist
    resultsViewport.querySelectorAll('.view-artist-btn, .artist-result').forEach((el) => {
      el.addEventListener('click', () => {
        const id = Number(el.getAttribute('data-id'));
        const name = el.getAttribute('data-name') || '';
        if (id) {
          appState.setActiveView({ type: 'artist_detail', artistId: id, artistName: name });
          appState.closeUniversalSearch();
        }
      });
    });

    // Online Track Inspect (Match Inspector)
    resultsViewport.querySelectorAll('.inspect-online-btn').forEach((btn) => {
      btn.addEventListener('click', (e) => {
        e.stopPropagation();
        const title = btn.getAttribute('data-title') || '';
        const artist = btn.getAttribute('data-artist') || '';
        appState.openMatchInspector({
          title,
          artistName: artist,
        });
        appState.closeUniversalSearch();
      });
    });

    // Online Track Find Lyrics
    resultsViewport.querySelectorAll('.find-lyrics-btn').forEach((btn) => {
      btn.addEventListener('click', (e) => {
        e.stopPropagation();
        const title = btn.getAttribute('data-title') || '';
        const artist = btn.getAttribute('data-artist') || '';
        appState.openLyricsManager({
          title,
          artist,
        });
        appState.closeUniversalSearch();
      });
    });

    // Online Track / Album Find Artwork
    resultsViewport.querySelectorAll('.find-art-btn').forEach((btn) => {
      btn.addEventListener('click', (e) => {
        e.stopPropagation();
        const releaseMbid = btn.getAttribute('data-rel') || null;
        const releaseGroupMbid = btn.getAttribute('data-rg') || null;
        const artistName = btn.getAttribute('data-artist') || null;
        const albumTitle = btn.getAttribute('data-title') || null;
        appState.openArtworkFinder({
          targetType: 'album',
          title: albumTitle || 'Album Artwork',
          artistName: artistName || undefined,
          releaseMbid: releaseMbid || undefined,
          releaseGroupMbid: releaseGroupMbid || undefined,
        });
        appState.closeUniversalSearch();
      });
    });

    // Online Artist Find Artwork
    resultsViewport.querySelectorAll('.find-artist-art-btn').forEach((btn) => {
      btn.addEventListener('click', (e) => {
        e.stopPropagation();
        const artistMbid = btn.getAttribute('data-mbid') || null;
        const artistName = btn.getAttribute('data-name') || null;
        appState.openArtworkFinder({
          targetType: 'artist',
          title: artistName || 'Artist Artwork',
          artistName: artistName || undefined,
          mbid: artistMbid || undefined,
        });
        appState.closeUniversalSearch();
      });
    });

    // Lyrics Candidate Preview
    resultsViewport.querySelectorAll('.preview-lyrics-btn').forEach((btn) => {
      btn.addEventListener('click', (e) => {
        e.stopPropagation();
        const idx = Number(btn.getAttribute('data-index'));
        const candidate = this.lyricsResults[idx];
        if (candidate) {
          appState.openLyricsManager({
            title: candidate.track_name,
            artist: candidate.artist_name,
            album: candidate.album_name,
            durationMs: candidate.duration_seconds * 1000,
          });
          appState.closeUniversalSearch();
        }
      });
    });

    // Lyrics Candidate Use
    resultsViewport.querySelectorAll('.use-lyrics-btn').forEach((btn) => {
      btn.addEventListener('click', async (e) => {
        e.stopPropagation();
        const idx = Number(btn.getAttribute('data-index'));
        const candidate = this.lyricsResults[idx];
        if (candidate) {
          const currentTrack = appState.getStatus().current_track;
          try {
            await api.applyLyricsCandidate(
              candidate,
              currentTrack?.track_id ?? null,
              currentTrack?.file_path ?? null
            );
            btn.textContent = 'Applied ✓';
            (btn as HTMLButtonElement).disabled = true;
          } catch (err) {
            console.error('Failed to apply lyrics candidate:', err);
          }
        }
      });
    });
  }

  // --- Execution & Async Queries ---
  public async executeSearch(_immediate: boolean = false) {
    const query = this.currentQuery.trim();
    if (!query) {
      this.localResults = { tracks: [], albums: [], artists: [] };
      this.onlineResults = { recordings: [], releaseGroups: [], artists: [] };
      this.lyricsResults = [];
      this.isLoading = false;
      this.render();
      return;
    }

    const currentReq = ++this.requestId;

    if (this.currentScope === 'local') {
      // Instant local search
      try {
        const [tracks, allAlbums, allArtists] = await Promise.all([
          api.searchLibrary(query, 25),
          api.getAllAlbums(),
          api.getAllArtists(),
        ]);

        if (currentReq !== this.requestId) return;

        const qLower = query.toLowerCase();
        const matchedAlbums = allAlbums.filter(
          (a) =>
            a.title.toLowerCase().includes(qLower) ||
            (a.artist_name && a.artist_name.toLowerCase().includes(qLower))
        );
        const matchedArtists = allArtists.filter((a) => a.name.toLowerCase().includes(qLower));

        this.localResults = {
          tracks,
          albums: matchedAlbums.slice(0, 10),
          artists: matchedArtists.slice(0, 10),
        };
        this.isLoading = false;
        this.errorMessage = null;
        this.render();
      } catch (err) {
        if (currentReq === this.requestId) {
          this.errorMessage = 'Local search failed: ' + (err as Error).message;
          this.isLoading = false;
          this.render();
        }
      }
      return;
    }

    // Online searches
    this.isLoading = true;
    this.errorMessage = null;
    this.render();

    try {
      if (this.currentScope === 'online_metadata') {
        const [recordings, releaseGroups, artists] = await Promise.allSettled([
          api.searchOnlineTracks(query, 15),
          api.searchOnlineAlbums(query, 10),
          api.searchOnlineArtists(query, 10),
        ]);

        if (currentReq !== this.requestId) return;

        this.onlineResults = {
          recordings: recordings.status === 'fulfilled' ? recordings.value : [],
          releaseGroups: releaseGroups.status === 'fulfilled' ? releaseGroups.value : [],
          artists: artists.status === 'fulfilled' ? artists.value : [],
        };
        this.isLoading = false;
        this.render();
      } else if (this.currentScope === 'lyrics') {
        const candidates = await api.findLyricsCandidates({
          track_name: query,
        });

        if (currentReq !== this.requestId) return;

        this.lyricsResults = candidates;
        this.isLoading = false;
        this.render();
      }
    } catch (err: any) {
      if (currentReq !== this.requestId) return;

      this.isLoading = false;
      const msg = String(err?.message || err || '');
      if (msg.includes('429') || msg.includes('rate limit')) {
        this.isRateLimited = true;
      } else if (msg.includes('offline') || msg.includes('network') || (typeof navigator !== 'undefined' && !navigator.onLine)) {
        this.isOffline = true;
      } else {
        this.errorMessage = 'Online provider search failed: ' + msg;
      }
      this.render();
    }
  }

  private moveSelection(delta: number) {
    const rows = this.container.querySelectorAll('.result-row');
    if (rows.length === 0) return;

    this.selectedIndex = Math.max(0, Math.min(rows.length - 1, this.selectedIndex + delta));
    rows.forEach((row, i) => {
      row.classList.toggle('active-result', i === this.selectedIndex);
      if (i === this.selectedIndex) {
        row.scrollIntoView({ block: 'nearest' });
      }
    });
  }

  private triggerPrimarySelection() {
    const activeRow = this.container.querySelector('.result-row.active-result') as HTMLElement;
    if (!activeRow) return;

    const type = activeRow.getAttribute('data-type');
    if (type === 'local-track') {
      const id = Number(activeRow.getAttribute('data-id'));
      if (id) {
        appState.playTrack(id);
        appState.closeUniversalSearch();
      }
    } else if (type === 'local-album') {
      const id = Number(activeRow.getAttribute('data-id'));
      const title = activeRow.getAttribute('data-title') || 'Album';
      const artist = activeRow.getAttribute('data-artist') || undefined;
      if (id) {
        appState.setActiveView({ type: 'album_detail', albumId: id, albumTitle: title, artistName: artist });
        appState.closeUniversalSearch();
      }
    } else if (type === 'local-artist') {
      const id = Number(activeRow.getAttribute('data-id'));
      const name = activeRow.getAttribute('data-name') || '';
      if (id) {
        appState.setActiveView({ type: 'artist_detail', artistId: id, artistName: name });
        appState.closeUniversalSearch();
      }
    } else if (type === 'online-track') {
      const inspectBtn = activeRow.querySelector('.inspect-online-btn') as HTMLButtonElement;
      inspectBtn?.click();
    } else if (type === 'online-album') {
      const artBtn = activeRow.querySelector('.find-art-btn') as HTMLButtonElement;
      artBtn?.click();
    } else if (type === 'lyrics-candidate') {
      const previewBtn = activeRow.querySelector('.preview-lyrics-btn') as HTMLButtonElement;
      previewBtn?.click();
    }
  }

  private cycleScope(direction: number) {
    const scopes: SearchScope[] = ['local', 'online_metadata', 'lyrics'];
    const curIdx = scopes.indexOf(this.currentScope);
    const nextIdx = (curIdx + direction + scopes.length) % scopes.length;
    this.currentScope = scopes[nextIdx];
    appState.setUniversalSearchScope(this.currentScope);
    this.selectedIndex = -1;
    this.render();
    if (this.currentQuery.trim()) {
      this.executeSearch(true);
    }
  }

  private formatDuration(ms?: number | null): string {
    if (!ms || isNaN(ms)) return '0:00';
    const totalSecs = Math.floor(ms / 1000);
    const mins = Math.floor(totalSecs / 60);
    const secs = totalSecs % 60;
    return `${mins}:${secs < 10 ? '0' : ''}${secs}`;
  }
}
