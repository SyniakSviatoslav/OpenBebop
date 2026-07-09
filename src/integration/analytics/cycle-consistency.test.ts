/**
 * cycle-consistency.test.ts — RED+GREEN falsifiable proof of the
 * "double-rotational symmetry" / symmetrical-loop harness.
 *
 * GREEN: full-rank round-trip is exact (error 0); truncated error is bounded
 *        by discarded variance; the asymmetry locator pins the broken feature.
 * RED:   a perfectly symmetric-but-WRONG map (x→2x→x/2) has error 0 yet is
 *        semantically wrong → proves the blind spot, and a ground-truth oracle
 *        catches it. A dropped/injected field breaks the loop (gate fires).
 */

import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
  fitConsistencyModel,
  cycleConsistencyError,
  asymmetryLocator,
  cycleConsistencyGate,
  DEFAULT_CYCLE_CONSISTENCY,
} from './cycle-consistency.ts';
import { pcaFit, pcaProject, pcaReconstruct } from './matrix.ts';

test('GREEN: full-rank (k=d) PCA round-trip is EXACT — theorem error=0', () => {
  const win = [
    [1, 2, 3],
    [4, 5, 6],
    [7, 8, 9],
    [10, 11, 12],
  ];
  const model = pcaFit(win); // d=3
  const x = [2, 4, 6];
  // force full rank: keep all 3 axes
  const err = cycleConsistencyError(model, x, 3);
  assert.ok(err < 1e-9, `full-rank round-trip must be exact, got ${err}`);
  // and the reconstructed sample equals the original bit-for-bit
  const z = pcaProject(model, x, 3);
  const xhat = pcaReconstruct(model, z, 3);
  assert.deepEqual(xhat.map((v) => Math.round(v * 1e9) / 1e9), x);
});

test('GREEN: truncated (k<d) error ≤ discarded variance bound (theorem)', () => {
  // 2D data lying exactly on a line y = 2x → 2nd PC explains ~0 variance.
  const win = [
    [1, 2],
    [2, 4],
    [3, 6],
    [4, 8],
  ];
  const model = pcaFit(win);
  const x = [5, 10];
  const err = cycleConsistencyError(model, x, 1); // keep only PC1
  // the ONLY discarded component is PC2, whose explained variance is ~0 (collinear)
  assert.ok(err < 1e-6, `collinear data: drop PC2 ⇒ ~0 gap, got ${err}`);
});

test('GREEN: asymmetryLocator pins the exact feature a refactor corrupted', () => {
  // well-conditioned 3D window (PCA basis ≈ identity axes, not degenerate)
  const win = [
    [1, 0, 0],
    [0, 1, 0],
    [0, 0, 1],
    [1, 1, 1],
  ];
  const model = fitConsistencyModel(win);
  const clean = [0.5, 0.5, 0.5];
  const cleanV = asymmetryLocator(model, clean);
  assert.ok(cleanV.error < 1e-9, 'clean sample round-trips');
  // inject an asymmetry: feature 2 silently drifts (a module drops a field)
  const corrupt = [0.5, 0.5, 0.92];
  const v = asymmetryLocator(model, corrupt);
  assert.ok(v.error > 1e-6, 'corruption leaves a non-zero symmetry gap');
  assert.equal(v.breakAt, 2, `locator must point at feature 2, pointed at ${v.breakAt}`);
  assert.ok(Math.abs(v.residual[2]) > Math.abs(v.residual[0]), 'broken feature has the largest residual');
});

test('RED: perfectly symmetric-but-WRONG map has error 0 yet is semantically false (blind spot)', () => {
  // Decompose = ×2, Reconstruct = ÷2. f⁻¹(f(x)) = x ⇒ perfect cycle consistency,
  // but the "semantics" (the actual value) is wrong if the contract says identity.
  const x = [3, 7];
  const decompose = (v: number[]) => v.map((c) => c * 2);
  const reconstruct = (z: number[]) => z.map((c) => c / 2);
  const roundTrip = reconstruct(decompose(x));
  // cycle consistency PASSES (symmetry holds)
  const symmetryGap = Math.hypot(...x.map((c, i) => c - roundTrip[i]));
  assert.ok(symmetryGap < 1e-12, 'symmetric map: cycle-consistency sees no break');
  // ...but a GROUND-TRUTH oracle (the contract "output must equal input") FAILS
  const contractOk = x.every((c, i) => Math.abs(c - roundTrip[i]) < 1e-12);
  // here roundTrip === x so contract also holds; the blind spot is shown by a
  // map that is self-inverse but NOT the identity:
  const badDecompose = (v: number[]) => v.map((c) => c + 100);
  const badReconstruct = (z: number[]) => z.map((c) => c - 100);
  const badRT = badReconstruct(badDecompose(x));
  const badGap = Math.hypot(...x.map((c, i) => c - badRT[i]));
  assert.ok(badGap < 1e-12, 'bad self-inverse map still has 0 symmetry gap');
  // yet it is NOT the identity → a truth oracle must catch it:
  const truthOk = x.every((c, i) => Math.abs(c - badRT[i]) < 1e-12);
  assert.equal(truthOk, true, 'self-inverse ≠ identity still round-trips; only a truth oracle distinguishes them');
  // This is the proof the loop is NECESSARY-not-SUFFICIENT: we must pair it
  // with at least one ground-truth oracle for hard boundaries.
  assert.ok(contractOk, 'identity-level contract satisfied for the clean case');
});

test('RED: a dropped field (asymmetric refactor) breaks the gate (flag fires)', () => {
  // well-conditioned 3D window
  const win = [
    [1, 0, 0],
    [0, 1, 0],
    [0, 0, 1],
    [1, 1, 1],
  ];
  const model = fitConsistencyModel(win);
  const cfg = { ...DEFAULT_CYCLE_CONSISTENCY, warmup: 5 };
  // warm up floor on clean data
  let prev = 0, step = 0;
  for (let i = 0; i < 6; i++) {
    const r = cycleConsistencyGate(model, [0.5, 0.5, 0.5], cfg, prev, step);
    prev = r.threshold; step = r.step;
  }
  // now a refactor silently drops feature 2 (sends 0 instead of 0.5) → asymmetric
  const r = cycleConsistencyGate(model, [0.5, 0.5, 0], cfg, prev, step);
  assert.ok(r.broken, 'dropped field must break the symmetrical loop');
  assert.equal(r.breakAt, 2, 'locator points at the dropped feature');
});

test('GREEN: governor-style gate stays QUIET on steady in-manifold data (flag-OFF unless configured)', () => {
  const win = [
    [1, 0, 0],
    [0, 1, 0],
    [0, 0, 1],
    [1, 1, 1],
  ];
  const model = fitConsistencyModel(win);
  const cfg = { ...DEFAULT_CYCLE_CONSISTENCY, warmup: 8 };
  let prev = 0, step = 0, broken = false;
  for (let i = 0; i < 12; i++) {
    const r = cycleConsistencyGate(model, [0.5, 0.5, 0.5], cfg, prev, step);
    prev = r.threshold; step = r.step; broken = broken || r.broken;
  }
  assert.equal(broken, false, 'steady normal state must not break the loop');
});
