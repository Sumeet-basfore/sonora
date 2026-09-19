import './tokens.css';
import './style.css';

import { HeaderComponent } from './components/Header';
import { LibraryViewComponent } from './components/LibraryView';
import { LyricsViewComponent } from './components/LyricsView';
import { MarketplaceViewComponent } from './components/MarketplaceView';
import { NowPlayingStageComponent } from './components/NowPlayingStage';
import { PlaybackBarComponent } from './components/PlaybackBar';
import { QueueDrawerComponent } from './components/QueueDrawer';
import { ScanModalComponent } from './components/ScanModal';
import { SettingsModalComponent } from './components/SettingsModal';
import { SidebarComponent } from './components/Sidebar';
import { VisualizerComponent } from './components/Visualizer';
import {
  themeEngine,
  layoutManager,
  artworkStyleManager,
} from './customization';
import { setupKeyboardShortcuts } from './shortcuts';
import { appState } from './state';
import { syncMarketplaceThemesFromBackend } from './marketplaceThemes';

let isAppInitialized = false;

function initializeApp() {
  if (isAppInitialized) return;
  isAppInitialized = true;

  const root = document.querySelector<HTMLDivElement>('#app');
  if (!root) return;

  root.innerHTML = `
    <div class="app-layout">
      <!-- Left Sidebar Navigation -->
      <aside class="app-sidebar" id="sidebar-container"></aside>

      <!-- Central Content Viewport -->
      <div class="app-viewport" id="viewport-container">
        <!-- Top App Header -->
        <header class="app-header" id="header-container"></header>

        <!-- Main Dynamic Viewport -->
        <main class="app-main" id="main-container" tabindex="-1"></main>
      </div>

      <!-- Right Now Playing & Context Stage -->
      <aside class="now-playing-stage" id="now-playing-container"></aside>

      <!-- Slide-out Active Queue Drawer -->
      <aside class="queue-drawer" id="queue-container"></aside>

      <!-- Synchronized Lyrics Drawer / Inspector -->
      <aside class="lyrics-drawer" id="lyrics-container" style="display: none;"></aside>

      <!-- Persistent Playback Transport Bar -->
      <footer class="playback-bar" id="playback-container"></footer>

      <!-- Floating / Docked Visualizer Panel -->
      <div class="visualizer-panel" id="visualizer-container" style="display: none;"></div>

      <!-- Scan Directory Modal Dialog -->
      <div class="modal-root" id="scan-modal-container"></div>

      <!-- Customization & Settings Modal Dialog -->
      <div class="modal-root" id="settings-modal-container"></div>
    </div>
  `;

  // Instantiate Components
  const sidebarContainer = document.querySelector('#sidebar-container') as HTMLElement;
  const headerContainer = document.querySelector('#header-container') as HTMLElement;
  const mainContainer = document.querySelector('#main-container') as HTMLElement;
  const nowPlayingContainer = document.querySelector('#now-playing-container') as HTMLElement;
  const queueContainer = document.querySelector('#queue-container') as HTMLElement;
  const lyricsContainer = document.querySelector('#lyrics-container') as HTMLElement;
  const playbackContainer = document.querySelector('#playback-container') as HTMLElement;
  const visualizerContainer = document.querySelector('#visualizer-container') as HTMLElement;
  const scanModalContainer = document.querySelector('#scan-modal-container') as HTMLElement;
  const settingsModalContainer = document.querySelector('#settings-modal-container') as HTMLElement;

  const sidebar = new SidebarComponent(sidebarContainer);
  const header = new HeaderComponent(headerContainer);
  new LibraryViewComponent(mainContainer);
  new MarketplaceViewComponent(mainContainer);
  new NowPlayingStageComponent(nowPlayingContainer);
  new PlaybackBarComponent(playbackContainer);
  new QueueDrawerComponent(queueContainer);
  new LyricsViewComponent(lyricsContainer);
  const visualizer = new VisualizerComponent(visualizerContainer);
  new ScanModalComponent(scanModalContainer);
  const settingsModal = new SettingsModalComponent(settingsModalContainer);

  // Setup Keyboard Navigation & Global Shortcuts
  setupKeyboardShortcuts(() => {
    header.focusSearch();
  });

  // Global settings open/close events
  window.addEventListener('sonora-open-settings', () => {
    settingsModal.open();
  });

  window.addEventListener('sonora-close-settings', () => {
    if (settingsModal.isVisible()) {
      settingsModal.close();
    }
  });

  window.addEventListener('sonora-toggle-lyrics', () => {
    layoutManager.toggleRegion('lyrics');
  });

  // Apply layout regions visibility
  const syncLayoutRegions = () => {
    const regions = layoutManager.getRegions();
    sidebarContainer.style.display = regions.sidebar ? 'flex' : 'none';
    mainContainer.style.display = regions.library ? 'block' : 'none';
    nowPlayingContainer.style.display = regions.nowPlaying ? 'flex' : 'none';
    queueContainer.style.display = regions.queue || appState.isQueueVisible() ? 'flex' : 'none';
    playbackContainer.style.display = regions.playbackBar ? 'flex' : 'none';
    lyricsContainer.style.display = regions.lyrics ? 'flex' : 'none';

    const showVis = regions.visualizer || appState.isVisualizerVisible();
    if (showVis) {
      visualizerContainer.style.display = 'block';
      visualizer.start();
    } else {
      visualizerContainer.style.display = 'none';
      visualizer.stop();
    }

    layoutManager.applyLayoutToDom();
  };

  layoutManager.subscribe(() => {
    syncLayoutRegions();
  });

  appState.subscribe(() => {
    syncLayoutRegions();
  });

  // Initial synchronization of DOM with customization engines
  themeEngine.applyThemeToDom();
  artworkStyleManager.applyStyleToDom();
  syncLayoutRegions();

  // Register marketplace-installed themes (validated backend-side and again
  // in the engine); a persisted backend selection wins over local state.
  void syncMarketplaceThemesFromBackend().catch((e) => {
    console.warn('Marketplace theme sync failed:', e);
  });

  // Listen for library updates to update counts (avoiding 4Hz polling overhead during playback)
  window.addEventListener('sonora-library-updated', () => {
    sidebar.refresh();
  });

  // Support direct URL query parameters for deep linking, presets, and inspection
  const searchParams = new URLSearchParams(window.location.search);
  const layoutParam = searchParams.get('layout');
  const viewParam = searchParams.get('view');
  const playParam = searchParams.get('play') === '1';
  const queueParam = searchParams.get('open_queue') === '1';
  const lyricsParam = searchParams.get('open_lyrics') === '1';
  const settingsParam = searchParams.get('open_settings') === '1';

  if (layoutParam) {
    if (layoutParam === 'modern_curator' || layoutParam === 'layout-default-studio') {
      layoutManager.setLayout('layout-default-studio');
    } else if (layoutParam === 'audiophile_studio' || layoutParam === 'layout-audiophile-deck') {
      layoutManager.setLayout('layout-audiophile-deck');
    } else if (layoutParam === 'atmospheric_theater' || layoutParam === 'layout-lyrics-stage') {
      layoutManager.setLayout('layout-lyrics-stage');
    } else if (layoutParam === 'minimal' || layoutParam === 'layout-minimal-player') {
      layoutManager.setLayout('layout-minimal-player');
    } else {
      layoutManager.setLayout(layoutParam);
    }
  }
  if (viewParam === 'albums') {
    appState.setActiveView({ type: 'albums' });
  } else if (viewParam === 'tracks') {
    appState.setActiveView({ type: 'tracks' });
  } else if (viewParam === 'album_detail') {
    appState.setActiveView({
      type: 'album_detail',
      albumId: 1,
      albumTitle: 'Random Access Memories',
      artistName: 'Daft Punk',
    });
  }
  if (playParam) {
    void appState.playTrack(1);
  }
  if (queueParam) {
    appState.toggleQueue(true);
  }
  if (lyricsParam) {
    layoutManager.setRegionVisible('lyrics', true);
  }
  if (settingsParam) {
    settingsModal.open();
  }
}

if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', initializeApp);
} else {
  initializeApp();
}
