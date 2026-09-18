import { api } from '../api';
import { appState } from '../state';
import type { LibrarySummary } from '../types';

export class SidebarComponent {
  private container: HTMLElement;
  private summary: LibrarySummary = { track_count: 0, album_count: 0, artist_count: 0 };
  private systemStatus: string = 'Core Ready';

  constructor(container: HTMLElement) {
    this.container = container;
    this.render();
    this.loadData();
    appState.subscribe(() => this.updateActive());
  }

  private async loadData() {
    try {
      this.summary = await api.getLibrarySummary();
      this.systemStatus = await api.getSystemStatus();
      this.updateCounts();
    } catch (e) {
      console.warn('Failed to load library summary:', e);
    }
  }

  private render() {
    this.container.innerHTML = `
      <div class="sidebar-inner">
        <!-- Brand Header -->
        <div class="brand-header">
          <div class="brand-logo-icon">
            <svg width="22" height="22" viewBox="0 0 24 24" fill="none">
              <circle cx="12" cy="12" r="10" stroke="currentColor" stroke-width="2"/>
              <circle cx="12" cy="12" r="4" fill="currentColor"/>
              <path d="M12 2v4M12 18v4M2 12h4M18 12h4" stroke="currentColor" stroke-width="2"/>
            </svg>
          </div>
          <div class="brand-title-group">
            <span class="brand-name">Sonora</span>
            <span class="brand-version-badge">v0.1</span>
          </div>
        </div>

        <!-- Navigation Menu -->
        <nav class="sidebar-nav" aria-label="Library Navigation">
          <div class="nav-section-label">LIBRARY</div>
          <button class="nav-item nav-albums active" data-view="albums">
            <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor">
              <path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm0 14.5c-2.49 0-4.5-2.01-4.5-4.5S9.51 7.5 12 7.5s4.5 2.01 4.5 4.5-2.01 4.5-4.5 4.5zm0-5.5c-.55 0-1 .45-1 1s.45 1 1 1 1-.45 1-1-.45-1-1-1z"/>
            </svg>
            <span>Albums</span>
          </button>
          <button class="nav-item nav-artists" data-view="artists">
            <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor">
              <path d="M12 12c2.21 0 4-1.79 4-4s-1.79-4-4-4-4 1.79-4 4 1.79 4 4 4zm0 2c-2.67 0-8 1.34-8 4v2h16v-2c0-2.66-5.33-4-8-4z"/>
            </svg>
            <span>Artists</span>
          </button>
          <button class="nav-item nav-tracks" data-view="tracks">
            <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor">
              <path d="M12 3v10.55c-.59-.34-1.27-.55-2-.55-2.21 0-4 1.79-4 4s1.79 4 4 4 4-1.79 4-4V7h4V3h-6z"/>
            </svg>
            <span>Tracks</span>
          </button>
          <button class="nav-item nav-extensions" data-view="marketplace">
            <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor">
              <path d="M20.5 11H19V7c0-1.1-.9-2-2-2h-4V3.5C13 2.12 11.88 1 10.5 1S8 2.12 8 3.5V5H4c-1.1 0-1.99.9-1.99 2v3.8H1.5c-.83 0-1.5.67-1.5 1.5v4c0 1.1.9 2 2 2h4.8c.41 1.16 1.52 2 2.7 2h5c1.38 0 2.5-1.12 2.5-2.5V11h3.5c.83 0 1.5-.67 1.5-1.5v-1c0-.28-.22-.5-.5-.5z"/>
            </svg>
            <span>Extensions</span>
          </button>

          <div class="nav-section-label" style="margin-top: 16px;">PREFERENCES</div>
          <button class="nav-item nav-settings" data-action="settings">
            <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor">
              <path d="M19.14 12.94c.04-.3.06-.61.06-.94 0-.32-.02-.64-.07-.94l2.03-1.58c.18-.14.23-.41.12-.61l-1.92-3.32c-.12-.22-.37-.29-.59-.22l-2.39.96c-.5-.38-1.03-.7-1.62-.94l-.36-2.54c-.04-.24-.24-.41-.48-.41h-3.84c-.24 0-.43.17-.47.41l-.36 2.54c-.59.24-1.13.57-1.62.94l-2.39-.96c-.22-.08-.47 0-.59.22L2.74 8.87c-.12.21-.08.47.12.61l2.03 1.58c-.05.3-.09.63-.09.94s.02.64.07.94l-2.03 1.58c-.18.14-.23.41-.12.61l1.92 3.32c.12.22.37.29.59.22l2.39-.96c.5.38 1.03.7 1.62.94l.36 2.54c.05.24.24.41.48.41h3.84c.24 0 .44-.17.47-.41l.36-2.54c.59-.24 1.13-.56 1.62-.94l2.39.96c.22.08.47 0 .59-.22l1.92-3.32c.12-.22.07-.47-.12-.61l-2.01-1.58zM12 15.6c-1.98 0-3.6-1.62-3.6-3.6s1.62-3.6 3.6-3.6 3.6 1.62 3.6 3.6-1.62 3.6-3.6 3.6z"/>
            </svg>
            <span>Customization</span>
          </button>
        </nav>

        <!-- Library Stats Footer -->
        <div class="sidebar-footer">
          <div class="stats-card">
            <div class="stats-label">COLLECTION</div>
            <div class="stats-details">
              <span class="stat-albums-count">0 Albums</span> • 
              <span class="stat-tracks-count">0 Tracks</span>
            </div>
            <div class="system-status-indicator" title="${this.systemStatus}">
              <span class="status-dot"></span>
              <span class="status-text">${this.systemStatus}</span>
            </div>
          </div>
        </div>
      </div>
    `;

    this.attachEventListeners();
  }

  private attachEventListeners() {
    const navButtons = this.container.querySelectorAll('.nav-item');
    navButtons.forEach((btn) => {
      btn.addEventListener('click', () => {
        const action = btn.getAttribute('data-action');
        if (action === 'settings') {
          window.dispatchEvent(new CustomEvent('sonora-open-settings'));
          return;
        }

        const viewType = btn.getAttribute('data-view');
        if (viewType === 'albums') appState.setActiveView({ type: 'albums' });
        else if (viewType === 'artists') appState.setActiveView({ type: 'artists' });
        else if (viewType === 'tracks') appState.setActiveView({ type: 'tracks' });
        else if (viewType === 'marketplace') appState.setActiveView({ type: 'marketplace' });
      });
    });
  }

  public updateCounts() {
    const albumsEl = this.container.querySelector('.stat-albums-count');
    const tracksEl = this.container.querySelector('.stat-tracks-count');
    if (albumsEl) albumsEl.textContent = `${this.summary.album_count} Albums`;
    if (tracksEl) tracksEl.textContent = `${this.summary.track_count} Tracks`;
  }

  public refresh() {
    this.loadData();
  }

  private updateActive() {
    const activeView = appState.getActiveView();
    const navItems = this.container.querySelectorAll('.nav-item');
    navItems.forEach((item) => {
      const view = item.getAttribute('data-view');
      if (view === activeView.type) {
        item.classList.add('active');
      } else {
        item.classList.remove('active');
      }
    });
  }
}
