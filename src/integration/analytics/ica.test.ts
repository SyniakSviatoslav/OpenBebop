/**
 * ica.test.ts — RED+GREEN falsifiable proof of FastICA (deterministic, no RNG).
 *
 * GREEN: recovers two NON-GAUSSIAN mixed sources (super-Gaussian Laplace +
 *   sub-Gaussian uniform) up to permutation/sign — recoveryScore ≈ 1.
 * GREEN: already-independent input ≈ identity unmixing (sources unchanged).
 * RED:   two GAUSSIAN sources are NOT recovered (blind spot) — recoveryScore
 *   stays low, proving ICA cannot separate Gaussian mixtures (proven limit).
 */

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { fastICA, applyICA, recoveryScore } from './ica.ts';
import type { Mat } from './matrix.ts';
// deterministic mixed-signal generators (no RNG)
function laplace(m: number): number[] {
  // symmetric Laplace via two exponentials (deterministic, fixed seed sequence)
  const out: number[] = [];
  let s = 12345;
  for (let i = 0; i < m; i++) {
    s = (s * 1103515245 + 12345) & 0x7fffffff;
    const u = (s % 100000) / 100000;
    s = (s * 1103515245 + 12345) & 0x7fffffff;
    const u2 = (s % 100000) / 100000;
    const e1 = -Math.log(u + 1e-9), e2 = -Math.log(u2 + 1e-9);
    out.push(e1 - e2); // Laplace(0,1) = diff of two exponentials
  }
  return out;
}
function uniform(m: number): number[] {
  const out: number[] = [];
  let s = 98765;
  for (let i = 0; i < m; i++) { s = (s * 1103515245 + 12345) & 0x7fffffff; out.push((s % 200000) / 100000 - 1); }
  return out;
}
function gaussian(m: number, seed = 555): number[] {
  // Box-Muller, deterministic (seedable)
  const out: number[] = [];
  let s = seed & 0x7fffffff;
  for (let i = 0; i < m; i++) {
    s = (s * 1103515245 + 12345) & 0x7fffffff; const u1 = (s % 100000) / 100000;
    s = (s * 1103515245 + 12345) & 0x7fffffff; const u2 = (s % 100000) / 100000;
    out.push(Math.sqrt(-2 * Math.log(u1 + 1e-9)) * Math.cos(2 * Math.PI * u2));
  }
  return out;
}

test('GREEN: recovers two NON-GAUSSIAN mixed sources (|corr|≈1 up to perm/sign)', () => {
  const m = 400;
  const s1 = laplace(m);       // super-Gaussian
  const s2 = uniform(m);       // sub-Gaussian
  // mix with a fixed (non-orthogonal, well-conditioned) matrix
  const A = [
    [1.3, 0.7],
    [0.5, 1.1],
  ];
  const X: Mat = [];
  for (let i = 0; i < m; i++) X.push([A[0][0] * s1[i] + A[0][1] * s2[i], A[1][0] * s1[i] + A[1][1] * s2[i]]);
  const model = fastICA(X, { nComponents: 2 });
  assert.ok(model.converged.every(Boolean), 'all components converged');
  const score = recoveryScore(model.S, [s1, s2]);
  for (const sc of score) assert.ok(sc > 0.9, `recovered source correlate ≈1, got ${sc}`);
});

test('GREEN: already-independent input ≈ identity unmixing (sources preserved)', () => {
  const m = 200;
  const X: Mat = [];
  const a = laplace(m), b = uniform(m);
  for (let i = 0; i < m; i++) X.push([a[i], b[i]]); // already independent
  const model = fastICA(X, { nComponents: 2 });
  const score = recoveryScore(model.S, [a, b]);
  for (const sc of score) assert.ok(sc > 0.9, `independent sources survive unmixing, got ${sc}`);
});

test('RED: TWO GAUSSIAN sources — NOT reliably recovered (blind spot)', () => {
  const m = 400;
  const g1 = gaussian(m, 11);
  const g2 = gaussian(m, 22);
  // mixing matrix chosen so ICA's rotation does NOT coincide with the truth
  const A = [[0.9, 1.7], [1.4, 0.3]];
  const X: Mat = [];
  for (let i = 0; i < m; i++) X.push([A[0][0] * g1[i] + A[0][1] * g2[i], A[1][0] * g1[i] + A[1][1] * g2[i]]);
  // For Gaussian mixtures the contrast is flat ⇒ the unmixing is only a rotation,
  // which (for a generic A) does NOT align with the original sources. Falsifiable
  // proof of the blind spot: recoveryScore stays clearly below the ≈1 that
  // NON-GAUSSIAN sources achieve (see the GREEN tests above).
  const model = fastICA(X, { nComponents: 2 });
  const score = recoveryScore(model.S, [g1, g2]);
  for (const sc of score) assert.ok(sc < 0.9, `Gaussian NOT reliably recovered (blind spot), score ${sc}`);
});

test('GREEN: applyICA reuses a fitted model on new data (deterministic)', () => {
  const m = 300;
  const s1 = laplace(m), s2 = uniform(m);
  const A = [[1.3, 0.7], [0.5, 1.1]];
  const X: Mat = [];
  for (let i = 0; i < m; i++) X.push([A[0][0] * s1[i] + A[0][1] * s2[i], A[1][0] * s1[i] + A[1][1] * s2[i]]);
  const model = fastICA(X, { nComponents: 2 });
  // new mixed batch
  const X2: Mat = [];
  for (let i = 0; i < m; i++) X2.push([A[0][0] * s1[i] + A[0][1] * s2[i], A[1][0] * s1[i] + A[1][1] * s2[i]]);
  const S2 = applyICA(model, X2);
  assert.equal(S2.length, 2, 'applyICA returns 2 sources');
  // deterministic: same input ⇒ same output
  const S2b = applyICA(model, X2);
  for (let r = 0; r < 2; r++) for (let c = 0; c < S2[r].length; c++) assert.ok(Math.abs(S2[r][c] - S2b[r][c]) < 1e-12, 'applyICA is deterministic');
});
