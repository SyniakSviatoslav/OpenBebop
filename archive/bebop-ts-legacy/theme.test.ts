import { test } from 'node:test';
import assert from 'node:assert/strict';
import { accentHexFor, makePaint } from './theme.ts';

// The looks axis selects a real accent per value. Tested via the pure resolver (no TTY needed).
test('GREEN: accentHexFor(looks) yields a distinct hex per axis', () => {
  const hexes = new Set([
    accentHexFor('bebop'),
    accentHexFor('claude'),
    accentHexFor('opencode'),
    accentHexFor('codex'),
  ]);
  assert.equal(hexes.size, 4, `each looks axis should yield a distinct accent, got ${hexes.size}`);
});

test('GREEN: custom looks reads BEBOP_THEME_ACCENT env (hex)', () => {
  process.env.BEBOP_THEME_ACCENT = '#FF0000';
  assert.equal(accentHexFor('custom'), '#FF0000', 'custom accent should use the env hex');
  delete process.env.BEBOP_THEME_ACCENT;
});

test('GREEN: unknown looks falls back to bebop accent', () => {
  assert.equal(accentHexFor('nonsense'), accentHexFor('bebop'), 'unknown looks must fall back to bebop');
});

test('GREEN: makePaint emits the accent only in a TTY (else plain, never crashes)', () => {
  // non-TTY: paint returns the text unchanged (graceful degradation)
  const plain = makePaint('claude').teal('x');
  assert.equal(plain, 'x', 'non-TTY must not emit ANSI');
});
