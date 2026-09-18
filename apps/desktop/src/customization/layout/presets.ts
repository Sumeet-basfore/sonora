/**
 * Built-in Sonora Layout Presets.
 */

import { type LayoutDefinition, LAYOUT_SCHEMA_URI } from './schema.ts';

export const LAYOUT_DEFAULT_STUDIO: LayoutDefinition = {
  $schema: LAYOUT_SCHEMA_URI,
  id: 'layout-default-studio',
  name: 'Default Studio',
  description: 'Standard desktop workspace with navigation sidebar, central library grid, and bottom transport bar.',
  version: '1.0.0',
  isBuiltIn: true,
  regions: {
    sidebar: true,
    library: true,
    queue: false,
    playbackBar: true,
    visualizer: false,
    lyrics: false,
  },
};

export const LAYOUT_MINIMAL_PLAYER: LayoutDefinition = {
  $schema: LAYOUT_SCHEMA_URI,
  id: 'layout-minimal-player',
  name: 'Minimal Player',
  description: 'Distraction-free listening view hiding the sidebar and side panels for maximum focus.',
  version: '1.0.0',
  isBuiltIn: true,
  regions: {
    sidebar: false,
    library: true,
    queue: false,
    playbackBar: true,
    visualizer: false,
    lyrics: false,
  },
};

export const LAYOUT_AUDIOPHILE_DECK: LayoutDefinition = {
  $schema: LAYOUT_SCHEMA_URI,
  id: 'layout-audiophile-deck',
  name: 'Audiophile Deck',
  description: 'All instrumentation enabled simultaneously: persistent active queue, dockable spectrum visualizer, and navigation.',
  version: '1.0.0',
  isBuiltIn: true,
  regions: {
    sidebar: true,
    library: true,
    queue: true,
    playbackBar: true,
    visualizer: true,
    lyrics: false,
  },
};

export const LAYOUT_LYRICS_STAGE: LayoutDefinition = {
  $schema: LAYOUT_SCHEMA_URI,
  id: 'layout-lyrics-stage',
  name: 'Lyrics & Visualizer Stage',
  description: 'Immersive stage focusing solely on real-time synchronized lyrics, audio visualizer, and playback transport.',
  version: '1.0.0',
  isBuiltIn: true,
  regions: {
    sidebar: false,
    library: false,
    queue: false,
    playbackBar: true,
    visualizer: true,
    lyrics: true,
  },
};

export const BUILT_IN_LAYOUTS: LayoutDefinition[] = [
  LAYOUT_DEFAULT_STUDIO,
  LAYOUT_MINIMAL_PLAYER,
  LAYOUT_AUDIOPHILE_DECK,
  LAYOUT_LYRICS_STAGE,
];
