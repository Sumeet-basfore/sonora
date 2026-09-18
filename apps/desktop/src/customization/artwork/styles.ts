/**
 * Sonora Album Artwork Presentation Styles.
 * All styles consume the exact same artwork source URL.
 */

export type AlbumArtStyleId = 'classic' | 'minimal' | 'immersive' | 'blurred' | 'vinyl';

export interface AlbumArtStyleDefinition {
  id: AlbumArtStyleId;
  name: string;
  description: string;
}

export const ALBUM_ART_STYLES: AlbumArtStyleDefinition[] = [
  {
    id: 'classic',
    name: 'Classic Card',
    description: 'Crisp 1:1 square cover with smooth rounded corners and subtle drop shadow.',
  },
  {
    id: 'minimal',
    name: 'Minimal Flush',
    description: 'Borderless, razor-sharp square edges with clean, flat modern presentation.',
  },
  {
    id: 'immersive',
    name: 'Large Immersive',
    description: 'Expansive full-width hero presentation with cinematic gradient atmospheric overlay.',
  },
  {
    id: 'blurred',
    name: 'Blurred Ambient Glow',
    description: 'Ethereal ambient glow halo dynamically reflected from the album artwork behind the cover.',
  },
  {
    id: 'vinyl',
    name: 'Vinyl Gatefold',
    description: 'Die-cut jacket sleeve with a grooved vinyl LP disc sliding out and center spindle label.',
  },
];

const STORAGE_KEY_ARTWORK_STYLE = 'sonora_artwork_style_active';

export type ArtworkStyleChangeListener = (style: AlbumArtStyleId) => void;

export class ArtworkStyleManager {
  private activeStyle: AlbumArtStyleId = 'classic';
  private listeners: Set<ArtworkStyleChangeListener> = new Set();

  constructor() {
    this.loadFromStorage();
  }

  public subscribe(listener: ArtworkStyleChangeListener): () => void {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  }

  private notify() {
    for (const listener of this.listeners) {
      try {
        listener(this.activeStyle);
      } catch (e) {
        console.error('Error in artwork style listener:', e);
      }
    }
  }

  public getAvailableStyles(): AlbumArtStyleDefinition[] {
    return [...ALBUM_ART_STYLES];
  }

  public getActiveStyle(): AlbumArtStyleId {
    return this.activeStyle;
  }

  public setStyle(style: AlbumArtStyleId) {
    this.activeStyle = style;
    this.applyStyleToDom();
    this.saveToStorage();
    this.notify();
  }

  public resetToDefault() {
    this.setStyle('classic');
  }

  public applyStyleToDom() {
    const root = document.documentElement;
    if (root) {
      root.setAttribute('data-artwork-style', this.activeStyle);
    }
  }

  private saveToStorage() {
    try {
      if (typeof localStorage === 'undefined') return;
      localStorage.setItem(STORAGE_KEY_ARTWORK_STYLE, this.activeStyle);
    } catch (e) {
      console.warn('Failed to save artwork style to storage:', e);
    }
  }

  private loadFromStorage() {
    try {
      if (typeof localStorage === 'undefined') return;
      const saved = localStorage.getItem(STORAGE_KEY_ARTWORK_STYLE) as AlbumArtStyleId | null;
      if (saved && ALBUM_ART_STYLES.some(s => s.id === saved)) {
        this.activeStyle = saved;
      }
    } catch (e) {
      console.warn('Failed to load artwork style from storage:', e);
    }
    this.applyStyleToDom();
  }
}

export const artworkStyleManager = new ArtworkStyleManager();
