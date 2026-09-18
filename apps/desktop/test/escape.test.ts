import test from 'node:test';
import assert from 'node:assert';

import { escapeHtml } from '../src/escape.ts';

test('escapeHtml: escapes markup-significant characters', () => {
  assert.strictEqual(
    escapeHtml('<img src=x onerror=alert(1)>'),
    '&lt;img src=x onerror=alert(1)&gt;'
  );
  assert.strictEqual(escapeHtml('Fish & "Chips"'), 'Fish &amp; &quot;Chips&quot;');
  assert.strictEqual(escapeHtml("it's"), 'it&#039;s');
});

test('escapeHtml: leaves safe text untouched', () => {
  assert.strictEqual(escapeHtml('Get Lucky'), 'Get Lucky');
  assert.strictEqual(escapeHtml('R&B / Hip-Hop (2024)'), 'R&amp;B / Hip-Hop (2024)');
  assert.strictEqual(escapeHtml(''), '');
});

test('escapeHtml: neutralizes attribute breakouts', () => {
  const evil = 'x" onmouseover="alert(1)';
  const escaped = escapeHtml(evil);
  assert.ok(!escaped.includes('"'));
  assert.ok(escaped.includes('&quot;'));
});
