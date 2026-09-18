import { api } from '../api';
import { appState } from '../state';
import {
  applyMarketplaceTheme,
  previewMarketplaceTheme,
} from '../marketplaceThemes';
import type {
  CatalogEntry,
  InstalledEntry,
  MarketplaceCatalog,
  UpdateInfo,
} from '../types';
import {
  categoriesOf,
  describeCapabilities,
  filterCatalog,
  orderUpdates,
  sortCatalog,
  timeAgoLabel,
  type BrowseFilter,
  type MarketplaceTab,
} from '../marketplace';

function esc(text: string): string {
  return text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;');
}

export class MarketplaceViewComponent {
  private container: HTMLElement;
  private tab: MarketplaceTab = 'browse';
  private filter: BrowseFilter = { query: '', category: 'all', kind: 'all' };
  private catalog: MarketplaceCatalog | null = null;
  private installed: InstalledEntry[] = [];
  private updates: UpdateInfo[] = [];
  private selectedId: string | null = null;
  private pendingConsentId: string | null = null;
  private busy: Set<string> = new Set();
  private notice: { kind: 'error' | 'info'; text: string } | null = null;
  private lastViewKey = '';

  constructor(container: HTMLElement) {
    this.container = container;
    appState.subscribe(() => this.renderIfActive());
    this.renderIfActive();
  }

  private get active(): boolean {
    return appState.getActiveView().type === 'marketplace';
  }

  private renderIfActive() {
    const key = JSON.stringify(appState.getActiveView());
    if (!this.active) {
      if (this.lastViewKey !== '') {
        this.lastViewKey = '';
        this.container.innerHTML = '';
      }
      return;
    }
    if (key !== this.lastViewKey) {
      this.lastViewKey = key;
      this.selectedId = null;
      this.pendingConsentId = null;
      this.load();
    }
  }

  private async load() {
    this.renderLoading();
    try {
      const [catalog, installed, updates] = await Promise.all([
        api.marketplaceCatalog(),
        api.marketplaceInstalled(),
        api.marketplaceUpdates(),
      ]);
      this.catalog = catalog;
      this.installed = installed;
      this.updates = orderUpdates(updates);
      this.notice = null;
    } catch {
      // Graceful fallback for offline / unreachable registry
      try {
        const installed = await api.marketplaceInstalled();
        this.installed = installed;
      } catch {
        this.installed = [];
      }
      this.notice = {
        kind: 'error',
        text: 'Unable to reach the extension registry. You can still manage your installed extensions.',
      };
    }
    this.render();
  }

  private async reload() {
    try {
      const [catalog, installed, updates] = await Promise.all([
        api.marketplaceCatalog(),
        api.marketplaceInstalled(),
        api.marketplaceUpdates(),
      ]);
      this.catalog = catalog;
      this.installed = installed;
      this.updates = orderUpdates(updates);
    } catch {
      this.notice = {
        kind: 'error',
        text: 'Unable to refresh extension catalog. Showing cached data.',
      };
    }
    this.render();
  }

  private renderLoading() {
    this.container.innerHTML = `
      <div class="view-loading-state">
        <div class="spinner"></div>
        <p>Loading extensions…</p>
      </div>`;
  }


  private render() {
    if (!this.active) return;
    const tabs: MarketplaceTab[] = ['browse', 'installed', 'updates'];
    const updateCount = this.updates.filter((u) => u.compatible).length;
    this.container.innerHTML = `
      <div class="marketplace-view">
        <div class="marketplace-header">
          <h2>Extensions</h2>
          <div class="marketplace-tabs" role="tablist">
            ${tabs
              .map(
                (t) => `
              <button class="marketplace-tab${this.tab === t ? ' active' : ''}" data-tab="${t}" role="tab">
                ${t === 'browse' ? 'Browse' : t === 'installed' ? `Installed (${this.installed.length})` : `Updates${updateCount ? ` (${updateCount})` : ''}`}
              </button>`
              )
              .join('')}
          </div>
        </div>
        ${this.offlineBanner()}
        ${this.notice ? `<div class="marketplace-notice ${this.notice.kind}">${esc(this.notice.text)}</div>` : ''}
        <div class="marketplace-body">
          ${this.tab === 'browse' ? this.renderBrowse() : ''}
          ${this.tab === 'installed' ? this.renderInstalled() : ''}
          ${this.tab === 'updates' ? this.renderUpdates() : ''}
        </div>
        ${this.selectedId ? this.renderDetail() : ''}
      </div>
    `;
    this.attachListeners();
  }

  private offlineBanner(): string {
    if (!this.catalog) return '';
    const parts: string[] = [];
    if (this.catalog.offline) parts.push('offline — showing cached registry');
    else if (this.catalog.stale) parts.push('registry cache is stale');
    if (!parts.length) return '';
    return `<div class="marketplace-offline">⚠ ${esc(parts.join('; '))} · updated ${esc(timeAgoLabel(this.catalog.fetched_at))}</div>`;
  }

  private allEntries(): CatalogEntry[] {
    if (!this.catalog) return [];
    return [...this.catalog.plugins, ...this.catalog.themes];
  }

  private renderBrowse(): string {
    const entries = sortCatalog(filterCatalog(this.allEntries(), this.filter));
    const categories = categoriesOf(this.allEntries());
    return `
      <div class="marketplace-filters">
        <input class="marketplace-search" type="search" placeholder="Search extensions…" value="${esc(this.filter.query)}" aria-label="Search extensions" />
        <select class="marketplace-kind" aria-label="Extension kind">
          ${['all', 'plugin', 'theme'].map((k) => `<option value="${k}"${this.filter.kind === k ? ' selected' : ''}>${k === 'all' ? 'All kinds' : k === 'plugin' ? 'Plugins' : 'Themes'}</option>`).join('')}
        </select>
        <select class="marketplace-category" aria-label="Category">
          <option value="all">All categories</option>
          ${categories.map((c) => `<option value="${esc(c)}"${this.filter.category === c ? ' selected' : ''}>${esc(c)}</option>`).join('')}
        </select>
      </div>
      <div class="marketplace-grid">
        ${entries.length ? entries.map((e) => this.card(e)).join('') : '<p class="marketplace-empty">No extensions match.</p>'}
      </div>
    `;
  }

  private card(entry: CatalogEntry): string {
    const busy = this.busy.has(entry.id);
    const action = entry.update_available
      ? `<button data-action="update" data-id="${esc(entry.id)}" ${busy ? 'disabled' : ''}>Update to ${esc(entry.update_available)}</button>`
      : entry.installed_version
        ? `<span class="marketplace-badge">Installed ${esc(entry.installed_version)}</span>`
        : `<button data-action="install" data-id="${esc(entry.id)}" ${busy ? 'disabled' : ''}>Install</button>`;
    return `
      <article class="marketplace-card" data-id="${esc(entry.id)}">
        <div class="marketplace-card-head">
          <strong>${esc(entry.name)}</strong>
          <span class="marketplace-kind-badge">${esc(entry.kind)}</span>
        </div>
        <div class="marketplace-card-meta">${esc(entry.category)} · v${esc(entry.latest_version)} · ${esc(entry.author)}</div>
        <p class="marketplace-card-desc">${esc(entry.description)}</p>
        <div class="marketplace-card-foot">
          ${action}
          <button data-action="detail" data-id="${esc(entry.id)}">Details</button>
        </div>
      </article>
    `;
  }

  private renderInstalled(): string {
    if (!this.installed.length) return '<p class="marketplace-empty">Nothing installed yet — browse the catalog to add extensions.</p>';
    return `
      <div class="marketplace-list">
        ${this.installed
          .map(
            (e) => `
          <div class="marketplace-row">
            <div><strong>${esc(e.name)}</strong> <span class="marketplace-kind-badge">${esc(e.kind)}</span>
            <div class="marketplace-card-meta">v${esc(e.version)} · ${esc(e.state)}</div></div>
            <div class="marketplace-row-actions">
              ${e.kind === 'theme' ? `<button data-action="theme-preview" data-id="${esc(e.id)}">Preview</button>` : ''}
              ${e.kind === 'theme' ? `<button data-action="theme-apply" data-id="${esc(e.id)}">Apply</button>` : ''}
              ${e.kind === 'plugin' ? `<button data-action="rollback" data-id="${esc(e.id)}" ${this.busy.has(e.id) ? 'disabled' : ''}>Roll back</button>` : ''}
              <button data-action="uninstall" data-id="${esc(e.id)}" ${this.busy.has(e.id) ? 'disabled' : ''}>${e.kind === 'theme' ? 'Remove' : 'Uninstall'}</button>
            </div>
          </div>`
          )
          .join('')}
      </div>
    `;
  }

  private renderUpdates(): string {
    if (!this.updates.length) return '<p class="marketplace-empty">Everything is up to date.</p>';
    return `
      <div class="marketplace-list">
        ${this.updates
          .map(
            (u) => `
          <div class="marketplace-row">
            <div><strong>${esc(u.name)}</strong>
            <div class="marketplace-card-meta">${esc(u.current)} → ${esc(u.available)}${u.compatible ? '' : ` · requires Sonora ${esc(u.min_sonora_version)}`}</div>
            <p class="marketplace-changelog">${esc(u.changelog)}</p></div>
            <div class="marketplace-row-actions">
              <button data-action="update" data-id="${esc(u.id)}" ${!u.compatible || this.busy.has(u.id) ? 'disabled' : ''}>${u.compatible ? 'Update' : 'Incompatible'}</button>
            </div>
          </div>`
          )
          .join('')}
      </div>
    `;
  }

  private renderDetail(): string {
    const entry = this.allEntries().find((e) => e.id === this.selectedId);
    if (!entry) return '';
    const consent = this.pendingConsentId === entry.id;
    return `
      <div class="marketplace-detail-overlay" data-action="close-detail">
        <div class="marketplace-detail" role="dialog" aria-label="${esc(entry.name)}">
          <h3>${esc(entry.name)}</h3>
          <div class="marketplace-card-meta">${esc(entry.kind)} · ${esc(entry.category)} · v${esc(entry.latest_version)} · by ${esc(entry.author)}</div>
          <p>${esc(entry.description)}</p>
          ${entry.homepage ? `<p><a href="${esc(entry.homepage)}" target="_blank" rel="noreferrer">Homepage</a></p>` : ''}
          <h4>Permissions</h4>
          <ul>${describeCapabilities(entry).map((d) => `<li>${esc(d)}</li>`).join('')}</ul>
          <p class="marketplace-card-meta">Requires Sonora ≥ ${esc(entry.min_sonora_version)}${entry.installed_version ? ` · installed: ${esc(entry.installed_version)}` : ''}</p>
          ${consent ? `<p class="marketplace-consent">This will download and verify the package, then install it. Capability enforcement stays active at runtime.</p>` : ''}
          <div class="marketplace-row-actions">
            ${entry.installed_version ? '' : `<button data-action="${consent ? 'install-confirm' : 'install'}" data-id="${esc(entry.id)}">${consent ? 'Confirm install' : 'Install'}</button>`}
            ${entry.update_available ? `<button data-action="update" data-id="${esc(entry.id)}">Update to ${esc(entry.update_available)}</button>` : ''}
            <button data-action="close-detail">Close</button>
          </div>
        </div>
      </div>
    `;
  }

  private attachListeners() {
    this.container.querySelectorAll('[data-tab]').forEach((el) => {
      el.addEventListener('click', () => {
        this.tab = (el as HTMLElement).dataset.tab as MarketplaceTab;
        this.render();
      });
    });
    const search = this.container.querySelector('.marketplace-search') as HTMLInputElement | null;
    search?.addEventListener('input', () => {
      this.filter.query = search.value;
      this.render();
      const redo = this.container.querySelector('.marketplace-search') as HTMLInputElement | null;
      if (redo) {
        redo.focus();
        redo.setSelectionRange(redo.value.length, redo.value.length);
      }
    });
    const kind = this.container.querySelector('.marketplace-kind') as HTMLSelectElement | null;
    kind?.addEventListener('change', () => {
      this.filter.kind = kind.value as BrowseFilter['kind'];
      this.render();
    });
    const cat = this.container.querySelector('.marketplace-category') as HTMLSelectElement | null;
    cat?.addEventListener('change', () => {
      this.filter.category = cat.value;
      this.render();
    });
    this.container.querySelectorAll('[data-action]').forEach((el) => {
      el.addEventListener('click', (ev) => {
        ev.stopPropagation();
        const action = (el as HTMLElement).dataset.action!;
        const id = (el as HTMLElement).dataset.id ?? null;
        void this.onAction(action, id);
      });
    });
  }

  private async onAction(action: string, id: string | null) {
    if (action === 'detail' && id) {
      this.selectedId = id;
      this.pendingConsentId = null;
      this.render();
      return;
    }
    if (action === 'close-detail') {
      this.selectedId = null;
      this.pendingConsentId = null;
      this.render();
      return;
    }
    if (!id) return;
    if (action === 'install') {
      // First click arms the consent step (full permission list in detail);
      // from a card it installs directly after this confirmation gate.
      if (this.pendingConsentId !== id) {
        this.pendingConsentId = id;
        this.selectedId = id;
        this.render();
        return;
      }
    }
    if (action === 'install-confirm') {
      return this.runBusy(id, async () => {
        const report = await api.marketplaceInstall(id);
        this.pendingConsentId = null;
        this.notice = {
          kind: report.started ? 'info' : 'error',
          text: report.started
            ? `Installed ${report.id} v${report.version}.`
            : `Installed but inactive: ${report.start_error ?? 'failed to start'}`,
        };
        await this.reload();
      });
    }
    if (action === 'update') {
      return this.runBusy(id, async () => {
        const report = await api.marketplaceUpdate(id);
        this.notice = { kind: 'info', text: `Updated ${report.id} to v${report.version}.` };
        await this.reload();
      });
    }
    if (action === 'rollback') {
      return this.runBusy(id, async () => {
        const report = await api.marketplaceRollback(id);
        this.notice = { kind: 'info', text: `Rolled back ${report.id} to v${report.restored_version}.` };
        await this.reload();
      });
    }
    if (action === 'uninstall') {
      return this.runBusy(id, async () => {
        await api.marketplaceUninstall(id);
        this.notice = { kind: 'info', text: `Uninstalled ${id}.` };
        await this.reload();
      });
    }
    if (action === 'theme-preview') {
      const ok = await previewMarketplaceTheme(id);
      this.notice = ok
        ? { kind: 'info', text: `Previewing theme — open Customization to Apply or pick another theme to revert.` }
        : { kind: 'error', text: `Cannot preview ${id}.` };
      this.render();
      return;
    }
    if (action === 'theme-apply') {
      return this.runBusy(id, async () => {
        const ok = await applyMarketplaceTheme(id);
        this.notice = ok
          ? { kind: 'info', text: `Applied theme ${id}.` }
          : { kind: 'error', text: `Could not apply ${id}.` };
        await this.reload();
      });
    }
  }

  private async runBusy(id: string, fn: () => Promise<void>) {
    this.busy.add(id);
    this.render();
    try {
      await fn();
    } catch (e: any) {
      const raw = e?.message || String(e);
      const clean = raw.replace(/^Error:\s*/i, '').replace(/^Backend error:\s*/i, '');
      this.notice = { kind: 'error', text: clean || 'Operation failed. Please try again.' };
      this.render();
    } finally {
      this.busy.delete(id);
      this.render();
    }

  }
}
