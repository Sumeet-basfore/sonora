import { api } from '../api';
import { appState } from '../state';
import type { ArtworkCandidate, ArtworkQuery } from '../types';

export class ArtworkFinderComponent {
  private container: HTMLElement;
  private candidates: ArtworkCandidate[] = [];
  private selectedIndex: number = 0;
  private isLoading: boolean = false;
  private isApplying: boolean = false;
  private errorMessage: string | null = null;
  private activeFilter: string = 'all';
  private targetInfo: {
    targetType: 'album' | 'artist' | 'track';
    targetId?: number;
    title: string;
    artistName?: string | null;
    mbid?: string | null;
    releaseMbid?: string | null;
    releaseGroupMbid?: string | null;
  } | null = null;

  constructor(container: HTMLElement) {
    this.container = container;
    this.render();
    appState.subscribe(() => this.onStateChange());
  }

  private onStateChange() {
    const isVisible = appState.isArtworkFinderVisible();
    const activeTarget = appState.getActiveArtworkTarget();

    if (!isVisible || !activeTarget) {
      this.container.innerHTML = '';
      this.targetInfo = null;
      this.candidates = [];
      this.selectedIndex = 0;
      this.isLoading = false;
      this.errorMessage = null;
      return;
    }

    if (
      !this.targetInfo ||
      this.targetInfo.targetId !== activeTarget.targetId ||
      this.targetInfo.targetType !== activeTarget.targetType ||
      this.targetInfo.title !== activeTarget.title
    ) {
      this.targetInfo = activeTarget;
      this.candidates = [];
      this.selectedIndex = 0;
      this.activeFilter = 'all';
      this.loadArtwork(activeTarget);
    } else {
      this.render();
    }
  }

  private async loadArtwork(target: {
    targetType: 'album' | 'artist' | 'track';
    targetId?: number;
    title: string;
    artistName?: string | null;
    mbid?: string | null;
    releaseMbid?: string | null;
    releaseGroupMbid?: string | null;
  }) {
    this.isLoading = true;
    this.errorMessage = null;
    this.render();

    const query: ArtworkQuery = {};
    if (target.targetType === 'album') {
      query.album_title = target.title;
      query.artist_name = target.artistName ?? undefined;
      query.release_mbid = (target.releaseMbid || target.mbid) ?? undefined;
      query.release_group_mbid = target.releaseGroupMbid ?? undefined;
    } else if (target.targetType === 'artist') {
      query.artist_name = target.title;
      query.artist_mbid = target.mbid ?? undefined;
    } else {
      query.album_title = target.title;
      query.artist_name = target.artistName ?? undefined;
    }

    try {
      const results = await api.findArtworkCandidates(query);
      this.candidates = results ?? [];
      this.selectedIndex = 0;
    } catch (err: unknown) {
      this.errorMessage = err instanceof Error ? err.message : String(err);
      this.candidates = [];
    } finally {
      this.isLoading = false;
      this.render();
    }
  }

  private async applySelectedArtwork() {
    const filtered = this.getFilteredCandidates();
    const selected = filtered[this.selectedIndex];
    if (!this.targetInfo || !selected) return;

    this.isApplying = true;
    this.render();

    try {
      if (this.targetInfo.targetId !== undefined) {
        await api.applyArtwork(
          this.targetInfo.targetType,
          this.targetInfo.targetId,
          selected.original_url
        );
      }
      await appState.refresh();
      appState.closeArtworkFinder();
    } catch (err: unknown) {
      this.errorMessage = `Failed to apply artwork: ${err instanceof Error ? err.message : String(err)}`;
      this.isApplying = false;
      this.render();
    }
  }

  private getFilteredCandidates(): ArtworkCandidate[] {
    if (this.activeFilter === 'all') return this.candidates;
    return this.candidates.filter(
      (c) => c.provider_name.toLowerCase().replace(/\s+/g, '') === this.activeFilter.toLowerCase()
    );
  }

  private render() {
    if (!appState.isArtworkFinderVisible() || !this.targetInfo) {
      this.container.innerHTML = '';
      return;
    }

    const filtered = this.getFilteredCandidates();
    const selected = filtered[this.selectedIndex] ?? null;

    let contentHtml = '';

    if (this.isLoading) {
      contentHtml = `
        <div class="enrichment-loading-state">
          <div class="spinner-indicator"></div>
          <h4>Querying artwork providers...</h4>
          <p>Fetching candidate imagery from Cover Art Archive, Wikidata, and Fanart.tv.</p>
        </div>
      `;
    } else if (this.errorMessage) {
      contentHtml = `
        <div class="enrichment-error-state">
          <div class="error-icon">⚠️</div>
          <h4>Unable to fetch artwork</h4>
          <p class="error-text">${this.escapeHtml(this.errorMessage)}</p>
          <button class="button button-secondary retry-search-btn">Retry Discovery</button>
        </div>
      `;
    } else if (filtered.length === 0) {
      contentHtml = `
        <div class="enrichment-empty-state">
          <div class="empty-icon">🖼️</div>
          <h4>No artwork candidates found</h4>
          <p>No candidate images were discovered for <em>"${this.escapeHtml(this.targetInfo.title)}"</em>.</p>
          <button class="button button-ghost close-empty-btn">Dismiss</button>
        </div>
      `;
    } else {
      contentHtml = `
        <div class="artwork-finder-layout">
          <!-- Filter Bar -->
          <div class="artwork-filters-bar">
            <button class="filter-chip ${this.activeFilter === 'all' ? 'active' : ''}" data-filter="all">All Providers (${this.candidates.length})</button>
            <button class="filter-chip ${this.activeFilter === 'coverartarchive' ? 'active' : ''}" data-filter="coverartarchive">Cover Art Archive</button>
            <button class="filter-chip ${this.activeFilter === 'wikidata' ? 'active' : ''}" data-filter="wikidata">Wikidata</button>
            <button class="filter-chip ${this.activeFilter === 'fanart.tv' || this.activeFilter === 'fanarttv' ? 'active' : ''}" data-filter="fanarttv">Fanart.tv</button>
          </div>

          <!-- Gallery Grid -->
          <div class="artwork-grid" role="radiogroup" aria-label="Artwork candidates">
            ${filtered
              .map((cand, idx) => {
                const isSelected = idx === this.selectedIndex;
                const dimStr = cand.width && cand.height ? `${cand.width} × ${cand.height}` : 'Variable';
                const confidencePct = Math.round(cand.match_confidence * 100);

                return `
                  <div 
                    class="artwork-card ${isSelected ? 'active' : ''}" 
                    data-index="${idx}"
                    role="radio"
                    aria-checked="${isSelected}"
                    tabindex="0"
                  >
                    <div class="artwork-image-wrapper">
                      <img 
                        src="${this.escapeHtml(cand.preview_thumbnail_url || cand.original_url)}" 
                        alt="Candidate artwork" 
                        loading="lazy"
                        onerror="this.src=''; this.classList.add('image-fallback');"
                      />
                      ${cand.is_canonical ? '<span class="canonical-badge">★ Canonical</span>' : ''}
                      <span class="provider-tag">${this.escapeHtml(cand.provider_name)}</span>
                    </div>
                    <div class="artwork-card-info">
                      <div class="artwork-dim-format">
                        <span class="dim-badge">${dimStr}</span>
                        <span class="format-badge">${this.escapeHtml(cand.format || 'IMG')}</span>
                      </div>
                      <div class="artwork-confidence-bar">
                        <div class="conf-fill" style="width: ${confidencePct}%;"></div>
                      </div>
                    </div>
                  </div>
                `;
              })
              .join('')}
          </div>
        </div>
      `;
    }

    this.container.innerHTML = `
      <div class="modal-backdrop">
        <div class="modal-card artwork-finder-modal" role="dialog" aria-labelledby="art-modal-title" aria-modal="true">
          <div class="modal-header">
            <div class="modal-title-group">
              <h3 id="art-modal-title" class="modal-title">Find Artwork • Visual Discovery</h3>
              <span class="modal-subtitle">
                Target: <strong>${this.escapeHtml(this.targetInfo.title)}</strong> 
                ${this.targetInfo.artistName ? `• ${this.escapeHtml(this.targetInfo.artistName)}` : ''} 
                (${this.targetInfo.targetType.toUpperCase()})
              </span>
            </div>
            <button class="icon-button close-modal-btn" title="Close (Esc)" aria-label="Close modal">✕</button>
          </div>

          <div class="modal-body modal-body-flush">
            ${contentHtml}
          </div>

          <div class="modal-footer">
            <div class="footer-safety-notice">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <rect x="3" y="3" width="18" height="18" rx="2" ry="2"/>
                <circle cx="8.5" cy="8.5" r="1.5"/>
                <polyline points="21 15 16 10 5 21"/>
              </svg>
              <span>Zero-trust pipeline: downloads, resizes to 1200px + 300px thumbnail WebP, and caches locally.</span>
            </div>
            <div class="footer-actions">
              <button class="button button-ghost cancel-btn">Cancel</button>
              <button class="button button-primary set-artwork-btn" ${!selected || this.isApplying ? 'disabled' : ''}>
                ${this.isApplying ? '<span class="btn-spinner"></span> Saving...' : 'Set as Active Artwork'}
              </button>
            </div>
          </div>
        </div>
      </div>
    `;

    this.attachEventListeners();
  }

  private attachEventListeners() {
    const backdrop = this.container.querySelector('.modal-backdrop') as HTMLElement | null;
    const closeBtn = this.container.querySelector('.close-modal-btn') as HTMLButtonElement | null;
    const cancelBtn = this.container.querySelector('.cancel-btn') as HTMLButtonElement | null;
    const closeEmptyBtn = this.container.querySelector('.close-empty-btn') as HTMLButtonElement | null;
    const retryBtn = this.container.querySelector('.retry-search-btn') as HTMLButtonElement | null;
    const setArtBtn = this.container.querySelector('.set-artwork-btn') as HTMLButtonElement | null;

    const closeModal = () => {
      if (!this.isApplying) {
        appState.closeArtworkFinder();
      }
    };

    closeBtn?.addEventListener('click', closeModal);
    cancelBtn?.addEventListener('click', closeModal);
    closeEmptyBtn?.addEventListener('click', closeModal);

    backdrop?.addEventListener('click', (e) => {
      if (e.target === backdrop) closeModal();
    });

    retryBtn?.addEventListener('click', () => {
      if (this.targetInfo) {
        this.loadArtwork(this.targetInfo);
      }
    });

    setArtBtn?.addEventListener('click', () => {
      this.applySelectedArtwork();
    });

    const filterChips = this.container.querySelectorAll('.filter-chip');
    filterChips.forEach((chip) => {
      chip.addEventListener('click', () => {
        const filter = chip.getAttribute('data-filter') || 'all';
        this.activeFilter = filter;
        this.selectedIndex = 0;
        this.render();
      });
    });

    const cards = this.container.querySelectorAll('.artwork-card');
    cards.forEach((card) => {
      card.addEventListener('click', () => {
        const idx = Number(card.getAttribute('data-index'));
        if (!isNaN(idx) && idx !== this.selectedIndex) {
          this.selectedIndex = idx;
          this.render();
        }
      });
    });
  }

  private escapeHtml(str: string): string {
    return str
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;')
      .replace(/'/g, '&#039;');
  }
}
