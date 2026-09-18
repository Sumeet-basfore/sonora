/**
 * Color utility functions for Sonora design tokens and theme engine.
 */

export interface RgbColor {
  r: number;
  g: number;
  b: number;
}

export interface RgbaColor extends RgbColor {
  a: number;
}

/**
 * Parse a hex string (#RGB, #RGBA, #RRGGBB, #RRGGBBAA) into an RGB object.
 */
export function hexToRgb(hex: string): RgbColor | null {
  const cleanHex = hex.trim().replace(/^#/, '');
  if (cleanHex.length === 3) {
    const r = parseInt(cleanHex[0] + cleanHex[0], 16);
    const g = parseInt(cleanHex[1] + cleanHex[1], 16);
    const b = parseInt(cleanHex[2] + cleanHex[2], 16);
    return isNaN(r) || isNaN(g) || isNaN(b) ? null : { r, g, b };
  }
  if (cleanHex.length === 6 || cleanHex.length === 8) {
    const r = parseInt(cleanHex.substring(0, 2), 16);
    const g = parseInt(cleanHex.substring(2, 4), 16);
    const b = parseInt(cleanHex.substring(4, 6), 16);
    return isNaN(r) || isNaN(g) || isNaN(b) ? null : { r, g, b };
  }
  return null;
}

/**
 * Convert RGB components (0-255) to a 6-character hex string.
 */
export function rgbToHex(r: number, g: number, b: number): string {
  const clamp = (val: number) => Math.max(0, Math.min(255, Math.round(val)));
  const toHex = (val: number) => clamp(val).toString(16).padStart(2, '0');
  return `#${toHex(r)}${toHex(g)}${toHex(b)}`;
}

/**
 * Convert hex color to CSS rgba string with given alpha (0.0 to 1.0).
 */
export function hexToRgba(hex: string, alpha: number): string {
  const rgb = hexToRgb(hex);
  if (!rgb) return hex;
  const clampedAlpha = Math.max(0, Math.min(1, alpha));
  return `rgba(${rgb.r}, ${rgb.g}, ${rgb.b}, ${clampedAlpha})`;
}

/**
 * Adjust color brightness by a factor (-1.0 to 1.0).
 * Positive increases brightness towards white, negative decreases towards black.
 */
export function adjustBrightness(hex: string, factor: number): string {
  const rgb = hexToRgb(hex);
  if (!rgb) return hex;

  let { r, g, b } = rgb;
  if (factor > 0) {
    r += (255 - r) * factor;
    g += (255 - g) * factor;
    b += (255 - b) * factor;
  } else {
    r += r * factor;
    g += g * factor;
    b += b * factor;
  }

  return rgbToHex(r, g, b);
}

/**
 * Calculate relative luminance (WCAG 2.1 formula).
 */
export function getRelativeLuminance(rgb: RgbColor): number {
  const toLinear = (c: number) => {
    const s = c / 255;
    return s <= 0.03928 ? s / 12.92 : Math.pow((s + 0.055) / 1.055, 2.4);
  };
  return 0.2126 * toLinear(rgb.r) + 0.7152 * toLinear(rgb.g) + 0.0722 * toLinear(rgb.b);
}

/**
 * Compute contrast ratio between two hex colors (1.0 to 21.0).
 */
export function getContrastRatio(hex1: string, hex2: string): number {
  const rgb1 = hexToRgb(hex1);
  const rgb2 = hexToRgb(hex2);
  if (!rgb1 || !rgb2) return 1.0;

  const l1 = getRelativeLuminance(rgb1);
  const l2 = getRelativeLuminance(rgb2);
  const lighter = Math.max(l1, l2);
  const darker = Math.min(l1, l2);
  return (lighter + 0.05) / (darker + 0.05);
}

/**
 * Generates interactive accent color derivatives from a primary hex color.
 */
export function generateAccentVariants(primaryHex: string) {
  return {
    '--accent-primary': primaryHex,
    '--accent-primary-hover': adjustBrightness(primaryHex, -0.12),
    '--accent-primary-active': adjustBrightness(primaryHex, -0.22),
    '--accent-glow': hexToRgba(primaryHex, 0.25),
    '--accent-subtle': hexToRgba(primaryHex, 0.12),
    '--border-focus': primaryHex,
    '--border-active': adjustBrightness(primaryHex, 0.20),
  };
}
