import { api } from '../api';
import { appState } from '../state';
import type { LyricsCandidate, LyricsCandidateQuery, LyricsDocument } from '../types';

export class LyricsManagerComponent {
  private container: HTMLElement;
  private candidates: LyricsCandidate[] = [];
  private selectedIndex: number = 0;
  private isLoading: boolean = false;
  private isSaving: boolean = false;
  private isExporting: boolean = false;
  private errorMessage: string | null = null;
  private successMessage: string | null = null;
  private offsetMs: number = 0;
  private previewDoc: LyricsDocument | null = null;
  private animationFrameId: number | null = null;

  private currentTrack: {
    trackId?: number | null;
    filePath?: string | null;
    title: string;
    artist?: string | null;
    album?: string | null;
    durationMs?: number | null;
  } | null = null;

  constructor(container: HTMLElement) {
    this.container = container;
    this.render();
    appState.subscribe(() => this.onStateChange());
  }

  private onStateChange() {
    const isVisible = appState.isLyricsManagerVisible();
    const activeTrack = appState.getActiveLyricsTrack();

    if (!isVisible || !activeTrack) {
      this.stopPlaybackSync();
      this.container.innerHTML = '';
      this.currentTrack = null;
      this.candidates = [];
      this.selectedIndex = 0;
      this.offsetMs = 0;
      this.previewDoc = null;
      this.errorMessage = null;
      this.successMessage = null;
      return;
    }

    if (
      !this.currentTrack ||
      this.currentTrack.title !== activeTrack.title ||
      this.currentTrack.trackId !== activeTrack.trackId
    ) {
      this.currentTrack = activeTrack;
      this.candidates = [];
      this.selectedIndex = 0;
      this.offsetMs = 0;
      this.previewDoc = null;
      this.errorMessage = null;
      this.successMessage = null;
      this.loadLyricsCandidates(activeTrack);
      this.startPlaybackSync();
    } else {
      this.render();
    }
  }

  private async loadLyricsCandidates(track: {
    trackId?: number | null;
    filePath?: string | null;
    title: string;
    artist?: string | null;
    album?: string | null;
    durationMs?: number | null;
  }) {
    this.isLoading = true;
    this.errorMessage = null;
    this.render();

    const query: LyricsCandidateQuery = {
      track_name: track.title,
      artist_name: track.artist ?? undefined,
      album_name: track.album ?? undefined,
      duration_seconds: track.durationMs ? track.durationMs / 1000 : undefined,
    };

    try {
      const results = await api.findLyricsCandidates(query);
      this.candidates = results ?? [];
      this.selectedIndex = 0;
      if (this.candidates.length > 0) {
        this.updatePreviewFromCandidate(this.candidates[0]);
      }
    } catch (err: unknown) {
      this.errorMessage = err instanceof Error ? err.message : String(err);
      this.candidates = [];
    } finally {
      this.isLoading = false;
      this.render();
    }
  }

  private parseLrcToDoc(raw: string, title: string, artist?: string | null): LyricsDocument {
    const lines: Array<{ start_time_ms: number; end_time_ms?: number | null; text: string; syllables: [] }> = [];
    const lineRegex = /\[(\d{2}):(\d{2})\.(\d{2,3})\](.*)/g;
    let match;

    while ((match = lineRegex.exec(raw)) !== null) {
      const mins = parseInt(match[1], 10);
      const secs = parseInt(match[2], 10);
      const ms = match[3].length === 2 ? parseInt(match[3], 10) * 10 : parseInt(match[3], 10);
      const startTimeMs = (mins * 60 + secs) * 1000 + ms;
      const text = match[4].trim();
      lines.push({ start_time_ms: startTimeMs, text, syllables: [] });
    }

    if (lines.length === 0) {
      // Plain text fallback
      const rawLines = raw.split('\n');
      rawLines.forEach((t, i) => {
        lines.push({ start_time_ms: i * 3000, text: t.trim(), syllables: [] });
      });
      return {
        title,
        artist,
        album: null,
        offset_ms: this.offsetMs,
        format: 'Plain',
        lines,
      };
    }

    // Sort by timestamp
    lines.sort((a, b) => a.start_time_ms - b.start_time_ms);

    return {
      title,
      artist,
      album: null,
      offset_ms: this.offsetMs,
      format: 'Lrc',
      lines,
    };
  }

  private updatePreviewFromCandidate(candidate: LyricsCandidate) {
    this.previewDoc = this.parseLrcToDoc(
      candidate.raw_content,
      candidate.track_name,
      candidate.artist_name
    );
  }

  private startPlaybackSync() {
    this.stopPlaybackSync();
    const tick = () => {
      this.updateActiveLine();
      this.animationFrameId = requestAnimationFrame(tick);
    };
    this.animationFrameId = requestAnimationFrame(tick);
  }

  private stopPlaybackSync() {
    if (this.animationFrameId !== null) {
      cancelAnimationFrame(this.animationFrameId);
      this.animationFrameId = null;
    }
  }

  private updateActiveLine() {
    if (!this.previewDoc || !this.previewDoc.lines.length) return;
    const status = appState.getStatus();
    const currentMs = status.position_ms + this.offsetMs;

    let activeIdx = -1;
    for (let i = 0; i < this.previewDoc.lines.length; i++) {
      if (currentMs >= this.previewDoc.lines[i].start_time_ms) {
        activeIdx = i;
      } else {
        break;
      }
    }

    const previewContainer = this.container.querySelector('.lyrics-preview-lines');
    if (!previewContainer) return;

    const lineElements = previewContainer.querySelectorAll('.preview-line');
    lineElements.forEach((el, idx) => {
      if (idx === activeIdx) {
        if (!el.classList.contains('active')) {
          el.classList.add('active');
          el.scrollIntoView({ behavior: 'smooth', block: 'center' });
        }
      } else {
        el.classList.remove('active');
      }
    });
  }

  private async saveLyrics() {
    const selected = this.candidates[this.selectedIndex];
    if (!this.currentTrack || !selected) return;

    this.isSaving = true;
    this.errorMessage = null;
    this.successMessage = null;
    this.render();

    try {
      await api.applyLyricsCandidate(
        selected,
        this.currentTrack.trackId,
        this.currentTrack.filePath
      );
      if (this.offsetMs !== 0) {
        await api.saveLyricsOffset(
          this.currentTrack.trackId,
          this.currentTrack.filePath,
          this.offsetMs
        );
      }
      this.successMessage = 'Saved lyrics to library cache successfully!';
      await appState.refresh();
    } catch (err: unknown) {
      this.errorMessage = `Failed to save lyrics: ${err instanceof Error ? err.message : String(err)}`;
    } finally {
      this.isSaving = false;
      this.render();
    }
  }

  private async exportSidecar() {
    const selected = this.candidates[this.selectedIndex];
    if (!this.currentTrack || !selected) return;

    this.isExporting = true;
    this.errorMessage = null;
    this.successMessage = null;
    this.render();

    try {
      const exportedPath = await api.exportLrcSidecar(
        selected.raw_content,
        this.currentTrack.trackId,
        this.currentTrack.filePath
      );
      this.successMessage = `Exported .lrc sidecar: ${exportedPath}`;
    } catch (err: unknown) {
      this.errorMessage = `Failed to export .lrc: ${err instanceof Error ? err.message : String(err)}`;
    } finally {
      this.isExporting = false;
      this.render();
    }
  }

  private adjustOffset(deltaMs: number) {
    this.offsetMs += deltaMs;
    this.renderOffsetDisplay();
  }

  private resetOffset() {
    this.offsetMs = 0;
    this.renderOffsetDisplay();
  }

  private renderOffsetDisplay() {
    const valDisplay = this.container.querySelector('.offset-value-display');
    const slider = this.container.querySelector('.offset-slider') as HTMLInputElement | null;
    if (valDisplay) {
      const sign = this.offsetMs > 0 ? '+' : '';
      valDisplay.textContent = `${sign}${this.offsetMs} ms`;
    }
    if (slider) {
      slider.value = String(this.offsetMs);
    }
  }

  private render() {
    if (!appState.isLyricsManagerVisible() || !this.currentTrack) {
      this.container.innerHTML = '';
      return;
    }

    const selected = this.candidates[this.selectedIndex] ?? null;

    let leftContentHtml = '';

    if (this.isLoading) {
      leftContentHtml = `
        <div class="enrichment-loading-state">
          <div class="spinner-indicator"></div>
          <h4>Searching LRCLIB & providers...</h4>
          <p>Querying synchronized line-by-line lyrics.</p>
        </div>
      `;
    } else if (this.errorMessage && this.candidates.length === 0) {
      leftContentHtml = `
        <div class="enrichment-error-state">
          <div class="error-icon">⚠️</div>
          <h4>Unable to find lyrics</h4>
          <p class="error-text">${this.escapeHtml(this.errorMessage)}</p>
          <button class="button button-secondary retry-search-btn">Retry Search</button>
        </div>
      `;
    } else if (this.candidates.length === 0) {
      leftContentHtml = `
        <div class="enrichment-empty-state">
          <div class="empty-icon">🎵</div>
          <h4>No online lyrics found</h4>
          <p>No candidate lyrics were returned for <em>"${this.escapeHtml(this.currentTrack.title)}"</em>.</p>
        </div>
      `;
    } else {
      leftContentHtml = `
        <div class="lyrics-candidates-list" role="tablist">
          ${this.candidates
            .map((cand, idx) => {
              const isSelected = idx === this.selectedIndex;
              const isSynced = cand.sync_type === 'LineSynced' || cand.sync_type === 'SyllableSynced';
              const syncBadgeClass = isSynced ? 'badge-synced' : 'badge-plain';
              const syncLabel = isSynced ? '⚡ Synchronized LRC' : '📄 Plain Text';
              const deltaLabel =
                cand.duration_delta_seconds === 0
                  ? 'Exact Duration'
                  : `±${cand.duration_delta_seconds.toFixed(1)}s delta`;

              return `
                <div 
                  class="candidate-card lyrics-candidate-card ${isSelected ? 'active' : ''}" 
                  data-index="${idx}"
                  role="tab"
                  aria-selected="${isSelected}"
                  tabindex="0"
                >
                  <div class="candidate-card-top">
                    <span class="candidate-title">${this.escapeHtml(cand.track_name)}</span>
                    <span class="lyrics-sync-badge ${syncBadgeClass}">${syncLabel}</span>
                  </div>
                  <div class="candidate-meta">
                    <span class="candidate-artist">${this.escapeHtml(cand.artist_name)}</span>
                    <span class="candidate-provider">${this.escapeHtml(cand.provider_name)} • ${deltaLabel}</span>
                  </div>
                </div>
              `;
            })
            .join('')}
        </div>
      `;
    }

    const rightPreviewHtml = this.previewDoc
      ? `
        <div class="lyrics-preview-container">
          <div class="lyrics-preview-lines">
            ${this.previewDoc.lines
              .map((line, idx) => {
                return `<div class="preview-line" data-index="${idx}"><span class="preview-line-time">[${this.formatTimeMs(line.start_time_ms)}]</span> <span class="preview-line-text">${this.escapeHtml(line.text || '♪')}</span></div>`;
              })
              .join('')}
          </div>
        </div>
      `
      : `
        <div class="lyrics-preview-empty">
          <p>Select a lyrics candidate to preview synchronization.</p>
        </div>
      `;

    const sign = this.offsetMs > 0 ? '+' : '';
    const offsetDisplayStr = `${sign}${this.offsetMs} ms`;

    this.container.innerHTML = `
      <div class="modal-backdrop">
        <div class="modal-card lyrics-manager-modal" role="dialog" aria-labelledby="lyrics-modal-title" aria-modal="true">
          <div class="modal-header">
            <div class="modal-title-group">
              <h3 id="lyrics-modal-title" class="modal-title">Lyrics Manager • Online Discovery & Alignment</h3>
              <span class="modal-subtitle">
                Track: <strong>${this.escapeHtml(this.currentTrack.title)}</strong>
                ${this.currentTrack.artist ? `by ${this.escapeHtml(this.currentTrack.artist)}` : ''}
              </span>
            </div>
            <button class="icon-button close-modal-btn" title="Close (Esc)" aria-label="Close modal">✕</button>
          </div>

          <div class="modal-body modal-body-flush">
            ${
              this.successMessage
                ? `<div class="banner-success">${this.escapeHtml(this.successMessage)}</div>`
                : ''
            }
            ${
              this.errorMessage && this.candidates.length > 0
                ? `<div class="banner-error">${this.escapeHtml(this.errorMessage)}</div>`
                : ''
            }

            <div class="lyrics-manager-layout">
              <!-- Left Candidates Panel -->
              <div class="lyrics-left-panel">
                <div class="lyrics-panel-header">
                  <span>Found ${this.candidates.length} Candidate${this.candidates.length !== 1 ? 's' : ''}</span>
                </div>
                ${leftContentHtml}
              </div>

              <!-- Right Audition & Timing Panel -->
              <div class="lyrics-right-panel">
                <div class="lyrics-panel-header">
                  <span>Live Preview & Timing Alignment</span>
                </div>
                ${rightPreviewHtml}

                <!-- Timing Offset Stepper & Slider -->
                <div class="timing-alignment-bar">
                  <div class="offset-header">
                    <span class="offset-label">Precision Timing Offset</span>
                    <span class="offset-value-display monospace">${offsetDisplayStr}</span>
                  </div>

                  <div class="offset-stepper-row">
                    <button class="button button-secondary offset-step-btn" data-delta="-100">-100ms</button>
                    <button class="button button-secondary offset-step-btn" data-delta="-10">-10ms</button>
                    <button class="button button-ghost offset-reset-btn">Reset (0ms)</button>
                    <button class="button button-secondary offset-step-btn" data-delta="10">+10ms</button>
                    <button class="button button-secondary offset-step-btn" data-delta="100">+100ms</button>
                  </div>

                  <div class="offset-slider-row">
                    <input 
                      type="range" 
                      class="offset-slider" 
                      min="-5000" 
                      max="5000" 
                      step="10" 
                      value="${this.offsetMs}"
                    />
                  </div>
                </div>
              </div>
            </div>
          </div>

          <div class="modal-footer">
            <div class="footer-safety-notice">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/>
              </svg>
              <span>Saves to SQLite cache or exports standard .lrc next to audio file.</span>
            </div>
            <div class="footer-actions">
              <button class="button button-ghost cancel-btn">Cancel</button>
              <button class="button button-secondary export-lrc-btn" ${!selected || this.isExporting ? 'disabled' : ''}>
                ${this.isExporting ? '<span class="btn-spinner"></span> Exporting...' : 'Export .lrc Sidecar'}
              </button>
              <button class="button button-primary save-lyrics-btn" ${!selected || this.isSaving ? 'disabled' : ''}>
                ${this.isSaving ? '<span class="btn-spinner"></span> Saving...' : 'Save to Library Cache'}
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
    const retryBtn = this.container.querySelector('.retry-search-btn') as HTMLButtonElement | null;
    const saveBtn = this.container.querySelector('.save-lyrics-btn') as HTMLButtonElement | null;
    const exportBtn = this.container.querySelector('.export-lrc-btn') as HTMLButtonElement | null;
    const resetBtn = this.container.querySelector('.offset-reset-btn') as HTMLButtonElement | null;
    const slider = this.container.querySelector('.offset-slider') as HTMLInputElement | null;

    const closeModal = () => {
      if (!this.isSaving && !this.isExporting) {
        this.stopPlaybackSync();
        appState.closeLyricsManager();
      }
    };

    closeBtn?.addEventListener('click', closeModal);
    cancelBtn?.addEventListener('click', closeModal);

    backdrop?.addEventListener('click', (e) => {
      if (e.target === backdrop) closeModal();
    });

    retryBtn?.addEventListener('click', () => {
      if (this.currentTrack) {
        this.loadLyricsCandidates(this.currentTrack);
      }
    });

    saveBtn?.addEventListener('click', () => {
      this.saveLyrics();
    });

    exportBtn?.addEventListener('click', () => {
      this.exportSidecar();
    });

    resetBtn?.addEventListener('click', () => {
      this.resetOffset();
    });

    slider?.addEventListener('input', () => {
      this.offsetMs = Number(slider.value);
      this.renderOffsetDisplay();
    });

    const stepBtns = this.container.querySelectorAll('.offset-step-btn');
    stepBtns.forEach((btn) => {
      btn.addEventListener('click', () => {
        const delta = Number(btn.getAttribute('data-delta'));
        if (!isNaN(delta)) {
          this.adjustOffset(delta);
        }
      });
    });

    const candidateCards = this.container.querySelectorAll('.candidate-card');
    candidateCards.forEach((card) => {
      card.addEventListener('click', () => {
        const idx = Number(card.getAttribute('data-index'));
        if (!isNaN(idx) && idx !== this.selectedIndex) {
          this.selectedIndex = idx;
          if (this.candidates[idx]) {
            this.updatePreviewFromCandidate(this.candidates[idx]);
          }
          this.render();
        }
      });
    });
  }

  private formatTimeMs(ms: number): string {
    const totalSec = Math.floor(ms / 1000);
    const m = Math.floor(totalSec / 60);
    const s = totalSec % 60;
    return `${m.toString().padStart(2, '0')}:${s.toString().padStart(2, '0')}`;
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
