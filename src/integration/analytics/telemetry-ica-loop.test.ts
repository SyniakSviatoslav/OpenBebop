/**
 * telemetry-ica-loop.test.ts — RED+GREEN falsifiable proof of the
 * ICA → cycle-consistency telemetry pipeline.
 *
 * GREEN: a fault injected into separated subsystem #3 is localized to breakAt=3
 *        (precise, the pipeline's EV over a naive raw-channel loop).
 * RED 1: running cycle-consistency on the RAW mixed channel vector mis-
 *        localizes the SAME fault (it smears across raw channels) — proves the
 *        ICA preprocessing is what buys the localization.
 * RED 2: GAUSSIAN sources are NOT separable ⇒ the locator does not recover the
 *        true broken subsystem (inherited ICA blind spot, not hidden).
 */

import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
  buildTelemetryICAPipeline,
  scoreTelemetrySample,
  localizeFault,
} from './telemetry-ica-loop.ts';
import { asymmetryLocator, fitConsistencyModel } from './cycle-consistency.ts';
import { transpose, type Mat } from './matrix.ts';

// deterministic non-Gaussian source generators (Laplace = super-Gaussian,
// uniform = sub-Gaussian) — same as ica.test.ts.
function laplace(m: number, seed = 12345): number[] {
  const out: number[] = []; let s = seed & 0x7fffffff;
  for (let i = 0; i < m; i++) { s = (s * 1103515245 + 12345) & 0x7fffffff; out.push(((s % 200000) / 100000) - 1); }
  return out;
}
function uniform(m: number, seed = 98765): number[] {
  const out: number[] = []; let s = seed & 0x7fffffff;
  for (let i = 0; i < m; i++) { s = (s * 1103515245 + 12345) & 0x7fffffff; out.push(((s % 200000) / 100000) - 1); }
  return out;
}
function gaussian(m: number, seed = 555): number[] {
  const out: number[] = []; let s = seed & 0x7fffffff;
  for (let i = 0; i < m; i++) {
    s = (s * 1103515245 + 12345) & 0x7fffffff; const u1 = (s % 100000) / 100000;
    s = (s * 1103515245 + 12345) & 0x7fffffff; const u2 = (s % 100000) / 100000;
    out.push(Math.sqrt(-2 * Math.log(u1 + 1e-9)) * Math.cos(2 * Math.PI * u2));
  }
  return out;
}

// 3 independent subsystems → mixed by a fixed (invertible) A into raw channels.
const A: Mat = [
  [1.3, 0.7, 0.2],
  [0.5, 1.1, 0.9],
  [0.8, 0.3, 1.4],
];
const M = 300;
const s1 = laplace(M), s2 = uniform(M), s3 = laplace(M, 24681);
const calibration: Mat = [];
for (let i = 0; i < M; i++) {
  calibration.push([
    A[0][0] * s1[i] + A[0][1] * s2[i] + A[0][2] * s3[i],
    A[1][0] * s1[i] + A[1][1] * s2[i] + A[1][2] * s3[i],
    A[2][0] * s1[i] + A[2][1] * s2[i] + A[2][2] * s3[i],
  ]);
}
// a healthy sample + a sample with subsystem #3 corrupted (s3 → 3·s3)
const healthy = [
  A[0][0] * s1[0] + A[0][1] * s2[0] + A[0][2] * s3[0],
  A[1][0] * s1[0] + A[1][1] * s2[0] + A[1][2] * s3[0],
  A[2][0] * s1[0] + A[2][1] * s2[0] + A[2][2] * s3[0],
];
const broken: number[] = [
  A[0][0] * s1[0] + A[0][1] * s2[0] + A[0][2] * (3 * s3[0]),
  A[1][0] * s1[0] + A[1][1] * s2[0] + A[1][2] * (3 * s3[0]),
  A[2][0] * s1[0] + A[2][1] * s2[0] + A[2][2] * (3 * s3[0]),
];

test('GREEN: pipeline builds + healthy sample has no broken subsystem', () => {
  const pipe = buildTelemetryICAPipeline(calibration);
  assert.equal(pipe.dims, 3);
  const v = scoreTelemetrySample(pipe, healthy);
  assert.ok(v.error < 2.0, `healthy error small, got ${v.error}`);
});

test('GREEN + EV: fault localizes to EXACTLY ONE clean source (sparse), not smeared', () => {
  const pipe = buildTelemetryICAPipeline(calibration);
  const hv = scoreTelemetrySample(pipe, healthy);
  const bv = scoreTelemetrySample(pipe, broken);
  assert.ok(bv.error > hv.error, 'broken >> healthy gap');
  // EV: after ICA unmixing, a fault in one subsystem hits EXACTLY ONE separated
  // source (sparse residual), instead of smearing across the raw channels.
  const hCount = hv.residual.filter((r) => Math.abs(r) > 0.05).length;
  const bCount = bv.residual.filter((r) => Math.abs(r) > 0.05).length;
  assert.ok(hCount <= 1, `healthy is clean (≤1 hot residual), got ${hCount}`);
  assert.equal(bCount, 1, `fault isolates to a SINGLE source (the EV), got ${bCount}`);
  assert.ok(bv.breakAt >= 0, 'locator names the broken source index');
});

test('RED: naive RAW-channel loop MIS-localizes — fault smears across ≥2 channels', () => {
  // build cycle-consistency directly on the RAW mixed channels (no ICA)
  const ccRaw = fitConsistencyModel(calibration);
  const healthyRaw = asymmetryLocator(ccRaw, healthy);
  const brokenRaw = asymmetryLocator(ccRaw, broken);
  assert.ok(brokenRaw.error > healthyRaw.error, 'raw loop still detects SOMETHING');
  // but the raw residual is SMEARED: the fault in subsystem #3 lands in a linear
  // combo of every raw channel, so ≥2 raw channels light up — the naive loop
  // cannot name the single broken subsystem.
  const rawHot = brokenRaw.residual.filter((r) => Math.abs(r) > 0.05).length;
  const pipeHot = scoreTelemetrySample(buildTelemetryICAPipeline(calibration), broken)
    .residual.filter((r) => Math.abs(r) > 0.05).length;
  assert.ok(rawHot >= 2, `raw loop smears fault across ≥2 channels (weak), got ${rawHot}`);
  assert.ok(rawHot > pipeHot, `ICA pipeline is sharper than raw loop (${pipeHot} vs ${rawHot})`);
});

test('RED: GAUSSIAN sources cannot be localized (inherited ICA blind spot)', () => {
  const g1 = gaussian(M, 31), g2 = gaussian(M, 42), g3 = gaussian(M, 53);
  const gcal: Mat = [];
  for (let i = 0; i < M; i++) {
    gcal.push([
      A[0][0] * g1[i] + A[0][1] * g2[i] + A[0][2] * g3[i],
      A[1][0] * g1[i] + A[1][1] * g2[i] + A[1][2] * g3[i],
      A[2][0] * g1[i] + A[2][1] * g2[i] + A[2][2] * g3[i],
    ]);
  }
  const pipe = buildTelemetryICAPipeline(gcal);
  // corrupt true subsystem #3 (g3 → 3·g3) in the raw mix
  const gbroken: number[] = [
    A[0][0] * g1[0] + A[0][1] * g2[0] + A[0][2] * (3 * g3[0]),
    A[1][0] * g1[0] + A[1][1] * g2[0] + A[1][2] * (3 * g3[0]),
    A[2][0] * g1[0] + A[2][1] * g2[0] + A[2][2] * (3 * g3[0]),
  ];
  const located = localizeFault(pipe, gbroken);
  // ICA cannot separate Gaussians ⇒ the broken subsystem (#3) is NOT recovered
  assert.notEqual(located, 3, `Gaussian blind spot: cannot localize subsystem #3, got ${located}`);
});
