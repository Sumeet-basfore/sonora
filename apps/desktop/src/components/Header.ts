import { appState } from '../state';
import { layoutManager } from '../customization';

export class HeaderComponent {
  private container: HTMLElement;
  private searchDebounceTimer: number | null = null;

  constructor(container: HTMLElement) {
    this.container = container;
    this.render();
    appState.subscribe(() => this.update());
    layoutManager.subscribe(() => this.updateWorkspaceSelector());
  }

  private render() {
    const activeLayout = layoutManager.getActiveLayout();
    const availableLayouts = layoutManager.getAvailableLayouts();

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
          <!-- Workspace Preset Switcher -->
          <div class="workspace-switcher-wrapper" title="Switch Workspace Layout">
            <svg class="workspace-icon" width="15" height="15" viewBox="0 0 24 24" fill="currentColor">
              <path d="M4 4h7v7H4V4zm9 0h7v7h-7V4zm-9 9h7v7H4v-7zm9 0h7v7h-7v-7z"/>
            </svg>
            <select class="workspace-select-pill" id="header-workspace-select" aria-label="Workspace Preset">
              ${availableLayouts
                .map(
                  (l) => `<option value="${l.id}" ${l.id === activeLayout.id ? 'selected' : ''}>${l.name}</option>`
                )
                .join('')}
            </select>
          </div>

          <button class="button button-secondary scan-dir-btn" title="Scan local music folder">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
              <path d="M10 4H4c-1.1 0-1.99.9-1.99 2L2 18c0 1.1.9 2 2 2h16c1.1 0 2-.9 2-2V8c0-1.1-.9-2-2-2h-8l-2-2z"/>
            </svg>
            <span>Scan Music</span>
          </button>

          <button class="icon-button header-settings-btn" title="Settings & Customization (,)" aria-label="Settings">
            <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor">
              <path d="M19.14 12.94c.04-.3.06-.61.06-.94 0-.32-.02-.64-.07-.94l2.03-1.58c.18-.14.23-.41.12-.61l-1.92-3.32c-.12-.22-.37-.29-.59-.22l-2.39.96c-.5-.38-1.03-.7-1.62-.94l-.36-2.54c-.04-.24-.24-.41-.48-.41h-3.84c-.24 0-.43.17-.47.41l-.36 2.54c-.59.24-1.13.57-1.62.94l-2.39-.96c-.22-.08-.47 0-.59.22L2.74 8.87c-.12.21-.08.47.12.61l2.03 1.58c-.05.3-.09.63-.09.94s.02.64.07.94l-2.03 1.58c-.18.14-.23.41-.12.61l1.92 3.32c.12.22.37.29.59.22l2.39-.96c.5.38 1.03.7 1.62.94l.36 2.54c.05.24.24.41.48.41h3.84c.24 0 .44-.17.47-.41l.36-2.54c.59-.24 1.13-.56 1.62-.94l2.39.96c.22.08.47 0 .59-.22l1.92-3.32c.12-.22.07-.47-.12-.61l-2.01-1.58zM12 15.6c-1.98 0-3.6-1.62-3.6-3.6s1.62-3.6 3.6-3.6 3.6 1.62 3.6 3.6-1.62 3.6-3.6 3.6z"/>
            </svg>
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
    const workspaceSelect = this.container.querySelector('#header-workspace-select') as HTMLSelectElement;
    const settingsBtn = this.container.querySelector('.header-settings-btn') as HTMLButtonElement;

    workspaceSelect?.addEventListener('change', () => {
      layoutManager.setLayout(workspaceSelect.value);
    });

    settingsBtn?.addEventListener('click', () => {
      window.dispatchEvent(new CustomEvent('sonora-open-settings'));
    });

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

  private updateWorkspaceSelector() {
    const activeLayout = layoutManager.getActiveLayout();
    const workspaceSelect = this.container.querySelector('#header-workspace-select') as HTMLSelectElement;
    if (workspaceSelect && workspaceSelect.value !== activeLayout.id) {
      workspaceSelect.value = activeLayout.id;
    }
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
