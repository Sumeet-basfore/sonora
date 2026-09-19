/**
 * Sonora Visualizer Configuration and Settings Model.
 * Manages presentation styling, FPS limits, temporal smoothing,
 * gain sensitivity, and hardware-efficient toggling.
 */

export type VisualizerStyleId = 'bars' | 'wave' | 'mirror' | 'led';
export type VisualizerColorSource = 'accent' | 'gradient' | 'monochrome' | 'artwork';
export type VisualizerDisplayMode = 'floating' | 'docked';

export interface VisualizerConfig {
  style: VisualizerStyleId;
  sensitivity: number; // 0.2 to 3.0
  smoothing: number;   // 0.01 to 0.50 (time constant tau in seconds)
  height: number;      // 60 to 320 px
  fps: number;         // 30, 60, or 120
  colorSource: VisualizerColorSource;
  displayMode: VisualizerDisplayMode;
  enabled: boolean;
}

export const DEFAULT_VISUALIZER_CONFIG: VisualizerConfig = {
  style: 'bars',
  sensitivity: 1.0,
  smoothing: 0.12,
  height: 120,
  fps: 60,
  colorSource: 'accent',
  displayMode: 'floating',
  enabled: true,
};

const STORAGE_KEY_VISUALIZER_CONFIG = 'sonora_visualizer_config';

export type VisualizerConfigChangeListener = (config: VisualizerConfig) => void;

export class VisualizerConfigManager {
  private config: VisualizerConfig = { ...DEFAULT_VISUALIZER_CONFIG };
  private listeners: Set<VisualizerConfigChangeListener> = new Set();

  constructor() {
    this.loadFromStorage();
  }

  public subscribe(listener: VisualizerConfigChangeListener): () => void {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  }

  private notify() {
    for (const listener of this.listeners) {
      try {
        listener(this.config);
      } catch (e) {
        console.error('Error in visualizer config listener:', e);
      }
    }
  }

  public getConfig(): VisualizerConfig {
    return { ...this.config };
  }

  public updateConfig(partial: Partial<VisualizerConfig>) {
    this.config = {
      ...this.config,
      ...partial,
      sensitivity: partial.sensitivity !== undefined
        ? Math.max(0.2, Math.min(3.0, partial.sensitivity))
        : this.config.sensitivity,
      smoothing: partial.smoothing !== undefined
        ? Math.max(0.01, Math.min(0.50, partial.smoothing))
        : this.config.smoothing,
      height: partial.height !== undefined
        ? Math.max(60, Math.min(320, Math.round(partial.height)))
        : this.config.height,
      fps: partial.fps !== undefined
        ? (partial.fps === 30 || partial.fps === 120 ? partial.fps : 60)
        : this.config.fps,
    };

    this.saveToStorage();
    this.notify();
  }

  public resetToDefault() {
    this.config = { ...DEFAULT_VISUALIZER_CONFIG };
    this.saveToStorage();
    this.notify();
  }

  private saveToStorage() {
    try {
      if (typeof localStorage === 'undefined') return;
      localStorage.setItem(STORAGE_KEY_VISUALIZER_CONFIG, JSON.stringify(this.config));
    } catch (e) {
      console.warn('Failed to save visualizer config to storage:', e);
    }
  }

  private loadFromStorage() {
    try {
      if (typeof localStorage === 'undefined') return;
      const saved = localStorage.getItem(STORAGE_KEY_VISUALIZER_CONFIG);
      if (saved) {
        const parsed = JSON.parse(saved);
        if (parsed && typeof parsed === 'object') {
          this.config = {
            ...DEFAULT_VISUALIZER_CONFIG,
            ...parsed,
          };
        }
      }
    } catch (e) {
      console.warn('Failed to load visualizer config from storage:', e);
    }
  }
}

export const visualizerConfigManager = new VisualizerConfigManager();
