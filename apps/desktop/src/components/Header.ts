import { appState } from '../state';

export class HeaderComponent {
  private container: HTMLElement;
  private searchDebounceTimer: number | null = null;

  constructor(container: HTMLElement) {
    this.container = container;
    this.render();
    appState.subscribe(() => this.update());
  }

  private render() {
    this.container.innerHTML = `
      <div class="header-inner">
        <div class="header-left">
          <button class="nav-back-btn icon-button" title="Back" aria-label="Go back" style="display: none;">
            <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor">
              <path d="M20 11H7.83l5.59-5.59L12 4l-8 8 8 8 1.41-1.41L7.83 13H20v-2z"/>
            </svg>
          </button>
          <div class="breadcrumbs">
            <span class="breadcrumb-root">Library</span>
            <span class="breadcrumb-separator">/</span>
            <span class="breadcrumb-current">Albums</span>
          </div>
        </div>

        <div class="header-center">
          <div class="search-box">
            <svg class="search-icon" width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
              <path d="M15.5 14h-.79l-.28-.27C15.41 12.59 16 11.11 16 9.5 16 5.91 13.09 3 9.5 3S3 5.91 3 9.5 5.91 16 9.5 16c1.61 0 3.09-.59 4.23-1.57l.27.28v.79l5 4.99L20.49 19l-4.99-5zm-6 0C7.01 14 5 11.99 5 9.5S7.01 5 9.5 5 14 7.01 14 9.5 11.99 14 9.5 14z"/>
            </svg>
            <input 
              type="text" 
              class="search-input" 
              placeholder="Search albums, artists, tracks..." 
              aria-label="Search library"
            />
            <span class="search-shortcut-badge">/</span>
            <button class="search-clear-btn" title="Clear search" aria-label="Clear search" style="display: none;">✕</button>
          </div>
        </div>

        <div class="header-right">
          <button class="button button-secondary scan-dir-btn" title="Scan local music folder">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
              <path d="M10 4H4c-1.1 0-1.99.9-1.99 2L2 18c0 1.1.9 2 2 2h16c1.1 0 2-.9 2-2V8c0-1.1-.9-2-2-2h-8l-2-2z"/>
            </svg>
            <span>Scan Music</span>
          </button>
        </div>
      </div>
    `;

    this.attachEventListeners();
  }

  private attachEventListeners() {
    const searchInput = this.container.querySelector('.search-input') as HTMLInputElement;
    const clearBtn = this.container.querySelector('.search-clear-btn') as HTMLButtonElement;
    const scanBtn = this.container.querySelector('.scan-dir-btn') as HTMLButtonElement;
    const backBtn = this.container.querySelector('.nav-back-btn') as HTMLButtonElement;

    searchInput.addEventListener('input', () => {
      const query = searchInput.value.trim();
      clearBtn.style.display = query ? 'flex' : 'none';

      if (this.searchDebounceTimer) clearTimeout(this.searchDebounceTimer);
      this.searchDebounceTimer = window.setTimeout(() => {
        if (query) {
          appState.setActiveView({ type: 'search', query });
        } else {
          appState.setActiveView({ type: 'albums' });
        }
      }, 150);
    });

    clearBtn.addEventListener('click', () => {
      searchInput.value = '';
      clearBtn.style.display = 'none';
      appState.setActiveView({ type: 'albums' });
      searchInput.focus();
    });

    scanBtn.addEventListener('click', () => {
      appState.toggleScanModal(true);
    });

    backBtn.addEventListener('click', () => {
      appState.setActiveView({ type: 'albums' });
    });
  }

  private update() {
    const activeView = appState.getActiveView();
    const currentBreadcrumb = this.container.querySelector('.breadcrumb-current') as HTMLElement;
    const backBtn = this.container.querySelector('.nav-back-btn') as HTMLElement;
    const searchInput = this.container.querySelector('.search-input') as HTMLInputElement;

    if (!currentBreadcrumb || !backBtn) return;

    switch (activeView.type) {
      case 'albums':
        currentBreadcrumb.textContent = 'Albums';
        backBtn.style.display = 'none';
        if (searchInput.value) searchInput.value = '';
        break;
      case 'artists':
        currentBreadcrumb.textContent = 'Artists';
        backBtn.style.display = 'none';
        if (searchInput.value) searchInput.value = '';
        break;
      case 'tracks':
        currentBreadcrumb.textContent = 'Tracks';
        backBtn.style.display = 'none';
        if (searchInput.value) searchInput.value = '';
        break;
      case 'album_detail':
        currentBreadcrumb.textContent = activeView.albumTitle;
        backBtn.style.display = 'inline-flex';
        break;
      case 'artist_detail':
        currentBreadcrumb.textContent = activeView.artistName;
        backBtn.style.display = 'inline-flex';
        break;
      case 'search':
        currentBreadcrumb.textContent = `Search: "${activeView.query}"`;
        backBtn.style.display = 'none';
        break;
    }
  }

  public focusSearch() {
    const searchInput = this.container.querySelector('.search-input') as HTMLInputElement;
    if (searchInput) {
      searchInput.focus();
      searchInput.select();
    }
  }
}
