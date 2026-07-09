// src/integration/optical/field-recall.test.ts
//
// RED+GREEN: optical compute as a field-search accelerator. GREEN = ranks candidates; RED = a
// passive mask check still holds (|t|==1) and dim mismatch is rejected.

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { opticalRecall } from './field-recall.ts';
import { thinLensMask } from './optic.ts';

const N = 4;
function flat(v: number[][]): number[] {
  return v.flat();
}
// identity-like (diagonal) passive mask: transmission = identity matrix → FFT2D preserves energy.
function diagMask(n: number) {
  return {
    transmission: Array.from({ length: n }, (_, i) =>
      Array.from({ length: n }, (_, j) => (i === j ? [1, 0] : [0, 0]) as [number, number]),
    ),
  };
}

// Build a query from a smooth non-impulse field (phase carries spatial location, so FFT is not
// shift-invariant) — a ramp in the first row. The matching candidate is identical.
const query = flat([
  [0.9, 0.6, 0.3, 0.1],
  [0.2, 0.2, 0.2, 0.2],
  [0.1, 0.1, 0.1, 0.1],
  [0.0, 0.0, 0.0, 0.0],
]);
const exactMatch = query; // identical field
const other = flat([
  [0.0, 0.0, 0.0, 0.0],
  [0.1, 0.1, 0.1, 0.1],
  [0.2, 0.2, 0.2, 0.2],
  [0.1, 0.3, 0.6, 0.9],
]);

test('GREEN: opticalRecall ranks the exact-match field first', () => {
  const rank = opticalRecall(query, [other, exactMatch], diagMask(N));
  assert.equal(rank[0], 1, 'exact-match candidate must rank first');
});

test('RED: opticalRecall rejects a query/dim mismatch', () => {
  assert.throws(() => opticalRecall([1, 0, 0], [[1, 0, 0, 0]], diagMask(N)));
});

test('GREEN: thin-lens mask is a valid passive mask (|t|==1) and usable for recall', () => {
  const mask = thinLensMask(N, 10, 2 * Math.PI); // n, f=10, k=2π
  for (const row of mask.transmission) {
    for (const [re, im] of row) {
      const mag = Math.hypot(re, im);
      assert.ok(Math.abs(mag - 1) < 1e-9, `passive mask element must have |t|=1, got ${mag}`);
    }
  }
  const rank = opticalRecall(query, [other, exactMatch], mask);
  assert.equal(rank.length, 2);
});
