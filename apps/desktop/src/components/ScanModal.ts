import { api } from '../api';
import { appState } from '../state';

export class ScanModalComponent {
  private container: HTMLElement;
  private isScanning: boolean = false;

  constructor(container: HTMLElement) {
    this.container = container;
    this.render();
    appState.subscribe(() => this.update());
  }

  private render() {
    this.container.innerHTML = `
      <div class="modal-backdrop">
        <div class="modal-card" role="dialog" aria-labelledby="modal-scan-title" aria-modal="true">
          <div class="modal-header">
            <h3 id="modal-scan-title" class="modal-title">Scan Music Directory</h3>
            <button class="icon-button close-modal-btn" title="Close (Esc)" aria-label="Close modal">✕</button>
          </div>

          <div class="modal-body">
            <p class="modal-description">
              Provide a directory path containing your audio files (FLAC, MP3, WAV, AAC, OGG, Opus, ALAC). Sonora will extract metadata with Lofty and index your collection into SQLite.
            </p>

            <div class="form-group">
              <label for="scan-path-input" class="form-label">Music Directory</label>
              <div class="input-with-action">
                <input 
                  type="text" 
                  id="scan-path-input" 
                  class="text-input" 
                  placeholder="Select or enter music directory..."
                  value=""
                />
                <button type="button" class="button button-secondary browse-dir-btn" title="Open native directory picker">
                  <svg width="15" height="15" viewBox="0 0 24 24" fill="currentColor">
                    <path d="M10 4H4c-1.1 0-1.99.9-1.99 2L2 18c0 1.1.9 2 2 2h16c1.1 0 2-.9 2-2V8c0-1.1-.9-2-2-2h-8l-2-2z"/>
                  </svg>
                  <span>Browse...</span>
                </button>
              </div>
            </div>

            <!-- Supported Audio Formats Notice -->
            <div class="quick-paths-group">
              <span class="quick-paths-label">Supported Formats:</span>
              <span class="supported-formats-badge">FLAC • ALAC • WAV • AIFF • MP3 • AAC • OGG • Opus</span>
            </div>

            <!-- Scan Status & Results Banner -->
            <div class="scan-result-banner" style="display: none;"></div>
          </div>

          <div class="modal-footer">
            <button class="button button-ghost cancel-scan-btn">Cancel</button>
            <button class="button button-primary start-scan-btn">
              <span class="btn-spinner" style="display: none;"></span>
              <span class="btn-label">Start Scan</span>
            </button>
          </div>
        </div>
      </div>
    `;

    this.attachEventListeners();
  }

  private attachEventListeners() {
    const backdrop = this.container.querySelector('.modal-backdrop') as HTMLElement;
    const closeBtn = this.container.querySelector('.close-modal-btn') as HTMLButtonElement;
    const cancelBtn = this.container.querySelector('.cancel-scan-btn') as HTMLButtonElement;
    const startBtn = this.container.querySelector('.start-scan-btn') as HTMLButtonElement;
    const browseBtn = this.container.querySelector('.browse-dir-btn') as HTMLButtonElement;
    const pathInput = this.container.querySelector('#scan-path-input') as HTMLInputElement;

    const closeModal = () => {
      if (!this.isScanning) {
        appState.toggleScanModal(false);
      }
    };

    closeBtn.addEventListener('click', closeModal);
    cancelBtn.addEventListener('click', closeModal);
    backdrop.addEventListener('click', (e) => {
      if (e.target === backdrop) closeModal();
    });

    browseBtn?.addEventListener('click', async () => {
      try {
        const folder = await api.pickDirectory();
        if (folder) {
          pathInput.value = folder;
          startBtn.focus();
        }
      } catch (err) {
        console.warn('Native folder picker failed or canceled:', err);
      }
    });

    pathInput.addEventListener('keydown', (e) => {
      if (e.key === 'Enter') {
        e.preventDefault();
        startBtn.click();
      }
    });

    startBtn.addEventListener('click', async () => {
      const path = pathInput.value.trim();
      if (!path) {
        this.showResult('Please specify a directory path to scan.', 'error');
        return;
      }

      this.isScanning = true;
      this.setLoading(true);

      try {
        const stats = await api.scanDirectory(path);
        this.showResult(
          `✓ Scan complete: ${stats.indexed_tracks} tracks indexed, ${stats.skipped_unmodified} unchanged skipped, ${stats.errors} errors (${stats.scanned_files} total files).`,
          'success'
        );
        // Refresh app data, queue, and dispatch global library update event
        await appState.refresh();
        window.dispatchEvent(new CustomEvent('sonora-library-updated'));
      } catch (err: any) {
        this.showResult(`Scan failed: ${err.message || String(err)}`, 'error');
      } finally {
        this.isScanning = false;
        this.setLoading(false);
      }
    });
  }

  private showResult(message: string, type: 'success' | 'error') {
    const banner = this.container.querySelector('.scan-result-banner') as HTMLElement;
    if (banner) {
      banner.textContent = message;
      banner.className = `scan-result-banner banner-${type}`;
      banner.style.display = 'block';
    }
  }

  private setLoading(loading: boolean) {
    const startBtn = this.container.querySelector('.start-scan-btn') as HTMLButtonElement;
    const spinner = this.container.querySelector('.btn-spinner') as HTMLElement;
    const label = this.container.querySelector('.btn-label') as HTMLElement;

    if (startBtn && spinner && label) {
      startBtn.disabled = loading;
      spinner.style.display = loading ? 'inline-block' : 'none';
      label.textContent = loading ? 'Scanning...' : 'Start Scan';
    }
  }

  private update() {
    const isOpen = appState.isScanModalVisible();
    if (isOpen) {
      this.container.classList.add('visible');
      const pathInput = this.container.querySelector('#scan-path-input') as HTMLInputElement;
      if (pathInput && !pathInput.value) {
        pathInput.focus();
      }
    } else {
      this.container.classList.remove('visible');
    }
  }
}
