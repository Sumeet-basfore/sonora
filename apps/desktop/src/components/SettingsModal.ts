/**
 * Sonora Customization & Settings Modal Component.
 * Provides coherent, live-preview controls for Themes, Layouts,
 * Artwork Presentation Styles, and Audio Visualizer parameters.
 */

import {
  themeEngine,
  layoutManager,
  artworkStyleManager,
  visualizerConfigManager,
  ACCENT_COLOR_PRESETS,
  ALL_REGION_IDS,
  RegionId,
  AlbumArtStyleId,
  VisualizerStyleId,
  VisualizerColorSource,
} from '../customization';
import { lyricsManager } from '../lyricsManager.ts';
import { api } from '../api';
import { escapeHtml } from '../escape';
import type { InstalledTheme } from '../types';
import {
  applyMarketplaceTheme,
  cancelMarketplacePreview,
  previewMarketplaceTheme,
  removeMarketplaceTheme,
  resetToBuiltInTheme,
  syncMarketplaceThemesFromBackend,
} from '../marketplaceThemes';

export type SettingsTab = 'theme' | 'layout' | 'artwork' | 'visualizer' | 'lyrics';

export class SettingsModalComponent {
  private container: HTMLElement;
  private isOpen: boolean = false;
  private activeTab: SettingsTab = 'theme';
  private marketplaceThemes: InstalledTheme[] = [];
  private marketplaceStatus: string | null = null;
  private previewThemeId: string | null = null;

  constructor(container: HTMLElement) {
    this.container = container;
    this.render();
    this.attachEventListeners();
  }

  public open(tab: SettingsTab = 'theme') {
    this.isOpen = true;
    this.activeTab = tab;
    this.render();
    this.attachEventListeners();
    // Pull marketplace themes in the background; re-render when ready.
    void syncMarketplaceThemesFromBackend()
      .then(() => api.marketplaceThemes())
      .then((themes) => {
        if (!this.isOpen) return;
        this.marketplaceThemes = themes;
        this.render();
        this.attachEventListeners();
      })
      .catch((e) => {
        if (!this.isOpen) return;
        this.marketplaceStatus = `Marketplace themes unavailable: ${String(e)}`;
        this.render();
        this.attachEventListeners();
      });
  }

  public close() {
    this.isOpen = false;
    this.render();
  }

  public isVisible(): boolean {
    return this.isOpen;
  }

  private render() {
    if (!this.isOpen) {
      this.container.innerHTML = '';
      return;
    }

    const currentTheme = themeEngine.getActiveTheme();
    const availableThemes = themeEngine.getAvailableThemes();
    const customAccent = themeEngine.getCustomAccent();
    const availableLayouts = layoutManager.getAvailableLayouts();
    const activeLayout = layoutManager.getActiveLayout();
    const currentRegions = layoutManager.getRegions();
    const activeArtStyle = artworkStyleManager.getActiveStyle();
    const availableArtStyles = artworkStyleManager.getAvailableStyles();
    const visConfig = visualizerConfigManager.getConfig();
    const lyricsPrefs = lyricsManager.getPreferences();

    this.container.innerHTML = `
      <div class="settings-modal-backdrop" role="dialog" aria-modal="true" aria-labelledby="settings-modal-title">
        <div class="settings-modal-dialog">
          <!-- Header -->
          <div class="settings-modal-header">
            <div class="settings-title-group">
              <svg width="22" height="22" viewBox="0 0 24 24" fill="currentColor">
                <path d="M19.14 12.94c.04-.3.06-.61.06-.94 0-.32-.02-.64-.07-.94l2.03-1.58c.18-.14.23-.41.12-.61l-1.92-3.32c-.12-.22-.37-.29-.59-.22l-2.39.96c-.5-.38-1.03-.7-1.62-.94l-.36-2.54c-.04-.24-.24-.41-.48-.41h-3.84c-.24 0-.43.17-.47.41l-.36 2.54c-.59.24-1.13.57-1.62.94l-2.39-.96c-.22-.08-.47 0-.59.22L2.74 8.87c-.12.21-.08.47.12.61l2.03 1.58c-.05.3-.09.63-.09.94s.02.64.07.94l-2.03 1.58c-.18.14-.23.41-.12.61l1.92 3.32c.12.22.37.29.59.22l2.39-.96c.5.38 1.03.7 1.62.94l.36 2.54c.05.24.24.41.48.41h3.84c.24 0 .44-.17.47-.41l.36-2.54c.59-.24 1.13-.56 1.62-.94l2.39.96c.22.08.47 0 .59-.22l1.92-3.32c.12-.22.07-.47-.12-.61l-2.01-1.58zM12 15.6c-1.98 0-3.6-1.62-3.6-3.6s1.62-3.6 3.6-3.6 3.6 1.62 3.6 3.6-1.62 3.6-3.6 3.6z"/>
              </svg>
              <h2 id="settings-modal-title">Customization & Settings</h2>
            </div>
            <button class="icon-button close-settings-btn" title="Close settings (Esc)" aria-label="Close settings">✕</button>
          </div>

          <!-- Navigation Tabs -->
          <div class="settings-tabs-nav" role="tablist">
            <button class="settings-tab-btn ${this.activeTab === 'theme' ? 'active' : ''}" data-tab="theme" role="tab" aria-selected="${this.activeTab === 'theme'}">
              🎨 Themes & Colors
            </button>
            <button class="settings-tab-btn ${this.activeTab === 'layout' ? 'active' : ''}" data-tab="layout" role="tab" aria-selected="${this.activeTab === 'layout'}">
              📐 Workspace Layout
            </button>
            <button class="settings-tab-btn ${this.activeTab === 'artwork' ? 'active' : ''}" data-tab="artwork" role="tab" aria-selected="${this.activeTab === 'artwork'}">
              🖼️ Artwork Styles
            </button>
            <button class="settings-tab-btn ${this.activeTab === 'visualizer' ? 'active' : ''}" data-tab="visualizer" role="tab" aria-selected="${this.activeTab === 'visualizer'}">
              📊 Visualizer
            </button>
            <button class="settings-tab-btn ${this.activeTab === 'lyrics' ? 'active' : ''}" data-tab="lyrics" role="tab" aria-selected="${this.activeTab === 'lyrics'}">
              🎤 Lyrics
            </button>
          </div>

          <!-- Tab Panels -->
          <div class="settings-tab-content">
            <!-- TAB 1: THEMES -->
            <div class="settings-panel ${this.activeTab === 'theme' ? 'active' : ''}" id="tab-panel-theme">
              <div class="settings-section">
                <div class="section-header-row">
                  <div>
                    <h3 class="settings-section-title">Theme Presets</h3>
                    <p class="settings-section-desc">Select an engineered theme palette. Changes apply immediately.</p>
                  </div>
                  <div class="theme-io-actions">
                    <button class="button button-secondary export-theme-btn" title="Export current theme as JSON file">Export JSON</button>
                    <button class="button button-secondary import-theme-btn" title="Import a theme JSON file">Import JSON</button>
                    <input type="file" class="import-theme-file-input" accept=".json,application/json" style="display: none;" />
                  </div>
                </div>

                <div class="theme-grid">
                  ${availableThemes
                    .map(
                      (theme) => `
                    <div class="theme-card ${theme.id === currentTheme.id ? 'active' : ''}" data-theme-id="${theme.id}">
                      <div class="theme-preview-palette" style="background: ${theme.tokens['--bg-surface']}; border-color: ${theme.tokens['--border-subtle']};">
                        <span class="preview-base" style="background: ${theme.tokens['--bg-base']}"></span>
                        <span class="preview-accent" style="background: ${theme.tokens['--accent-primary']}"></span>
                        <span class="preview-secondary" style="background: ${theme.tokens['--accent-secondary']}"></span>
                        <span class="preview-text" style="background: ${theme.tokens['--text-primary']}"></span>
                      </div>
                      <div class="theme-info">
                        <div class="theme-card-name">${escapeHtml(theme.name)}</div>
                        <div class="theme-card-badge">${escapeHtml(theme.mode.toUpperCase())}</div>
                      </div>
                      ${theme.id === currentTheme.id ? '<span class="theme-active-indicator">✓</span>' : ''}
                    </div>
                  `
                    )
                    .join('')}
                </div>
              </div>

              <div class="settings-section">
                <h3 class="settings-section-title">Marketplace Themes</h3>
                <p class="settings-section-desc">Themes installed from Extensions. Preview applies instantly without saving; Apply persists your choice.</p>
                ${this.marketplaceStatus ? `<p class="settings-section-desc">${escapeHtml(this.marketplaceStatus)}</p>` : ''}
                ${
                  this.marketplaceThemes.length
                    ? `
                <div class="marketplace-theme-list">
                  ${this.marketplaceThemes
                    .map(
                      (theme) => `
                    <div class="marketplace-theme-row" data-theme-id="${theme.id}">
                      <div class="marketplace-theme-info">
                        <div class="theme-card-name">${escapeHtml(theme.name)} <span class="theme-card-badge">${escapeHtml(theme.mode.toUpperCase())} · v${escapeHtml(theme.version)}</span></div>
                        ${theme.id === currentTheme.id ? '<span class="theme-active-indicator">✓ active</span>' : ''}
                        ${this.previewThemeId === theme.id ? '<span class="theme-active-indicator">previewing…</span>' : ''}
                      </div>
                      <div class="marketplace-theme-actions">
                        <button class="button button-secondary mp-preview-btn" data-id="${theme.id}">Preview</button>
                        <button class="button button-secondary mp-apply-btn" data-id="${theme.id}">Apply</button>
                        <button class="button button-text mp-remove-btn" data-id="${theme.id}">Remove</button>
                      </div>
                    </div>
                  `
                    )
                    .join('')}
                </div>
                `
                    : '<p class="settings-section-desc">No marketplace themes installed. Find some under Extensions in the sidebar.</p>'
                }
                <div class="theme-io-actions">
                  ${this.previewThemeId ? '<button class="button button-secondary mp-cancel-preview-btn">Cancel preview</button>' : ''}
                  <button class="button button-secondary mp-reset-builtin-btn">Reset to built-in theme</button>
                </div>
              </div>

              <div class="settings-section">
                <h3 class="settings-section-title">Accent Color Override</h3>
                <p class="settings-section-desc">Customize interactive buttons, sliders, and highlights across the UI.</p>
                <div class="accent-palette-row">
                  ${ACCENT_COLOR_PRESETS.map(
                    (preset) => `
                    <button 
                      class="accent-preset-chip ${customAccent === preset.hex || (!customAccent && currentTheme.tokens['--accent-primary'] === preset.hex) ? 'selected' : ''}" 
                      data-hex="${preset.hex}" 
                      title="${preset.name}"
                      style="background-color: ${preset.hex};">
                    </button>
                  `
                  ).join('')}

                  <label class="custom-color-picker-label" title="Custom Hex Color">
                    <span class="color-picker-swatch" style="background-color: ${customAccent || currentTheme.tokens['--accent-primary']};"></span>
                    <input type="color" class="custom-accent-input" value="${customAccent || currentTheme.tokens['--accent-primary']}" />
                    <span class="color-picker-text">Custom...</span>
                  </label>

                  ${
                    customAccent
                      ? `<button class="button button-text clear-accent-btn" title="Revert to theme default accent">Clear Override</button>`
                      : ''
                  }
                </div>
              </div>
            </div>

            <!-- TAB 2: LAYOUT -->
            <div class="settings-panel ${this.activeTab === 'layout' ? 'active' : ''}" id="tab-panel-layout">
              <div class="settings-section">
                <div class="section-header-row">
                  <div>
                    <h3 class="settings-section-title">Saved Workspaces</h3>
                    <p class="settings-section-desc">Switch between curated window layouts or save your own.</p>
                  </div>
                  <div class="layout-actions">
                    <button class="button button-secondary save-custom-layout-btn">Save Current As...</button>
                  </div>
                </div>

                <div class="layout-preset-selector">
                  <select class="layout-select" aria-label="Select saved layout">
                    ${availableLayouts
                      .map(
                        (l) => `
                      <option value="${l.id}" ${l.id === activeLayout.id ? 'selected' : ''}>
                        ${l.name} ${l.isBuiltIn ? '(Built-in)' : '(Custom)'}
                      </option>
                    `
                      )
                      .join('')}
                  </select>
                  ${
                    !activeLayout.isBuiltIn
                      ? `<button class="button button-danger delete-layout-btn" title="Delete custom layout">Delete Layout</button>`
                      : ''
                  }
                </div>
              </div>

              <div class="settings-section">
                <h3 class="settings-section-title">Component Regions</h3>
                <p class="settings-section-desc">Show or hide modular UI panels. Toggles take effect instantly.</p>
                <div class="region-toggle-grid">
                  <label class="region-toggle-item">
                    <input type="checkbox" data-region="sidebar" ${currentRegions.sidebar ? 'checked' : ''} />
                    <div class="region-toggle-meta">
                      <span class="region-name">Sidebar Navigation</span>
                      <span class="region-desc">Left rail with Albums, Artists, Tracks navigation</span>
                    </div>
                  </label>
                  <label class="region-toggle-item">
                    <input type="checkbox" data-region="library" ${currentRegions.library ? 'checked' : ''} />
                    <div class="region-toggle-meta">
                      <span class="region-name">Library Viewport</span>
                      <span class="region-desc">Central grid and list views of your music library</span>
                    </div>
                  </label>
                  <label class="region-toggle-item">
                    <input type="checkbox" data-region="playbackBar" ${currentRegions.playbackBar ? 'checked' : ''} />
                    <div class="region-toggle-meta">
                      <span class="region-name">Playback Transport Bar</span>
                      <span class="region-desc">Bottom transport bar with seeker, volume, and controls</span>
                    </div>
                  </label>
                  <label class="region-toggle-item">
                    <input type="checkbox" data-region="queue" ${currentRegions.queue ? 'checked' : ''} />
                    <div class="region-toggle-meta">
                      <span class="region-name">Queue Drawer</span>
                      <span class="region-desc">Slide-out panel showing upcoming tracks</span>
                    </div>
                  </label>
                  <label class="region-toggle-item">
                    <input type="checkbox" data-region="visualizer" ${currentRegions.visualizer ? 'checked' : ''} />
                    <div class="region-toggle-meta">
                      <span class="region-name">Audio Visualizer</span>
                      <span class="region-desc">Real-time spectrum analysis and audio waveforms</span>
                    </div>
                  </label>
                  <label class="region-toggle-item">
                    <input type="checkbox" data-region="lyrics" ${currentRegions.lyrics ? 'checked' : ''} />
                    <div class="region-toggle-meta">
                      <span class="region-name">Synchronized Lyrics</span>
                      <span class="region-desc">Real-time synced lyrics line inspector</span>
                    </div>
                  </label>
                </div>
              </div>
            </div>

            <!-- TAB 3: ARTWORK -->
            <div class="settings-panel ${this.activeTab === 'artwork' ? 'active' : ''}" id="tab-panel-artwork">
              <div class="settings-section">
                <h3 class="settings-section-title">Album Artwork Presentation</h3>
                <p class="settings-section-desc">Select how album covers and gatefolds are styled throughout the application.</p>
                <div class="art-style-grid">
                  ${availableArtStyles
                    .map(
                      (style) => `
                    <div class="art-style-card ${style.id === activeArtStyle ? 'active' : ''}" data-style-id="${style.id}">
                      <div class="art-style-preview" data-preview-style="${style.id}">
                        <div class="preview-mini-cover">
                          <span class="preview-vinyl-disc"></span>
                          <span class="preview-ambient-glow"></span>
                          <span class="preview-art-icon">🎵</span>
                        </div>
                      </div>
                      <div class="art-style-meta">
                        <div class="art-style-title">${style.name}</div>
                        <div class="art-style-desc">${style.description}</div>
                      </div>
                      ${style.id === activeArtStyle ? '<span class="style-active-badge">✓ Active</span>' : ''}
                    </div>
                  `
                    )
                    .join('')}
                </div>
              </div>
            </div>

            <!-- TAB 4: VISUALIZER -->
            <div class="settings-panel ${this.activeTab === 'visualizer' ? 'active' : ''}" id="tab-panel-visualizer">
              <div class="settings-section">
                <div class="section-header-row">
                  <div>
                    <h3 class="settings-section-title">Audio Visualizer Engine</h3>
                    <p class="settings-section-desc">Configure the real-time FFT spectrum analyzer and oscilloscope physics.</p>
                  </div>
                  <label class="switch-toggle" title="Master toggle for visualizer">
                    <input type="checkbox" class="vis-enable-toggle" ${visConfig.enabled ? 'checked' : ''} />
                    <span class="switch-slider"></span>
                  </label>
                </div>

                <div class="vis-controls-grid">
                  <!-- Visualizer Style -->
                  <div class="setting-control-row">
                    <label class="setting-label">Visualizer Mode</label>
                    <select class="setting-select vis-style-select">
                      <option value="bars" ${visConfig.style === 'bars' ? 'selected' : ''}>Spectrum Bars (48-Band FFT)</option>
                      <option value="wave" ${visConfig.style === 'wave' ? 'selected' : ''}>Oscilloscope Waveform</option>
                      <option value="mirror" ${visConfig.style === 'mirror' ? 'selected' : ''}>Bilateral Mirrored Spectrum</option>
                      <option value="led" ${visConfig.style === 'led' ? 'selected' : ''}>Retro LED Meter Blocks</option>
                    </select>
                  </div>

                  <!-- Color Source -->
                  <div class="setting-control-row">
                    <label class="setting-label">Color Source</label>
                    <select class="setting-select vis-color-select">
                      <option value="accent" ${visConfig.colorSource === 'accent' ? 'selected' : ''}>Theme Accent Primary</option>
                      <option value="gradient" ${visConfig.colorSource === 'gradient' ? 'selected' : ''}>Vibrant Rainbow Gradient</option>
                      <option value="monochrome" ${visConfig.colorSource === 'monochrome' ? 'selected' : ''}>Monochrome Silver</option>
                    </select>
                  </div>

                  <!-- Sensitivity Gain -->
                  <div class="setting-control-row">
                    <div class="setting-label-group">
                      <label class="setting-label">Gain Sensitivity</label>
                      <span class="setting-value vis-sens-val">${visConfig.sensitivity.toFixed(1)}x</span>
                    </div>
                    <input type="range" class="slider-input vis-sens-slider" min="0.2" max="3.0" step="0.1" value="${visConfig.sensitivity}" />
                  </div>

                  <!-- Temporal Smoothing -->
                  <div class="setting-control-row">
                    <div class="setting-label-group">
                      <label class="setting-label">Decay Smoothing (tau)</label>
                      <span class="setting-value vis-smooth-val">${(visConfig.smoothing * 1000).toFixed(0)} ms</span>
                    </div>
                    <input type="range" class="slider-input vis-smooth-slider" min="0.02" max="0.40" step="0.01" value="${visConfig.smoothing}" />
                  </div>

                  <!-- Panel Height -->
                  <div class="setting-control-row">
                    <div class="setting-label-group">
                      <label class="setting-label">Panel Height</label>
                      <span class="setting-value vis-height-val">${visConfig.height} px</span>
                    </div>
                    <input type="range" class="slider-input vis-height-slider" min="70" max="280" step="10" value="${visConfig.height}" />
                  </div>

                  <!-- Max FPS -->
                  <div class="setting-control-row">
                    <label class="setting-label">Frame Rate Cap</label>
                    <select class="setting-select vis-fps-select">
                      <option value="30" ${visConfig.fps === 30 ? 'selected' : ''}>30 FPS (Power Saver)</option>
                      <option value="60" ${visConfig.fps === 60 ? 'selected' : ''}>60 FPS (Balanced)</option>
                      <option value="120" ${visConfig.fps === 120 ? 'selected' : ''}>120 FPS (High Refresh)</option>
                    </select>
                  </div>
                </div>
              </div>
            </div>

            <!-- TAB 5: LYRICS -->
            <div class="settings-panel ${this.activeTab === 'lyrics' ? 'active' : ''}" id="tab-panel-lyrics">
              <!-- Display Modes -->
              <div class="settings-section">
                <h3 class="settings-section-title">Display Presentation Mode</h3>
                <p class="settings-section-desc">Select how synchronized lyrics are rendered and centered.</p>
                <div class="settings-grid">
                  <div class="style-card lyrics-mode-card ${lyricsPrefs.mode === 'classic' ? 'active' : ''}" data-lyrics-mode="classic">
                    <div class="style-preview classic-lyrics-preview">
                      <span class="preview-line dim">First verse line</span>
                      <span class="preview-line active">Active singing line</span>
                      <span class="preview-line dim">Upcoming line</span>
                    </div>
                    <div class="style-card-info">
                      <span class="style-card-name">Classic List</span>
                      <span class="style-card-desc">Continuous scrolling list with active line highlighted.</span>
                    </div>
                  </div>

                  <div class="style-card lyrics-mode-card ${lyricsPrefs.mode === 'focused' ? 'active' : ''}" data-lyrics-mode="focused">
                    <div class="style-preview focused-lyrics-preview">
                      <span class="preview-line dim">...</span>
                      <span class="preview-line active bold">Focused Center Line</span>
                      <span class="preview-line dim">...</span>
                    </div>
                    <div class="style-card-info">
                      <span class="style-card-name">Focused / Cinematic</span>
                      <span class="style-card-desc">Cinematic focus on active line with soft ambient gradients.</span>
                    </div>
                  </div>

                  <div class="style-card lyrics-mode-card ${lyricsPrefs.mode === 'compact' ? 'active' : ''}" data-lyrics-mode="compact">
                    <div class="style-preview compact-lyrics-preview">
                      <span class="preview-line-compact active">Compact view line</span>
                      <span class="preview-line-compact">Tight secondary line</span>
                    </div>
                    <div class="style-card-info">
                      <span class="style-card-name">Compact</span>
                      <span class="style-card-desc">Dense typography tailored for narrow panels and sidebars.</span>
                    </div>
                  </div>

                  <div class="style-card lyrics-mode-card ${lyricsPrefs.mode === 'minimal' ? 'active' : ''}" data-lyrics-mode="minimal">
                    <div class="style-preview minimal-lyrics-preview">
                      <span class="preview-line-minimal">Pure minimal text line</span>
                    </div>
                    <div class="style-card-info">
                      <span class="style-card-name">Minimal</span>
                      <span class="style-card-desc">Distraction-free typography without cards or borders.</span>
                    </div>
                  </div>

                  <div class="style-card lyrics-mode-card ${lyricsPrefs.mode === 'dual' ? 'active' : ''}" data-lyrics-mode="dual">
                    <div class="style-preview dual-lyrics-preview">
                      <div class="preview-dual-curr">Now Singing</div>
                      <div class="preview-dual-next">Up Next</div>
                    </div>
                    <div class="style-card-info">
                      <span class="style-card-name">Dual-Line</span>
                      <span class="style-card-desc">Shows current active line alongside the next upcoming line.</span>
                    </div>
                  </div>
                </div>
              </div>

              <!-- Typography & Formatting -->
              <div class="settings-section">
                <h3 class="settings-section-title">Typography & Layout</h3>
                <div class="settings-controls-group">
                  <!-- Font Size -->
                  <div class="setting-control-row">
                    <label class="setting-label">Font Size</label>
                    <select class="setting-select lyrics-font-size-select">
                      <option value="sm" ${lyricsPrefs.fontSize === 'sm' ? 'selected' : ''}>Small (14px)</option>
                      <option value="md" ${lyricsPrefs.fontSize === 'md' ? 'selected' : ''}>Medium (16px)</option>
                      <option value="lg" ${lyricsPrefs.fontSize === 'lg' ? 'selected' : ''}>Large (20px)</option>
                      <option value="xl" ${lyricsPrefs.fontSize === 'xl' ? 'selected' : ''}>Extra Large (24px)</option>
                    </select>
                  </div>

                  <!-- Font Weight -->
                  <div class="setting-control-row">
                    <label class="setting-label">Font Weight</label>
                    <select class="setting-select lyrics-font-weight-select">
                      <option value="normal" ${lyricsPrefs.fontWeight === 'normal' ? 'selected' : ''}>Normal (400)</option>
                      <option value="medium" ${lyricsPrefs.fontWeight === 'medium' ? 'selected' : ''}>Medium (500)</option>
                      <option value="bold" ${lyricsPrefs.fontWeight === 'bold' ? 'selected' : ''}>Bold (700)</option>
                    </select>
                  </div>

                  <!-- Text Alignment -->
                  <div class="setting-control-row">
                    <label class="setting-label">Text Alignment</label>
                    <select class="setting-select lyrics-alignment-select">
                      <option value="left" ${lyricsPrefs.alignment === 'left' ? 'selected' : ''}>Left Aligned</option>
                      <option value="center" ${lyricsPrefs.alignment === 'center' ? 'selected' : ''}>Centered</option>
                      <option value="right" ${lyricsPrefs.alignment === 'right' ? 'selected' : ''}>Right Aligned</option>
                    </select>
                  </div>

                  <!-- Line Spacing -->
                  <div class="setting-control-row">
                    <label class="setting-label">Line Spacing</label>
                    <select class="setting-select lyrics-line-spacing-select">
                      <option value="compact" ${lyricsPrefs.lineSpacing === 'compact' ? 'selected' : ''}>Compact (Tight)</option>
                      <option value="normal" ${lyricsPrefs.lineSpacing === 'normal' ? 'selected' : ''}>Normal</option>
                      <option value="relaxed" ${lyricsPrefs.lineSpacing === 'relaxed' ? 'selected' : ''}>Relaxed (Spacious)</option>
                    </select>
                  </div>
                </div>
              </div>

              <!-- Highlighting & Background -->
              <div class="settings-section">
                <h3 class="settings-section-title">Visual Styling</h3>
                <div class="settings-controls-group">
                  <!-- Highlight Style -->
                  <div class="setting-control-row">
                    <label class="setting-label">Active Line Highlight</label>
                    <select class="setting-select lyrics-highlight-select">
                      <option value="glow" ${lyricsPrefs.highlightStyle === 'glow' ? 'selected' : ''}>Luminous Glow</option>
                      <option value="accent" ${lyricsPrefs.highlightStyle === 'accent' ? 'selected' : ''}>Accent Pill & Border</option>
                      <option value="underline" ${lyricsPrefs.highlightStyle === 'underline' ? 'selected' : ''}>Accent Underline</option>
                      <option value="scale" ${lyricsPrefs.highlightStyle === 'scale' ? 'selected' : ''}>Scale Zoom (106%)</option>
                    </select>
                  </div>

                  <!-- Background Mode -->
                  <div class="setting-control-row">
                    <label class="setting-label">Panel Background</label>
                    <select class="setting-select lyrics-bg-select">
                      <option value="surface" ${lyricsPrefs.backgroundMode === 'surface' ? 'selected' : ''}>Solid Theme Surface</option>
                      <option value="blurred" ${lyricsPrefs.backgroundMode === 'blurred' ? 'selected' : ''}>Glassmorphism Blur</option>
                      <option value="transparent" ${lyricsPrefs.backgroundMode === 'transparent' ? 'selected' : ''}>Transparent</option>
                    </select>
                  </div>

                  <!-- Inactive Opacity Slider -->
                  <div class="setting-control-row">
                    <div class="setting-label-group">
                      <label class="setting-label">Inactive Lines Opacity</label>
                      <span class="setting-value-badge lyrics-inactive-opacity-val">${Math.round(lyricsPrefs.inactiveOpacity * 100)}%</span>
                    </div>
                    <input type="range" class="setting-slider lyrics-inactive-opacity-slider" min="0.1" max="0.9" step="0.05" value="${lyricsPrefs.inactiveOpacity}" />
                  </div>
                </div>
              </div>

              <!-- Playback & Behavior -->
              <div class="settings-section">
                <h3 class="settings-section-title">Playback & Auto-Scroll</h3>
                <div class="settings-controls-group">
                  <div class="setting-control-row">
                    <div>
                      <label class="setting-label">Auto-scroll to Active Line</label>
                      <p class="setting-desc">Automatically center the currently sung lyric line during playback.</p>
                    </div>
                    <label class="toggle-switch">
                      <input type="checkbox" class="lyrics-autoscroll-toggle" ${lyricsPrefs.autoScroll ? 'checked' : ''} />
                      <span class="toggle-slider"></span>
                    </label>
                  </div>

                  <div class="setting-control-row">
                    <div>
                      <label class="setting-label">Timing Offset</label>
                      <p class="setting-desc">Current global offset: ${lyricsPrefs.manualOffsetMs}ms</p>
                    </div>
                    <button class="button button-secondary lyrics-reset-offset-btn">Reset Offset to 0ms</button>
                  </div>
                </div>
              </div>
            </div>
          </div>

          <!-- Footer -->
          <div class="settings-modal-footer">
            <button class="button button-danger reset-defaults-btn">Reset to Default</button>
            <div class="footer-right-buttons">
              <button class="button button-primary done-btn">Done</button>
            </div>
          </div>
        </div>
      </div>
    `;
  }

  private attachEventListeners() {
    if (!this.isOpen) return;

    // Close buttons
    const closeBtn = this.container.querySelector('.close-settings-btn');
    const doneBtn = this.container.querySelector('.done-btn');
    closeBtn?.addEventListener('click', () => this.close());
    doneBtn?.addEventListener('click', () => this.close());

    // Tab buttons
    const tabButtons = this.container.querySelectorAll('.settings-tab-btn');
    tabButtons.forEach((btn) => {
      btn.addEventListener('click', () => {
        const tab = btn.getAttribute('data-tab') as SettingsTab;
        if (tab) {
          this.activeTab = tab;
          this.render();
          this.attachEventListeners();
        }
      });
    });

    // Theme preset selection
    const themeCards = this.container.querySelectorAll('.theme-card');
    themeCards.forEach((card) => {
      card.addEventListener('click', () => {
        const themeId = card.getAttribute('data-theme-id');
        if (themeId) {
          themeEngine.setTheme(themeId);
          this.render();
          this.attachEventListeners();
        }
      });
    });

    // Marketplace theme actions: preview / apply / remove / reset.
    const rerender = () => {
      this.render();
      this.attachEventListeners();
    };
    this.container.querySelectorAll('.mp-preview-btn').forEach((btn) => {
      btn.addEventListener('click', () => {
        const id = (btn as HTMLElement).dataset.id;
        if (!id) return;
        void previewMarketplaceTheme(id).then((ok) => {
          this.previewThemeId = ok ? id : null;
          this.marketplaceStatus = ok ? `Previewing — Apply to keep, or Cancel preview.` : `Cannot preview ${id}.`;
          rerender();
        });
      });
    });
    this.container.querySelectorAll('.mp-apply-btn').forEach((btn) => {
      btn.addEventListener('click', () => {
        const id = (btn as HTMLElement).dataset.id;
        if (!id) return;
        cancelMarketplacePreview();
        void applyMarketplaceTheme(id).then((ok) => {
          this.previewThemeId = null;
          this.marketplaceStatus = ok ? `Applied ${id}.` : `Could not apply ${id}.`;
          rerender();
        });
      });
    });
    this.container.querySelectorAll('.mp-remove-btn').forEach((btn) => {
      btn.addEventListener('click', () => {
        const id = (btn as HTMLElement).dataset.id;
        if (!id) return;
        cancelMarketplacePreview();
        void removeMarketplaceTheme(id)
          .then(() => syncMarketplaceThemesFromBackend())
          .then(() => api.marketplaceThemes())
          .then((themes) => {
            this.marketplaceThemes = themes;
            this.previewThemeId = null;
            this.marketplaceStatus = `Removed ${id}.`;
            rerender();
          })
          .catch((e) => {
            this.marketplaceStatus = `Remove failed: ${String(e)}`;
            rerender();
          });
      });
    });
    const cancelPreview = this.container.querySelector('.mp-cancel-preview-btn');
    cancelPreview?.addEventListener('click', () => {
      cancelMarketplacePreview();
      this.previewThemeId = null;
      this.marketplaceStatus = null;
      rerender();
    });
    const resetBuiltIn = this.container.querySelector('.mp-reset-builtin-btn');
    resetBuiltIn?.addEventListener('click', () => {
      cancelMarketplacePreview();
      void resetToBuiltInTheme().then(() => {
        this.previewThemeId = null;
        this.marketplaceStatus = 'Reset to the built-in theme.';
        rerender();
      });
    });

    // Accent chips
    const accentChips = this.container.querySelectorAll('.accent-preset-chip');
    accentChips.forEach((chip) => {
      chip.addEventListener('click', () => {
        const hex = chip.getAttribute('data-hex');
        if (hex) {
          themeEngine.setCustomAccent(hex);
          this.render();
          this.attachEventListeners();
        }
      });
    });

    // Custom accent color input
    const customAccentInput = this.container.querySelector('.custom-accent-input') as HTMLInputElement;
    customAccentInput?.addEventListener('input', () => {
      themeEngine.setCustomAccent(customAccentInput.value);
    });

    // Clear accent override
    const clearAccentBtn = this.container.querySelector('.clear-accent-btn');
    clearAccentBtn?.addEventListener('click', () => {
      themeEngine.setCustomAccent(null);
      this.render();
      this.attachEventListeners();
    });

    // Export theme
    const exportBtn = this.container.querySelector('.export-theme-btn');
    exportBtn?.addEventListener('click', () => {
      const json = themeEngine.exportActiveThemeJson();
      const blob = new Blob([json], { type: 'application/json' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `${themeEngine.getActiveTheme().id}.json`;
      a.click();
      URL.revokeObjectURL(url);
    });

    // Import theme
    const importBtn = this.container.querySelector('.import-theme-btn');
    const fileInput = this.container.querySelector('.import-theme-file-input') as HTMLInputElement;
    importBtn?.addEventListener('click', () => fileInput?.click());

    fileInput?.addEventListener('change', async () => {
      const file = fileInput.files?.[0];
      if (file) {
        try {
          const text = await file.text();
          const result = themeEngine.importThemeFromJson(text);
          if (result.success) {
            this.render();
            this.attachEventListeners();
          } else {
            alert(`Theme import error:\n${result.errors?.join('\n')}`);
          }
        } catch (e) {
          alert(`Failed to read theme file: ${e}`);
        }
      }
    });

    // Layout select
    const layoutSelect = this.container.querySelector('.layout-select') as HTMLSelectElement;
    layoutSelect?.addEventListener('change', () => {
      layoutManager.setLayout(layoutSelect.value);
      this.render();
      this.attachEventListeners();
    });

    // Save custom layout
    const saveLayoutBtn = this.container.querySelector('.save-custom-layout-btn');
    saveLayoutBtn?.addEventListener('click', () => {
      const name = prompt('Enter a name for this custom layout:', 'My Custom Workspace');
      if (name && name.trim()) {
        layoutManager.saveCurrentAsCustom(name.trim());
        this.render();
        this.attachEventListeners();
      }
    });

    // Delete custom layout
    const deleteLayoutBtn = this.container.querySelector('.delete-layout-btn');
    deleteLayoutBtn?.addEventListener('click', () => {
      const active = layoutManager.getActiveLayout();
      if (!active.isBuiltIn && confirm(`Delete layout "${active.name}"?`)) {
        layoutManager.deleteCustomLayout(active.id);
        this.render();
        this.attachEventListeners();
      }
    });

    // Regional checkboxes
    const regionCheckboxes = this.container.querySelectorAll('.region-toggle-item input[type="checkbox"]');
    regionCheckboxes.forEach((cb) => {
      cb.addEventListener('change', () => {
        const checkbox = cb as HTMLInputElement;
        const region = checkbox.getAttribute('data-region') as RegionId;
        if (region && ALL_REGION_IDS.includes(region)) {
          layoutManager.setRegionVisible(region, checkbox.checked);
        }
      });
    });

    // Artwork style selection
    const artStyleCards = this.container.querySelectorAll('.art-style-card');
    artStyleCards.forEach((card) => {
      card.addEventListener('click', () => {
        const styleId = card.getAttribute('data-style-id') as AlbumArtStyleId;
        if (styleId) {
          artworkStyleManager.setStyle(styleId);
          this.render();
          this.attachEventListeners();
        }
      });
    });

    // Visualizer enable toggle
    const visEnableToggle = this.container.querySelector('.vis-enable-toggle') as HTMLInputElement;
    visEnableToggle?.addEventListener('change', () => {
      visualizerConfigManager.updateConfig({ enabled: visEnableToggle.checked });
    });

    // Visualizer style select
    const visStyleSelect = this.container.querySelector('.vis-style-select') as HTMLSelectElement;
    visStyleSelect?.addEventListener('change', () => {
      visualizerConfigManager.updateConfig({ style: visStyleSelect.value as VisualizerStyleId });
    });

    // Visualizer color select
    const visColorSelect = this.container.querySelector('.vis-color-select') as HTMLSelectElement;
    visColorSelect?.addEventListener('change', () => {
      visualizerConfigManager.updateConfig({ colorSource: visColorSelect.value as VisualizerColorSource });
    });

    // Visualizer sensitivity slider
    const sensSlider = this.container.querySelector('.vis-sens-slider') as HTMLInputElement;
    const sensVal = this.container.querySelector('.vis-sens-val') as HTMLElement;
    sensSlider?.addEventListener('input', () => {
      const val = parseFloat(sensSlider.value);
      if (sensVal) sensVal.textContent = `${val.toFixed(1)}x`;
      visualizerConfigManager.updateConfig({ sensitivity: val });
    });

    // Visualizer smoothing slider
    const smoothSlider = this.container.querySelector('.vis-smooth-slider') as HTMLInputElement;
    const smoothVal = this.container.querySelector('.vis-smooth-val') as HTMLElement;
    smoothSlider?.addEventListener('input', () => {
      const val = parseFloat(smoothSlider.value);
      if (smoothVal) smoothVal.textContent = `${(val * 1000).toFixed(0)} ms`;
      visualizerConfigManager.updateConfig({ smoothing: val });
    });

    // Visualizer height slider
    const heightSlider = this.container.querySelector('.vis-height-slider') as HTMLInputElement;
    const heightVal = this.container.querySelector('.vis-height-val') as HTMLElement;
    heightSlider?.addEventListener('input', () => {
      const val = parseInt(heightSlider.value, 10);
      if (heightVal) heightVal.textContent = `${val} px`;
      visualizerConfigManager.updateConfig({ height: val });
    });

    // Visualizer FPS select
    const fpsSelect = this.container.querySelector('.vis-fps-select') as HTMLSelectElement;
    fpsSelect?.addEventListener('change', () => {
      visualizerConfigManager.updateConfig({ fps: parseInt(fpsSelect.value, 10) });
    });

    // --- TAB 5: LYRICS CONTROLS ---

    // Lyrics Mode Cards
    const lyricsModeCards = this.container.querySelectorAll('.lyrics-mode-card');
    lyricsModeCards.forEach((card) => {
      card.addEventListener('click', () => {
        const mode = card.getAttribute('data-lyrics-mode') as any;
        if (mode) {
          lyricsManager.setMode(mode);
          this.render();
          this.attachEventListeners();
        }
      });
    });

    // Font Size
    const lyricsFontSizeSelect = this.container.querySelector('.lyrics-font-size-select') as HTMLSelectElement;
    lyricsFontSizeSelect?.addEventListener('change', () => {
      lyricsManager.setFontSize(lyricsFontSizeSelect.value as any);
    });

    // Font Weight
    const lyricsFontWeightSelect = this.container.querySelector('.lyrics-font-weight-select') as HTMLSelectElement;
    lyricsFontWeightSelect?.addEventListener('change', () => {
      lyricsManager.setFontWeight(lyricsFontWeightSelect.value as any);
    });

    // Text Alignment
    const lyricsAlignmentSelect = this.container.querySelector('.lyrics-alignment-select') as HTMLSelectElement;
    lyricsAlignmentSelect?.addEventListener('change', () => {
      lyricsManager.setAlignment(lyricsAlignmentSelect.value as any);
    });

    // Line Spacing
    const lyricsLineSpacingSelect = this.container.querySelector('.lyrics-line-spacing-select') as HTMLSelectElement;
    lyricsLineSpacingSelect?.addEventListener('change', () => {
      lyricsManager.setLineSpacing(lyricsLineSpacingSelect.value as any);
    });

    // Highlight Style
    const lyricsHighlightSelect = this.container.querySelector('.lyrics-highlight-select') as HTMLSelectElement;
    lyricsHighlightSelect?.addEventListener('change', () => {
      lyricsManager.setHighlightStyle(lyricsHighlightSelect.value as any);
    });

    // Background Mode
    const lyricsBgSelect = this.container.querySelector('.lyrics-bg-select') as HTMLSelectElement;
    lyricsBgSelect?.addEventListener('change', () => {
      lyricsManager.setBackgroundMode(lyricsBgSelect.value as any);
    });

    // Inactive Opacity Slider
    const lyricsOpacitySlider = this.container.querySelector('.lyrics-inactive-opacity-slider') as HTMLInputElement;
    const lyricsOpacityVal = this.container.querySelector('.lyrics-inactive-opacity-val') as HTMLElement;
    lyricsOpacitySlider?.addEventListener('input', () => {
      const val = parseFloat(lyricsOpacitySlider.value);
      if (lyricsOpacityVal) lyricsOpacityVal.textContent = `${Math.round(val * 100)}%`;
      lyricsManager.updatePreferences({ inactiveOpacity: val });
    });

    // Auto-scroll toggle
    const lyricsAutoscrollToggle = this.container.querySelector('.lyrics-autoscroll-toggle') as HTMLInputElement;
    lyricsAutoscrollToggle?.addEventListener('change', () => {
      lyricsManager.setAutoScroll(lyricsAutoscrollToggle.checked);
    });

    // Reset offset button
    const lyricsResetOffsetBtn = this.container.querySelector('.lyrics-reset-offset-btn');
    lyricsResetOffsetBtn?.addEventListener('click', () => {
      lyricsManager.resetManualOffset();
      this.render();
      this.attachEventListeners();
    });

    // Reset to default
    const resetBtn = this.container.querySelector('.reset-defaults-btn');
    resetBtn?.addEventListener('click', () => {
      if (confirm('Reset settings on this tab to defaults?')) {
        switch (this.activeTab) {
          case 'theme':
            themeEngine.resetToDefault();
            break;
          case 'layout':
            layoutManager.resetToDefault();
            break;
          case 'artwork':
            artworkStyleManager.resetToDefault();
            break;
          case 'visualizer':
            visualizerConfigManager.resetToDefault();
            break;
          case 'lyrics':
            lyricsManager.resetToDefaults();
            break;
        }
        this.render();
        this.attachEventListeners();
      }
    });
  }
}
