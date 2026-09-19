/**
 * Sonora Layout System Schema and Typed Models.
 * Defines UI regions as configurable, serializable components.
 */

export const LAYOUT_SCHEMA_URI = 'https://sonora.audio/schemas/v1/layout.json';

export type RegionId =
  | 'sidebar'
  | 'library'
  | 'nowPlaying'
  | 'queue'
  | 'playbackBar'
  | 'visualizer'
  | 'lyrics';

export interface RegionVisibility {
  sidebar: boolean;
  library: boolean;
  nowPlaying: boolean;
  queue: boolean;
  playbackBar: boolean;
  visualizer: boolean;
  lyrics: boolean;
}

export interface LayoutDefinition {
  $schema?: string;
  id: string;
  name: string;
  description: string;
  version: string;
  isBuiltIn?: boolean;
  regions: RegionVisibility;
}

export const ALL_REGION_IDS: RegionId[] = [
  'sidebar',
  'library',
  'nowPlaying',
  'queue',
  'playbackBar',
  'visualizer',
  'lyrics',
];

export interface LayoutValidationResult {
  valid: boolean;
  errors: string[];
}

export function validateLayoutSchema(obj: unknown): LayoutValidationResult {
  const errors: string[] = [];

  if (!obj || typeof obj !== 'object') {
    return { valid: false, errors: ['Layout definition must be a non-null object'] };
  }

  const layout = obj as Record<string, unknown>;

  if (typeof layout.id !== 'string' || !layout.id.trim()) {
    errors.push('Layout must have a non-empty string "id"');
  }

  if (typeof layout.name !== 'string' || !layout.name.trim()) {
    errors.push('Layout must have a non-empty string "name"');
  }

  if (typeof layout.version !== 'string' || !layout.version.trim()) {
    errors.push('Layout must have a string "version"');
  }

  if (!layout.regions || typeof layout.regions !== 'object') {
    errors.push('Layout must specify a "regions" object');
  } else {
    const regions = layout.regions as Record<string, unknown>;
    for (const regionId of ALL_REGION_IDS) {
      if (typeof regions[regionId] !== 'boolean') {
        errors.push(`Layout regions must specify a boolean for region "${regionId}"`);
      }
    }
  }

  return {
    valid: errors.length === 0,
    errors,
  };
}

export function isLayoutDefinition(obj: unknown): obj is LayoutDefinition {
  return validateLayoutSchema(obj).valid;
}
