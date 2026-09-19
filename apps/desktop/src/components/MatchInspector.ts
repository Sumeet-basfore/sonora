import { api } from '../api';
import { appState } from '../state';
import type { RankedCandidateMatch, ConfidenceTier } from '../types';

export class MatchInspectorComponent {
  private container: HTMLElement;
  private candidates: RankedCandidateMatch[] = [];
  private selectedIndex: number = 0;
  private isLoading: boolean = false;
  private isApplying: boolean = false;
  private errorMessage: string | null = null;
  private currentTrackId: number | null = null;

  constructor(container: HTMLElement) {
    this.container = container;
    this.render();
    appState.subscribe(() => this.onStateChange());
  }

  private onStateChange() {
    const isVisible = appState.isMatchInspectorVisible();
    const activeTrack = appState.getActiveMatchTrack();

    if (!isVisible || !activeTrack) {
      this.container.innerHTML = '';
      this.currentTrackId = null;
      this.candidates = [];
      this.selectedIndex = 0;
      this.isLoading = false;
      this.errorMessage = null;
      return;
    }

    if (this.currentTrackId !== activeTrack.trackId) {
      this.currentTrackId = activeTrack.trackId;
      this.candidates = [];
      this.selectedIndex = 0;
      this.loadCandidates(activeTrack.trackId);
    } else {
      this.render();
    }
  }

  private async loadCandidates(trackId: number) {
    this.isLoading = true;
    this.errorMessage = null;
    this.render();

    try {
      const results = await api.findMetadataCandidates(trackId);
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

  private async applySelectedCandidate() {
    if (!this.currentTrackId || !this.candidates[this.selectedIndex]) return;
    this.isApplying = true;
    this.render();

    try {
      const candidate = this.candidates[this.selectedIndex];
      await api.applyMetadata(this.currentTrackId, candidate);
      await appState.refresh();
      appState.closeMatchInspector();
    } catch (err: unknown) {
      this.errorMessage = `Failed to apply metadata: ${err instanceof Error ? err.message : String(err)}`;
      this.isApplying = false;
      this.render();
    }
  }

  private getConfidenceBadgeClass(tier: ConfidenceTier): string {
    switch (tier) {
      case 'high':
        return 'badge-confidence-high';
      case 'medium':
        return 'badge-confidence-medium';
      case 'low':
        return 'badge-confidence-low';
      default:
        return '';
    }
  }

  private computeFieldDiff(
    localVal: string | number | null | undefined,
    candidateVal: string | number | null | undefined
  ): { status: 'exact' | 'fuzzy' | 'conflict' | 'missing'; label: string; badgeClass: string } {
    const l = localVal !== null && localVal !== undefined ? String(localVal).trim() : '';
    const c = candidateVal !== null && candidateVal !== undefined ? String(candidateVal).trim() : '';

    if (!l && !c) {
      return { status: 'missing', label: 'Empty', badgeClass: 'diff-missing' };
    }
    if (!l && c) {
      return { status: 'fuzzy', label: 'New Data', badgeClass: 'diff-fuzzy' };
    }
    if (l && !c) {
      return { status: 'missing', label: 'Missing Online', badgeClass: 'diff-missing' };
    }
    if (l.toLowerCase() === c.toLowerCase()) {
      return { status: 'exact', label: 'Exact Match', badgeClass: 'diff-exact' };
    }
    return { status: 'conflict', label: 'Conflict / Update', badgeClass: 'diff-conflict' };
  }

  private render() {
    if (!appState.isMatchInspectorVisible()) {
      this.container.innerHTML = '';
      return;
    }

    const localTrack = appState.getActiveMatchTrack();
    if (!localTrack) return;

    const selectedCandidate = this.candidates[this.selectedIndex] ?? null;

    let contentHtml = '';

    if (this.isLoading) {
      contentHtml = `
        <div class="enrichment-loading-state">
          <div class="spinner-indicator"></div>
          <h4>Searching MusicBrainz database...</h4>
          <p>Querying recordings, releases, and artist credits.</p>
        </div>
      `;
    } else if (this.errorMessage) {
      contentHtml = `
        <div class="enrichment-error-state">
          <div class="error-icon">⚠️</div>
          <h4>Unable to fetch metadata candidates</h4>
          <p class="error-text">${this.escapeHtml(this.errorMessage)}</p>
          <button class="button button-secondary retry-search-btn">Retry Search</button>
        </div>
      `;
    } else if (this.candidates.length === 0) {
      contentHtml = `
        <div class="enrichment-empty-state">
          <div class="empty-icon">🔍</div>
          <h4>No online candidate matches found</h4>
          <p>No matching MusicBrainz recordings were returned for <em>"${this.escapeHtml(localTrack.title)}"</em>.</p>
          <button class="button button-ghost close-empty-btn">Dismiss</button>
        </div>
      `;
    } else if (selectedCandidate) {
      const breakdown = selectedCandidate.score_breakdown;
      const pct = Math.round(breakdown.total_score * 100);

      // Local vs Candidate Values
      const candidateArtist = selectedCandidate.candidate_track.artist_credits
        .map((c) => c.name + (c.join_phrase ?? ''))
        .join('');
      const candidateAlbum =
        selectedCandidate.candidate_release?.title ??
        selectedCandidate.candidate_release_group?.title ??
        '';
      const candidateYear =
        selectedCandidate.candidate_release?.date?.split('-')[0] ??
        selectedCandidate.candidate_release_group?.first_release_date?.split('-')[0] ??
        '';
      const candidateTrackNum = selectedCandidate.candidate_track.position
        ? String(selectedCandidate.candidate_track.position)
        : selectedCandidate.candidate_track.number ?? '';
      const candidateDurationSec = selectedCandidate.candidate_track.duration_ms
        ? `${Math.round(selectedCandidate.candidate_track.duration_ms / 1000)}s`
        : '';
      const localDurationSec = localTrack.durationMs
        ? `${Math.round(localTrack.durationMs / 1000)}s`
        : '';

      const diffTitle = this.computeFieldDiff(localTrack.title, selectedCandidate.candidate_track.title);
      const diffArtist = this.computeFieldDiff(localTrack.artistName, candidateArtist);
      const diffAlbum = this.computeFieldDiff(localTrack.albumTitle, candidateAlbum);
      const diffTrackNum = this.computeFieldDiff(localTrack.trackNumber, candidateTrackNum);

      contentHtml = `
        <div class="match-inspector-layout">
          <!-- Left Candidates List -->
          <div class="candidates-sidebar">
            <div class="candidates-header">
              <span class="candidates-count">${this.candidates.length} Candidate${this.candidates.length > 1 ? 's' : ''} Found</span>
            </div>
            <div class="candidates-list" role="tablist">
              ${this.candidates
                .map((cand, idx) => {
                  const isSelected = idx === this.selectedIndex;
                  const cPct = Math.round(cand.score_breakdown.total_score * 100);
                  const cArtist = cand.candidate_track.artist_credits.map((c) => c.name).join(', ');
                  const cAlbum =
                    cand.candidate_release?.title ??
                    cand.candidate_release_group?.title ??
                    'Unknown Album';
                  const cYear =
                    cand.candidate_release?.date?.split('-')[0] ??
                    cand.candidate_release_group?.first_release_date?.split('-')[0] ??
                    '';

                  return `
                    <div 
                      class="candidate-card ${isSelected ? 'active' : ''}" 
                      data-index="${idx}"
                      role="tab"
                      aria-selected="${isSelected}"
                      tabindex="0"
                    >
                      <div class="candidate-card-top">
                        <span class="candidate-title">${this.escapeHtml(cand.candidate_track.title)}</span>
                        <span class="confidence-badge ${this.getConfidenceBadgeClass(cand.score_breakdown.confidence_tier)}">
                          ${cPct}% ${cand.score_breakdown.confidence_tier}
                        </span>
                      </div>
                      <div class="candidate-meta">
                        <span class="candidate-artist">${this.escapeHtml(cArtist)}</span>
                        <span class="candidate-album-year">${this.escapeHtml(cAlbum)}${cYear ? ` (${cYear})` : ''}</span>
                      </div>
                    </div>
                  `;
                })
                .join('')}
            </div>
          </div>

          <!-- Right Candidate Diff & Score Breakdown -->
          <div class="candidate-diff-pane">
            <!-- Score Breakdown Banner -->
            <div class="score-breakdown-card">
              <div class="score-main">
                <div class="score-circle ${this.getConfidenceBadgeClass(breakdown.confidence_tier)}">
                  <span class="score-number">${pct}%</span>
                  <span class="score-tier-label">${breakdown.confidence_tier.toUpperCase()}</span>
                </div>
                <div class="score-factors">
                  <div class="score-factor">
                    <span class="factor-label">Title</span>
                    <div class="factor-bar-bg"><div class="factor-bar-fill" style="width: ${Math.round(breakdown.title_score * 100)}%;"></div></div>
                    <span class="factor-val">${Math.round(breakdown.title_score * 100)}%</span>
                  </div>
                  <div class="score-factor">
                    <span class="factor-label">Artist</span>
                    <div class="factor-bar-bg"><div class="factor-bar-fill" style="width: ${Math.round(breakdown.artist_score * 100)}%;"></div></div>
                    <span class="factor-val">${Math.round(breakdown.artist_score * 100)}%</span>
                  </div>
                  <div class="score-factor">
                    <span class="factor-label">Album</span>
                    <div class="factor-bar-bg"><div class="factor-bar-fill" style="width: ${Math.round(breakdown.album_score * 100)}%;"></div></div>
                    <span class="factor-val">${Math.round(breakdown.album_score * 100)}%</span>
                  </div>
                  <div class="score-factor">
                    <span class="factor-label">Duration</span>
                    <div class="factor-bar-bg"><div class="factor-bar-fill" style="width: ${Math.round(breakdown.duration_score * 100)}%;"></div></div>
                    <span class="factor-val">${Math.round(breakdown.duration_score * 100)}%</span>
                  </div>
                </div>
              </div>
            </div>

            <!-- Side-by-Side Field Comparison Table -->
            <div class="diff-table-container">
              <table class="diff-table">
                <thead>
                  <tr>
                    <th>Field</th>
                    <th>Local Value</th>
                    <th>Online Candidate</th>
                    <th>Status</th>
                  </tr>
                </thead>
                <tbody>
                  <tr>
                    <td class="field-name">Title</td>
                    <td class="local-val">${this.escapeHtml(localTrack.title || '—')}</td>
                    <td class="candidate-val">${this.escapeHtml(selectedCandidate.candidate_track.title || '—')}</td>
                    <td><span class="diff-badge ${diffTitle.badgeClass}">${diffTitle.label}</span></td>
                  </tr>
                  <tr>
                    <td class="field-name">Artist</td>
                    <td class="local-val">${this.escapeHtml(localTrack.artistName || '—')}</td>
                    <td class="candidate-val">${this.escapeHtml(candidateArtist || '—')}</td>
                    <td><span class="diff-badge ${diffArtist.badgeClass}">${diffArtist.label}</span></td>
                  </tr>
                  <tr>
                    <td class="field-name">Album</td>
                    <td class="local-val">${this.escapeHtml(localTrack.albumTitle || '—')}</td>
                    <td class="candidate-val">${this.escapeHtml(candidateAlbum || '—')}</td>
                    <td><span class="diff-badge ${diffAlbum.badgeClass}">${diffAlbum.label}</span></td>
                  </tr>
                  <tr>
                    <td class="field-name">Year</td>
                    <td class="local-val">—</td>
                    <td class="candidate-val">${this.escapeHtml(candidateYear || '—')}</td>
                    <td><span class="diff-badge diff-fuzzy">${candidateYear ? 'Online Added' : 'Empty'}</span></td>
                  </tr>
                  <tr>
                    <td class="field-name">Track #</td>
                    <td class="local-val">${localTrack.trackNumber ? String(localTrack.trackNumber) : '—'}</td>
                    <td class="candidate-val">${this.escapeHtml(candidateTrackNum || '—')}</td>
                    <td><span class="diff-badge ${diffTrackNum.badgeClass}">${diffTrackNum.label}</span></td>
                  </tr>
                  <tr>
                    <td class="field-name">Duration</td>
                    <td class="local-val">${this.escapeHtml(localDurationSec || '—')}</td>
                    <td class="candidate-val">${this.escapeHtml(candidateDurationSec || '—')}</td>
                    <td><span class="diff-badge diff-exact">Matched</span></td>
                  </tr>
                  <tr>
                    <td class="field-name">MBID (Recording)</td>
                    <td class="local-val">—</td>
                    <td class="candidate-val monospace">${this.escapeHtml(selectedCandidate.candidate_track.recording_mbid || '—')}</td>
                    <td><span class="diff-badge diff-exact">Verified</span></td>
                  </tr>
                  ${
                    selectedCandidate.candidate_release?.mbid
                      ? `
                  <tr>
                    <td class="field-name">MBID (Release)</td>
                    <td class="local-val">—</td>
                    <td class="candidate-val monospace">${this.escapeHtml(selectedCandidate.candidate_release.mbid)}</td>
                    <td><span class="diff-badge diff-exact">Linked</span></td>
                  </tr>
                  `
                      : ''
                  }
                </tbody>
              </table>
            </div>
          </div>
        </div>
      `;
    }

    this.container.innerHTML = `
      <div class="modal-backdrop">
        <div class="modal-card match-inspector-modal" role="dialog" aria-labelledby="match-modal-title" aria-modal="true">
          <div class="modal-header">
            <div class="modal-title-group">
              <h3 id="match-modal-title" class="modal-title">Find Metadata • MusicBrainz Match Inspector</h3>
              <span class="modal-subtitle">Local Track: <strong>${this.escapeHtml(localTrack.title)}</strong> ${localTrack.artistName ? `by ${this.escapeHtml(localTrack.artistName)}` : ''}</span>
            </div>
            <button class="icon-button close-modal-btn" title="Close (Esc)" aria-label="Close modal">✕</button>
          </div>

          <div class="modal-body modal-body-flush">
            ${contentHtml}
          </div>

          <div class="modal-footer">
            <div class="footer-safety-notice">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/>
              </svg>
              <span>Non-destructive: updates SQLite library database. Audio files on disk are never altered.</span>
            </div>
            <div class="footer-actions">
              <button class="button button-ghost cancel-btn">Cancel</button>
              <button class="button button-primary apply-metadata-btn" ${!selectedCandidate || this.isApplying ? 'disabled' : ''}>
                ${this.isApplying ? '<span class="btn-spinner"></span> Applying...' : 'Apply Metadata'}
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
    const applyBtn = this.container.querySelector('.apply-metadata-btn') as HTMLButtonElement | null;

    const closeModal = () => {
      if (!this.isApplying) {
        appState.closeMatchInspector();
      }
    };

    closeBtn?.addEventListener('click', closeModal);
    cancelBtn?.addEventListener('click', closeModal);
    closeEmptyBtn?.addEventListener('click', closeModal);

    backdrop?.addEventListener('click', (e) => {
      if (e.target === backdrop) closeModal();
    });

    retryBtn?.addEventListener('click', () => {
      if (this.currentTrackId) {
        this.loadCandidates(this.currentTrackId);
      }
    });

    applyBtn?.addEventListener('click', () => {
      this.applySelectedCandidate();
    });

    const cards = this.container.querySelectorAll('.candidate-card');
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
