/**
 * Shared HTML escaper for untrusted strings (track metadata from audio
 * files, theme/registry data, lyrics) interpolated into innerHTML templates.
 * Prefer textContent where possible; use this where HTML is required.
 */
export function escapeHtml(str: string): string {
  return str
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#039;');
}
