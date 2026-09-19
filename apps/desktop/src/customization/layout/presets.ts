/**
 * Built-in Sonora Layout Presets.
 */

import { type LayoutDefinition, LAYOUT_SCHEMA_URI } from './schema.ts';

export const LAYOUT_DEFAULT_STUDIO: LayoutDefinition = {
  $schema: LAYOUT_SCHEMA_URI,
  id: 'layout-default-studio',
  name: 'Modern Curator',
  description: 'Balanced 3-zone layout with navigation rail, central library canvas, right-hand now playing & context stage, and transport bar.',
  version: '1.0.0',
  isBuiltIn: true,
  regions: {
    sidebar: true,
    library: true,
    nowPlaying: true,
    queue: false,
    playbackBar: true,
    visualizer: false,
    lyrics: false,
  },
};

export const LAYOUT_MODERN_CURATOR = LAYOUT_DEFAULT_STUDIO;

export const LAYOUT_MINIMAL_PLAYER: LayoutDefinition = {
  $schema: LAYOUT_SCHEMA_URI,
  id: 'layout-minimal-player',
  name: 'Minimal Player',
  description: 'Distraction-free listening view hiding the sidebar and side panels for maximum focus on music.',
  version: '1.0.0',
  isBuiltIn: true,
  regions: {
    sidebar: false,
    library: true,
    nowPlaying: false,
    queue: false,
    playbackBar: true,
    visualizer: false,
    lyrics: false,
  },
};

export const LAYOUT_AUDIOPHILE_DECK: LayoutDefinition = {
  $schema: LAYOUT_SCHEMA_URI,
  id: 'layout-audiophile-deck',
  name: 'Audiophile Studio',
  description: 'High-density Columns UI track playlist, persistent active queue, and live DSP & CAVA spectrum analyzer dock.',
  version: '1.0.0',
  isBuiltIn: true,
  regions: {
    sidebar: true,
    library: true,
    nowPlaying: false,
    queue: true,
    playbackBar: true,
    visualizer: true,
    lyrics: false,
  },
};

export const LAYOUT_AUDIOPHILE_STUDIO = LAYOUT_AUDIOPHILE_DECK;

export const LAYOUT_LYRICS_STAGE: LayoutDefinition = {
  $schema: LAYOUT_SCHEMA_URI,
  id: 'layout-lyrics-stage',
  name: 'Atmospheric Theater',
  description: 'Full-bleed immersive canvas with animated cover artwork, glowing kinetic lyrics, and fluid visualizer.',
  version: '1.0.0',
  isBuiltIn: true,
  regions: {
    sidebar: false,
    library: false,
    nowPlaying: true,
    queue: false,
    playbackBar: true,
    visualizer: true,
    lyrics: true,
  },
};

export const LAYOUT_ATMOSPHERIC_THEATER = LAYOUT_LYRICS_STAGE;

export const BUILT_IN_LAYOUTS: LayoutDefinition[] = [
  LAYOUT_DEFAULT_STUDIO,
  LAYOUT_MINIMAL_PLAYER,
  LAYOUT_AUDIOPHILE_DECK,
  LAYOUT_LYRICS_STAGE,
];
