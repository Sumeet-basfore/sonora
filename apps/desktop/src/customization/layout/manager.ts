/**
 * Sonora Layout Manager: Multi-layout switching, region toggling,
 * local persistence, and presentation-layer layout synchronization.
 */

import { BUILT_IN_LAYOUTS, LAYOUT_DEFAULT_STUDIO } from './presets.ts';
import {
  type LayoutDefinition,
  type RegionId,
  type RegionVisibility,
  validateLayoutSchema,
  LAYOUT_SCHEMA_URI,
} from './schema.ts';

const STORAGE_KEY_ACTIVE_LAYOUT = 'sonora_layout_active_id';
const STORAGE_KEY_CUSTOM_LAYOUTS = 'sonora_layout_custom_definitions';
const STORAGE_KEY_REGION_OVERRIDES = 'sonora_layout_region_state';

export type LayoutChangeListener = (layout: LayoutDefinition, regions: RegionVisibility) => void;

export class LayoutManager {
  private activeLayout: LayoutDefinition = LAYOUT_DEFAULT_STUDIO;
  private currentRegions: RegionVisibility = { ...LAYOUT_DEFAULT_STUDIO.regions };
  private customLayouts: Map<string, LayoutDefinition> = new Map();
  private listeners: Set<LayoutChangeListener> = new Set();

  constructor() {
    this.loadFromStorage();
  }

  public subscribe(listener: LayoutChangeListener): () => void {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  }

  private notify() {
    for (const listener of this.listeners) {
      try {
        listener(this.activeLayout, this.currentRegions);
      } catch (e) {
        console.error('Error in layout listener:', e);
      }
    }
  }

  public getAvailableLayouts(): LayoutDefinition[] {
    const all = [...BUILT_IN_LAYOUTS];
    for (const custom of this.customLayouts.values()) {
      if (!all.some(l => l.id === custom.id)) {
        all.push(custom);
      }
    }
    return all;
  }

  public getActiveLayout(): LayoutDefinition {
    return this.activeLayout;
  }

  public getRegions(): RegionVisibility {
    return { ...this.currentRegions };
  }

  public isRegionVisible(region: RegionId): boolean {
    return this.currentRegions[region] ?? true;
  }

  /**
   * Toggle or set the visibility of an individual UI region.
   */
  public setRegionVisible(region: RegionId, visible: boolean) {
    this.currentRegions[region] = visible;
    this.applyLayoutToDom();
    this.saveToStorage();
    this.notify();
  }

  public toggleRegion(region: RegionId) {
    this.setRegionVisible(region, !this.isRegionVisible(region));
  }

  /**
   * Switch active layout preset by ID.
   */
  public setLayout(layoutId: string): boolean {
    const layout = this.getAvailableLayouts().find(l => l.id === layoutId);
    if (!layout) return false;

    this.activeLayout = layout;
    this.currentRegions = { ...layout.regions };
    this.applyLayoutToDom();
    this.saveToStorage();
    this.notify();
    return true;
  }

  /**
   * Save the current regional configuration as a new custom layout.
   */
  public saveCurrentAsCustom(name: string, description: string = 'User customized layout'): LayoutDefinition {
    const id = `layout-custom-${Date.now()}`;
    const newLayout: LayoutDefinition = {
      $schema: LAYOUT_SCHEMA_URI,
      id,
      name,
      description,
      version: '1.0.0',
      isBuiltIn: false,
      regions: { ...this.currentRegions },
    };

    this.customLayouts.set(id, newLayout);
    this.activeLayout = newLayout;
    this.saveToStorage();
    this.notify();
    return newLayout;
  }

  /**
   * Delete a custom layout. Cannot delete built-in layouts.
   */
  public deleteCustomLayout(layoutId: string): boolean {
    if (!this.customLayouts.has(layoutId)) return false;

    this.customLayouts.delete(layoutId);
    if (this.activeLayout.id === layoutId) {
      this.setLayout(LAYOUT_DEFAULT_STUDIO.id);
    } else {
      this.saveToStorage();
      this.notify();
    }
    return true;
  }

  /**
   * Reset layout system to Default Studio.
   */
  public resetToDefault() {
    this.activeLayout = LAYOUT_DEFAULT_STUDIO;
    this.currentRegions = { ...LAYOUT_DEFAULT_STUDIO.regions };
    this.applyLayoutToDom();
    this.saveToStorage();
    this.notify();
  }

  /**
   * Apply data attributes and class names to DOM for responsive CSS styling.
   */
  public applyLayoutToDom() {
    const root = document.querySelector('.app-layout');
    if (!root) return;

    for (const [region, visible] of Object.entries(this.currentRegions)) {
      root.setAttribute(`data-show-${region}`, visible ? 'true' : 'false');
    }
  }

  private saveToStorage() {
    try {
      if (typeof localStorage === 'undefined') return;
      localStorage.setItem(STORAGE_KEY_ACTIVE_LAYOUT, this.activeLayout.id);
      localStorage.setItem(STORAGE_KEY_REGION_OVERRIDES, JSON.stringify(this.currentRegions));

      const customArray = Array.from(this.customLayouts.values());
      localStorage.setItem(STORAGE_KEY_CUSTOM_LAYOUTS, JSON.stringify(customArray));
    } catch (e) {
      console.warn('Failed to save layout settings to storage:', e);
    }
  }

  private loadFromStorage() {
    try {
      if (typeof localStorage === 'undefined') return;

      // Load custom layouts
      const customStr = localStorage.getItem(STORAGE_KEY_CUSTOM_LAYOUTS);
      if (customStr) {
        try {
          const parsed = JSON.parse(customStr);
          if (Array.isArray(parsed)) {
            for (const item of parsed) {
              if (validateLayoutSchema(item).valid) {
                this.customLayouts.set(item.id, item);
              }
            }
          }
        } catch {
          // ignore corrupted custom layouts
        }
      }

      // Load active layout
      const savedId = localStorage.getItem(STORAGE_KEY_ACTIVE_LAYOUT);
      if (savedId) {
        const found = this.getAvailableLayouts().find(l => l.id === savedId);
        if (found) {
          this.activeLayout = found;
          this.currentRegions = { ...found.regions };
        }
      }

      // Load region overrides if any
      const regionsStr = localStorage.getItem(STORAGE_KEY_REGION_OVERRIDES);
      if (regionsStr) {
        try {
          const parsed = JSON.parse(regionsStr);
          if (parsed && typeof parsed === 'object') {
            this.currentRegions = { ...this.currentRegions, ...parsed };
          }
        } catch {
          // ignore
        }
      }
    } catch (e) {
      console.warn('Failed to load layout settings from storage:', e);
    }
  }
}

export const layoutManager = new LayoutManager();
