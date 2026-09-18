/**
 * Sonora Synchronized Lyrics Component.
 * Features:
 * - Real-time line-level time synchronization
 * - 5 display modes: Classic, Focused/Cinematic, Compact, Minimal, Dual-line
 * - Customization: Font size, weight, alignment, line spacing, highlight style, background mode
 * - Click-to-seek playback integration
 * - Manual offset adjustments (+/- 100ms) with SQLite persistence
 * - Local sidecar .lrc file loader via native file picker
 * - Comprehensive state handling: Loading, Not Found, Error, Unsynced, Instrumental
 */

import { api } from '../api.ts';
import { appState } from '../state.ts';
import { lyricsManager } from '../lyricsManager.ts';
import type { LyricsDocument, LyricsPreferences, PlaybackStatus } from '../types.ts';

export type LyricsState = 'idle' | 'loading' | 'loaded' | 'instrumental' | 'not_found' | 'error';

export class LyricsViewComponent {
  private container: HTMLElement;
  private currentTrackKey: string | null = null;
  private lyricsDoc: LyricsDocument | null = null;
  private lyricsState: LyricsState = 'idle';
  private errorMessage: string | null = null;
  private activeLineIndex: number = -1;
  private userIsScrolling: boolean = false;
  private userScrollTimer: number | null = null;
  private prefs: LyricsPreferences;

  constructor(container: HTMLElement) {
    this.container = container;
    this.prefs = lyricsManager.getPreferences();
    this.render();

    // Subscribe to state updates (track change, position updates)
    appState.subscribe(() => this.onPlaybackUpdate());

    // Subscribe to user preference changes
    lyricsManager.subscribe((newPrefs) => {
      this.prefs = newPrefs;
      this.applyPreferences();
    });
  }

  private render() {
    this.container.innerHTML = `
      <div class="lyrics-view-wrapper" 
           data-lyrics-mode="${this.prefs.mode}"
           data-lyrics-size="${this.prefs.fontSize}"
           data-lyrics-weight="${this.prefs.fontWeight}"
           data-lyrics-align="${this.prefs.alignment}"
           data-lyrics-spacing="${this.prefs.lineSpacing}"
           data-lyrics-highlight="${this.prefs.highlightStyle}"
           data-lyrics-bg="${this.prefs.backgroundMode}">
        
        <!-- Header Toolbar -->
        <div class="lyrics-header">
          <div class="lyrics-title-group">
            <div class="lyrics-title-row">
              <span class="lyrics-heading">Lyrics</span>
              <span class="lyrics-badge" id="lyrics-format-badge">Ready</span>
            </div>
            <span class="lyrics-track-title" id="lyrics-track-info">No track playing</span>
          </div>

          <div class="lyrics-toolbar-actions">
            <!-- Mode Switcher -->
            <div class="lyrics-mode-selector" title="Display Mode">
              <select id="lyrics-mode-select" class="lyrics-select-pill" aria-label="Lyrics Display Mode">
                <option value="classic" ${this.prefs.mode === 'classic' ? 'selected' : ''}>Classic</option>
                <option value="focused" ${this.prefs.mode === 'focused' ? 'selected' : ''}>Focused</option>
                <option value="compact" ${this.prefs.mode === 'compact' ? 'selected' : ''}>Compact</option>
                <option value="minimal" ${this.prefs.mode === 'minimal' ? 'selected' : ''}>Minimal</option>
                <option value="dual" ${this.prefs.mode === 'dual' ? 'selected' : ''}>Dual-Line</option>
              </select>
            </div>

            <!-- Auto-scroll Toggle -->
            <button class="icon-button lyrics-action-btn ${this.prefs.autoScroll ? 'active' : ''}" 
                    id="lyrics-autoscroll-btn" 
                    title="${this.prefs.autoScroll ? 'Auto-scroll enabled' : 'Auto-scroll disabled'}" 
                    aria-label="Toggle Auto-scroll">
              <svg width="15" height="15" viewBox="0 0 24 24" fill="currentColor">
                <path d="M16 13h-3V3h-2v10H8l4 4 4-4zM4 19v2h16v-2H4z"/>
              </svg>
            </button>

            <!-- Manual File Import -->
            <button class="icon-button lyrics-action-btn" id="lyrics-import-lrc-btn" title="Load local .lrc file" aria-label="Load LRC File">
              <svg width="15" height="15" viewBox="0 0 24 24" fill="currentColor">
                <path d="M9 16h6v-6h4l-7-7-7 7h4v6zm-4 2h14v2H5v-2z"/>
              </svg>
            </button>

            <!-- Close Drawer -->
            <button class="icon-button close-lyrics-btn" title="Close lyrics panel" aria-label="Close lyrics">✕</button>
          </div>
        </div>

        <!-- Offset Adjuster Bar -->
        <div class="lyrics-offset-bar">
          <span class="offset-label">Sync Offset:</span>
          <button class="offset-step-btn" id="offset-minus-btn" title="Shift earlier (-100ms)">-100ms</button>
          <span class="offset-current-value" id="offset-display" title="Click to reset">${this.formatOffset(this.prefs.manualOffsetMs)}</span>
          <button class="offset-step-btn" id="offset-plus-btn" title="Shift later (+100ms)">+100ms</button>
          <button class="offset-reset-btn" id="offset-reset-btn" title="Reset offset to 0ms">Reset</button>
        </div>

        <!-- Main Lyrics Scroll Container -->
        <div class="lyrics-body" id="lyrics-scroll-body" tabindex="0">
          <div class="lyrics-empty-state">
            <svg width="36" height="36" viewBox="0 0 24 24" fill="currentColor" opacity="0.4">
              <path d="M12 3v10.55c-.59-.34-1.27-.55-2-.55-2.21 0-4 1.79-4 4s1.79 4 4 4 4-1.79 4-4V7h4V3h-6z"/>
            </svg>
            <p>Play a track to view synchronized lyrics</p>
          </div>
        </div>

        <!-- Floating Scroll Resume Pill (visible when user manually scrolled away) -->
        <button class="lyrics-resume-scroll-pill hidden" id="lyrics-resume-scroll-pill">
          <svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor">
            <path d="M12 16l-6-6h12z"/>
          </svg>
          Resume auto-scroll
        </button>
      </div>
    `;

    this.attachEvents();
  }

  private attachEvents() {
    // Close button
    const closeBtn = this.container.querySelector('.close-lyrics-btn') as HTMLButtonElement;
    closeBtn?.addEventListener('click', () => {
      const event = new CustomEvent('sonora-toggle-lyrics', { bubbles: true });
      this.container.dispatchEvent(event);
    });

    // Display mode change
    const modeSelect = this.container.querySelector('#lyrics-mode-select') as HTMLSelectElement;
    modeSelect?.addEventListener('change', () => {
      lyricsManager.setMode(modeSelect.value as LyricsPreferences['mode']);
    });

    // Auto-scroll toggle
    const autoScrollBtn = this.container.querySelector('#lyrics-autoscroll-btn') as HTMLButtonElement;
    autoScrollBtn?.addEventListener('click', () => {
      const next = !this.prefs.autoScroll;
      lyricsManager.setAutoScroll(next);
    });

    // Offset adjustment buttons
    const minusBtn = this.container.querySelector('#offset-minus-btn') as HTMLButtonElement;
    const plusBtn = this.container.querySelector('#offset-plus-btn') as HTMLButtonElement;
    const resetBtn = this.container.querySelector('#offset-reset-btn') as HTMLButtonElement;
    const offsetDisplay = this.container.querySelector('#offset-display') as HTMLElement;

    minusBtn?.addEventListener('click', () => this.adjustOffset(-100));
    plusBtn?.addEventListener('click', () => this.adjustOffset(100));
    resetBtn?.addEventListener('click', () => this.resetOffset());
    offsetDisplay?.addEventListener('click', () => this.resetOffset());

    // Import local .lrc
    const importBtn = this.container.querySelector('#lyrics-import-lrc-btn') as HTMLButtonElement;
    importBtn?.addEventListener('click', () => this.handlePickLrcFile());

    // Scroll resume pill
    const resumePill = this.container.querySelector('#lyrics-resume-scroll-pill') as HTMLButtonElement;
    resumePill?.addEventListener('click', () => {
      this.userIsScrolling = false;
      resumePill.classList.add('hidden');
      this.scrollToActiveLine(true);
    });

    // User manual scroll detection
    const scrollBody = this.container.querySelector('#lyrics-scroll-body') as HTMLElement;
    scrollBody?.addEventListener('wheel', () => this.onUserManualScroll(), { passive: true });
    scrollBody?.addEventListener('touchmove', () => this.onUserManualScroll(), { passive: true });
  }

  private onUserManualScroll() {
    if (!this.prefs.autoScroll || this.activeLineIndex < 0) return;
    this.userIsScrolling = true;
    const resumePill = this.container.querySelector('#lyrics-resume-scroll-pill') as HTMLElement;
    resumePill?.classList.remove('hidden');

    if (this.userScrollTimer !== null) {
      window.clearTimeout(this.userScrollTimer);
    }
    // Auto-resume auto-scroll after 5 seconds of inactivity
    this.userScrollTimer = window.setTimeout(() => {
      this.userIsScrolling = false;
      resumePill?.classList.add('hidden');
      this.scrollToActiveLine(true);
    }, 5000);
  }

  private applyPreferences() {
    const wrapper = this.container.querySelector('.lyrics-view-wrapper') as HTMLElement;
    if (!wrapper) return;

    wrapper.setAttribute('data-lyrics-mode', this.prefs.mode);
    wrapper.setAttribute('data-lyrics-size', this.prefs.fontSize);
    wrapper.setAttribute('data-lyrics-weight', this.prefs.fontWeight);
    wrapper.setAttribute('data-lyrics-align', this.prefs.alignment);
    wrapper.setAttribute('data-lyrics-spacing', this.prefs.lineSpacing);
    wrapper.setAttribute('data-lyrics-highlight', this.prefs.highlightStyle);
    wrapper.setAttribute('data-lyrics-bg', this.prefs.backgroundMode);

    // Update autoScroll button state
    const autoScrollBtn = this.container.querySelector('#lyrics-autoscroll-btn') as HTMLButtonElement;
    if (autoScrollBtn) {
      autoScrollBtn.classList.toggle('active', this.prefs.autoScroll);
      autoScrollBtn.title = this.prefs.autoScroll ? 'Auto-scroll enabled' : 'Auto-scroll disabled';
    }

    // Update mode select if needed
    const modeSelect = this.container.querySelector('#lyrics-mode-select') as HTMLSelectElement;
    if (modeSelect && modeSelect.value !== this.prefs.mode) {
      modeSelect.value = this.prefs.mode;
    }

    // Update offset display
    const offsetDisplay = this.container.querySelector('#offset-display') as HTMLElement;
    if (offsetDisplay) {
      offsetDisplay.textContent = this.formatOffset(this.prefs.manualOffsetMs);
    }

    // Re-render lyrics body if currently displaying
    if (this.lyricsState === 'loaded' && this.lyricsDoc) {
      this.renderLoadedLyrics();
    }
  }

  private formatOffset(offsetMs: number): string {
    const sign = offsetMs > 0 ? '+' : '';
    return `${sign}${offsetMs}ms`;
  }

  private async adjustOffset(deltaMs: number) {
    lyricsManager.adjustManualOffset(deltaMs);
    const newOffset = lyricsManager.getPreferences().manualOffsetMs;
    const status = appState.getStatus();
    const track = status.current_track;
    if (track) {
      try {
        await api.saveLyricsOffset(track.track_id, track.file_path, newOffset);
      } catch (err) {
        console.warn('Failed to persist lyrics offset to database:', err);
      }
    }
  }

  private async resetOffset() {
    lyricsManager.resetManualOffset();
    const status = appState.getStatus();
    const track = status.current_track;
    if (track) {
      try {
        await api.saveLyricsOffset(track.track_id, track.file_path, 0);
      } catch (err) {
        console.warn('Failed to reset lyrics offset in database:', err);
      }
    }
  }

  private async handlePickLrcFile() {
    const status = appState.getStatus();
    const track = status.current_track;
    if (!track) {
      alert('Please start playing a track before attaching a local .lrc file.');
      return;
    }

    try {
      const filePath = await api.pickLrcFile();
      if (!filePath) return;

      this.lyricsState = 'loading';
      this.renderState();

      const doc = await api.loadLrcFile(
        filePath,
        track.track_id,
        track.file_path,
        track.title,
        track.artist
      );

      this.lyricsDoc = doc;
      this.lyricsState = doc.lines.length === 0 ? 'instrumental' : 'loaded';
      lyricsManager.setManualOffset(doc.offset_ms);
      this.renderState();
    } catch (err) {
      console.error('Failed to load local LRC file:', err);
      this.lyricsState = 'error';
      this.errorMessage = String(err);
      this.renderState();
    }
  }

  private onPlaybackUpdate() {
    const status = appState.getStatus();
    const track = status.current_track;

    const trackInfoEl = this.container.querySelector('#lyrics-track-info') as HTMLElement;
    if (trackInfoEl) {
      if (track) {
        trackInfoEl.textContent = `${track.title} — ${track.artist || 'Unknown Artist'}`;
      } else {
        trackInfoEl.textContent = 'No track playing';
      }
    }

    if (!track) {
      if (this.currentTrackKey !== null) {
        this.currentTrackKey = null;
        this.lyricsDoc = null;
        this.lyricsState = 'idle';
        this.renderState();
      }
      return;
    }

    const trackKey = `${track.track_id ?? ''}_${track.file_path}_${track.title}`;
    if (this.currentTrackKey !== trackKey) {
      this.currentTrackKey = trackKey;
      this.fetchLyrics(track, status);
    }

    // If lyrics are loaded and synchronized, update active line
    if (this.lyricsState === 'loaded' && this.lyricsDoc && this.isDocumentSynced(this.lyricsDoc)) {
      this.syncActiveLine(status.position_ms);
    }
  }

  private isDocumentSynced(doc: LyricsDocument): boolean {
    return doc.format !== 'Plain' && doc.lines.some((l) => l.start_time_ms > 0 || l.end_time_ms != null);
  }

  private async fetchLyrics(track: NonNullable<PlaybackStatus['current_track']>, status: PlaybackStatus) {
    this.lyricsState = 'loading';
    this.errorMessage = null;
    this.activeLineIndex = -1;
    this.renderState();

    try {
      const doc = await api.getLyrics(
        track.track_id,
        track.file_path,
        track.title,
        track.artist,
        track.album,
        status.duration_ms
      );

      if (!doc) {
        this.lyricsDoc = null;
        this.lyricsState = 'not_found';
      } else if (doc.lines.length === 0) {
        this.lyricsDoc = doc;
        this.lyricsState = 'instrumental';
      } else {
        this.lyricsDoc = doc;
        this.lyricsState = 'loaded';
        // Apply manual offset from document if saved
        if (doc.offset_ms !== this.prefs.manualOffsetMs) {
          lyricsManager.setManualOffset(doc.offset_ms);
        }
      }
    } catch (err) {
      console.error('Failed to resolve lyrics:', err);
      this.lyricsState = 'error';
      this.errorMessage = String(err);
    }

    this.renderState();
  }

  private renderState() {
    const scrollBody = this.container.querySelector('#lyrics-scroll-body') as HTMLElement;
    const badge = this.container.querySelector('#lyrics-format-badge') as HTMLElement;
    if (!scrollBody) return;

    switch (this.lyricsState) {
      case 'idle':
        if (badge) badge.textContent = 'Idle';
        scrollBody.innerHTML = `
          <div class="lyrics-empty-state">
            <svg width="36" height="36" viewBox="0 0 24 24" fill="currentColor" opacity="0.4">
              <path d="M12 3v10.55c-.59-.34-1.27-.55-2-.55-2.21 0-4 1.79-4 4s1.79 4 4 4 4-1.79 4-4V7h4V3h-6z"/>
            </svg>
            <p>Play a track to view synchronized lyrics</p>
          </div>
        `;
        break;

      case 'loading':
        if (badge) badge.textContent = 'Searching...';
        scrollBody.innerHTML = `
          <div class="lyrics-loading-state">
            <div class="sonora-spinner"></div>
            <p>Searching embedded tags, local sidecars, and online lyrics...</p>
          </div>
        `;
        break;

      case 'not_found':
        if (badge) badge.textContent = 'Not Found';
        scrollBody.innerHTML = `
          <div class="lyrics-empty-state">
            <svg width="36" height="36" viewBox="0 0 24 24" fill="currentColor" opacity="0.4">
              <path d="M15.5 14h-.79l-.28-.27C15.41 12.59 16 11.11 16 9.5 16 5.91 13.09 3 9.5 3S3 5.91 3 9.5 5.91 16 9.5 16c1.61 0 3.09-.59 4.23-1.57l.27.28v.79l5 4.99L20.49 19l-4.99-5zm-6 0C7.01 14 5 11.99 5 9.5S7.01 5 9.5 5 14 7.01 14 9.5 11.99 14 9.5 14z"/>
            </svg>
            <h3>No lyrics found</h3>
            <p class="subtle-text">We checked embedded tags, sidecar files, and online providers.</p>
            <div class="lyrics-fallback-actions">
              <button class="primary-button" id="lyrics-retry-btn">Search Again</button>
              <button class="secondary-button" id="lyrics-attach-lrc-btn">Load Local .lrc</button>
            </div>
          </div>
        `;
        scrollBody.querySelector('#lyrics-retry-btn')?.addEventListener('click', () => {
          const status = appState.getStatus();
          if (status.current_track) this.fetchLyrics(status.current_track, status);
        });
        scrollBody.querySelector('#lyrics-attach-lrc-btn')?.addEventListener('click', () => {
          this.handlePickLrcFile();
        });
        break;

      case 'instrumental':
        if (badge) badge.textContent = 'Instrumental';
        scrollBody.innerHTML = `
          <div class="lyrics-empty-state lyrics-instrumental-state">
            <div class="instrumental-glyph">♪ ♫ ♪</div>
            <h3>Instrumental Piece</h3>
            <p class="subtle-text">This track is marked as instrumental with no spoken or sung lyrics.</p>
            <div class="lyrics-fallback-actions">
              <button class="secondary-button" id="lyrics-attach-lrc-btn">Attach Custom .lrc</button>
            </div>
          </div>
        `;
        scrollBody.querySelector('#lyrics-attach-lrc-btn')?.addEventListener('click', () => {
          this.handlePickLrcFile();
        });
        break;

      case 'error':
        if (badge) badge.textContent = 'Error';
        scrollBody.innerHTML = `
          <div class="lyrics-empty-state lyrics-error-state">
            <svg width="36" height="36" viewBox="0 0 24 24" fill="currentColor" opacity="0.6" color="var(--error)">
              <path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm1 15h-2v-2h2v2zm0-4h-2V7h2v6z"/>
            </svg>
            <h3>Could not resolve lyrics</h3>
            <p class="subtle-text">${this.errorMessage || 'Unknown error occurred while contacting lyrics providers.'}</p>
            <div class="lyrics-fallback-actions">
              <button class="primary-button" id="lyrics-retry-btn">Retry</button>
            </div>
          </div>
        `;
        scrollBody.querySelector('#lyrics-retry-btn')?.addEventListener('click', () => {
          const status = appState.getStatus();
          if (status.current_track) this.fetchLyrics(status.current_track, status);
        });
        break;

      case 'loaded':
        if (this.lyricsDoc) {
          const isSynced = this.isDocumentSynced(this.lyricsDoc);
          if (badge) {
            badge.textContent = isSynced ? 'Synced (LRC)' : 'Plain Text';
            badge.className = isSynced ? 'lyrics-badge badge-synced' : 'lyrics-badge badge-plain';
          }
          this.renderLoadedLyrics();
        }
        break;
    }
  }

  private renderLoadedLyrics() {
    const scrollBody = this.container.querySelector('#lyrics-scroll-body') as HTMLElement;
    if (!scrollBody || !this.lyricsDoc) return;

    const isSynced = this.isDocumentSynced(this.lyricsDoc);

    // If Unsynced / Plain text
    if (!isSynced) {
      let html = `
        <div class="lyrics-unsynced-container">
          <div class="lyrics-plain-badge">Unsynchronized Lyrics</div>
      `;
      for (const line of this.lyricsDoc.lines) {
        if (line.text.trim() === '') {
          html += `<div class="lyric-line-break"></div>`;
        } else {
          html += `<p class="lyric-line-plain">${this.escapeHtml(line.text)}</p>`;
        }
      }
      html += `</div>`;
      scrollBody.innerHTML = html;
      return;
    }

    // Synchronized lyrics modes
    if (this.prefs.mode === 'dual') {
      this.renderDualLineView(scrollBody);
      return;
    }

    let html = `<div class="lyrics-lines-container">`;
    this.lyricsDoc.lines.forEach((line, index) => {
      const isBlank = line.text.trim() === '';
      const text = isBlank ? '• • •' : this.escapeHtml(line.text);
      html += `
        <div class="lyric-line ${isBlank ? 'lyric-line-blank' : ''} ${index === this.activeLineIndex ? 'active' : ''}" 
             data-index="${index}" 
             data-start="${line.start_time_ms}" 
             data-end="${line.end_time_ms ?? ''}" 
             role="button" 
             tabindex="0"
             title="Click to seek to ${this.formatTimestamp(line.start_time_ms)}">
          <span class="lyric-time-hint">${this.formatTimestamp(line.start_time_ms)}</span>
          <span class="lyric-text">${text}</span>
        </div>
      `;
    });
    html += `</div>`;
    scrollBody.innerHTML = html;

    // Attach click-to-seek listeners
    const lineEls = scrollBody.querySelectorAll('.lyric-line');
    lineEls.forEach((el) => {
      el.addEventListener('click', () => {
        const start = parseInt(el.getAttribute('data-start') || '0', 10);
        this.seekToTimestamp(start);
      });
      el.addEventListener('keydown', (e: Event) => {
        const keyEvent = e as KeyboardEvent;
        if (keyEvent.key === 'Enter' || keyEvent.key === ' ') {
          keyEvent.preventDefault();
          const start = parseInt(el.getAttribute('data-start') || '0', 10);
          this.seekToTimestamp(start);
        }
      });
    });

    if (this.activeLineIndex >= 0) {
      this.scrollToActiveLine(false);
    }
  }

  private renderDualLineView(scrollBody: HTMLElement) {
    if (!this.lyricsDoc) return;
    const lines = this.lyricsDoc.lines;
    const currIdx = Math.max(0, this.activeLineIndex);
    const currLine = lines[currIdx] ?? { text: '♪ ♪ ♪', start_time_ms: 0 };
    const nextLine = lines[currIdx + 1] ?? { text: '— End of Lyrics —', start_time_ms: 0 };

    scrollBody.innerHTML = `
      <div class="lyrics-dual-container">
        <div class="dual-line-block current-block" role="button" tabindex="0" title="Click to seek to current line">
          <span class="dual-line-label">NOW SINGING</span>
          <div class="dual-line-text">${this.escapeHtml(currLine.text || '♪ ♪ ♪')}</div>
          <span class="dual-line-time">${this.formatTimestamp(currLine.start_time_ms)}</span>
        </div>
        <div class="dual-line-block next-block" role="button" tabindex="0" title="Click to seek to upcoming line">
          <span class="dual-line-label">UP NEXT</span>
          <div class="dual-line-text">${this.escapeHtml(nextLine.text || '♪ ♪ ♪')}</div>
          <span class="dual-line-time">${this.formatTimestamp(nextLine.start_time_ms)}</span>
        </div>
      </div>
    `;

    scrollBody.querySelector('.current-block')?.addEventListener('click', () => {
      this.seekToTimestamp(currLine.start_time_ms);
    });
    scrollBody.querySelector('.next-block')?.addEventListener('click', () => {
      this.seekToTimestamp(nextLine.start_time_ms);
    });
  }

  private seekToTimestamp(startMs: number) {
    // Apply opposite of manual offset so audio engine positions exactly at the lyric moment
    const effectiveSeek = Math.max(0, startMs - this.prefs.manualOffsetMs);
    appState.seek(effectiveSeek);
  }

  private syncActiveLine(positionMs: number) {
    if (!this.lyricsDoc) return;

    // Apply manual offset
    const effectiveTime = Math.max(0, positionMs + this.prefs.manualOffsetMs);
    const lines = this.lyricsDoc.lines;

    let targetIdx = -1;
    for (let i = 0; i < lines.length; i++) {
      if (effectiveTime >= lines[i].start_time_ms) {
        targetIdx = i;
      } else {
        break;
      }
    }

    if (targetIdx !== this.activeLineIndex) {
      this.activeLineIndex = targetIdx;

      if (this.prefs.mode === 'dual') {
        const scrollBody = this.container.querySelector('#lyrics-scroll-body') as HTMLElement;
        if (scrollBody) this.renderDualLineView(scrollBody);
        return;
      }

      const scrollBody = this.container.querySelector('#lyrics-scroll-body') as HTMLElement;
      if (!scrollBody) return;

      const lineEls = scrollBody.querySelectorAll('.lyric-line');
      lineEls.forEach((el, idx) => {
        if (idx === targetIdx) {
          el.classList.add('active');
        } else {
          el.classList.remove('active');
        }
      });

      if (this.prefs.autoScroll && !this.userIsScrolling) {
        this.scrollToActiveLine(true);
      }
    }
  }

  private scrollToActiveLine(smooth: boolean = true) {
    const scrollBody = this.container.querySelector('#lyrics-scroll-body') as HTMLElement;
    if (!scrollBody || this.activeLineIndex < 0) return;

    const activeEl = scrollBody.querySelector(`.lyric-line[data-index="${this.activeLineIndex}"]`) as HTMLElement;
    if (activeEl) {
      activeEl.scrollIntoView({
        behavior: smooth ? 'smooth' : 'auto',
        block: 'center',
      });
    }
  }

  private formatTimestamp(ms: number): string {
    const totalSecs = Math.floor(ms / 1000);
    const mins = Math.floor(totalSecs / 60);
    const secs = totalSecs % 60;
    return `${mins}:${secs.toString().padStart(2, '0')}`;
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
