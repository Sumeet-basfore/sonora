/**
 * Sonora Theme System Schema and Typed Models.
 * Strictly decoupled from audio and hardware logic.
 */

export const THEME_SCHEMA_URI = 'https://sonora.audio/schemas/v1/theme.json';

export type ThemeMode = 'dark' | 'light';

export interface ThemeTokens {
  // Surfaces & Canvas
  '--bg-base': string;
  '--bg-surface': string;
  '--bg-surface-hover': string;
  '--bg-elevated': string;
  '--bg-card': string;
  '--bg-card-hover': string;
  '--bg-active': string;
  '--bg-glass': string;

  // Accents & Brand
  '--accent-primary': string;
  '--accent-primary-hover': string;
  '--accent-primary-active': string;
  '--accent-secondary': string;
  '--accent-secondary-hover': string;
  '--accent-glow': string;
  '--accent-subtle': string;

  // Typography & Content
  '--text-primary': string;
  '--text-secondary': string;
  '--text-muted': string;
  '--text-disabled': string;
  '--text-inverse': string;

  // Borders & Dividers
  '--border-subtle': string;
  '--border-medium': string;
  '--border-focus': string;
  '--border-active': string;

  // Semantic Feedback States
  '--state-success': string;
  '--state-warning': string;
  '--state-danger': string;
  '--state-info': string;

  // Optional ambient shader or visualizer tuning
  '--glow-intensity'?: string;
  [key: string]: string | undefined;
}

export interface ThemeDefinition {
  $schema?: string;
  id: string;
  name: string;
  description?: string;
  version: string;
  author: string;
  mode: ThemeMode;
  isBuiltIn?: boolean;
  tokens: ThemeTokens;
}

/**
 * Validation result for theme schema checks.
 */
export interface ValidationResult {
  valid: boolean;
  errors: string[];
}

/**
 * Required token keys in a valid Sonora theme.
 */
export const REQUIRED_THEME_TOKENS: (keyof ThemeTokens)[] = [
  '--bg-base',
  '--bg-surface',
  '--bg-surface-hover',
  '--bg-elevated',
  '--bg-card',
  '--bg-card-hover',
  '--bg-active',
  '--bg-glass',
  '--accent-primary',
  '--accent-primary-hover',
  '--accent-primary-active',
  '--accent-secondary',
  '--accent-secondary-hover',
  '--accent-glow',
  '--accent-subtle',
  '--text-primary',
  '--text-secondary',
  '--text-muted',
  '--text-disabled',
  '--text-inverse',
  '--border-subtle',
  '--border-medium',
  '--border-focus',
  '--border-active',
  '--state-success',
  '--state-warning',
  '--state-danger',
  '--state-info',
];

/**
 * Validates whether an unknown object conforms to the ThemeDefinition schema.
 */
export function validateThemeSchema(obj: unknown): ValidationResult {
  const errors: string[] = [];

  if (!obj || typeof obj !== 'object') {
    return { valid: false, errors: ['Theme root must be a non-null JSON object'] };
  }

  const theme = obj as Record<string, unknown>;

  if (typeof theme.id !== 'string' || !theme.id.trim()) {
    errors.push('Theme must have a non-empty string "id"');
  }

  if (typeof theme.name !== 'string' || !theme.name.trim()) {
    errors.push('Theme must have a non-empty string "name"');
  }

  if (typeof theme.version !== 'string' || !theme.version.trim()) {
    errors.push('Theme must specify a string "version" (e.g. "1.0.0")');
  }

  if (typeof theme.author !== 'string') {
    errors.push('Theme must specify a string "author"');
  }

  if (theme.mode !== 'dark' && theme.mode !== 'light') {
    errors.push('Theme mode must be either "dark" or "light"');
  }

  if (!theme.tokens || typeof theme.tokens !== 'object') {
    errors.push('Theme must have a "tokens" dictionary object');
  } else {
    const tokens = theme.tokens as Record<string, unknown>;
    for (const reqToken of REQUIRED_THEME_TOKENS) {
      if (typeof tokens[reqToken] !== 'string' || !(tokens[reqToken] as string).trim()) {
        errors.push(`Missing or invalid required design token: "${reqToken}"`);
      }
    }
  }

  return {
    valid: errors.length === 0,
    errors,
  };
}

/**
 * Type guard for ThemeDefinition.
 */
export function isThemeDefinition(obj: unknown): obj is ThemeDefinition {
  return validateThemeSchema(obj).valid;
}
