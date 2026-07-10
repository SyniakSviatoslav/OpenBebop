/**
 * telemetry-shadow.test.ts — RED+GREEN falsifiable proof of the Dowiz telemetry shadow harness.
 *
 * GREEN: a known-good row reports no fault + no drift; a subsystem burst localizes to its index.
 * RED:   (a) a row whose width mismatches calibration is rejected, not silently mis-scored;
 *        (b) a slow structural shift of the telemetry mean past driftK·σ flips `drift` (the
 *            shadow catches distribution drift the point-fault locator alone would miss);
 *        (c) empty calibration is rejected (cannot base a gate on nothing).
 */

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { buildTelemetryShadow } from './telemetry-shadow.ts';

const CALIB: number[][] = [];
for (let k = 0; k < 200; k++) {
  const s0 = Math.sin(k / 20);
  const s1 = k % 17 === 0 ? 1 : 0;
  CALIB.push([s0 + 0.3 * s1, 0.8 * s0 + s1]);
}
const ON = [Math.sin(2.5), 0.8 * Math.sin(2.5)]; // on-manifold known-good

test('GREEN: known-good row → no fault, no drift', () => {
  const sh = buildTelemetryShadow(CALIB);
  for (let k = 0; k < 12; k++) sh.ingest(ON);
  const rep = sh.ingest(ON);
  assert.equal(rep.subsystemFault, -1, 'on-manifold row must not localize a fault');
  assert.equal(rep.drift, false, 'stable telemetry must not report drift');
  assert.equal(rep.driftSeverity, 0, 'on-baseline severity is 0');
});

test('GREEN: a subsystem burst localizes to its index', () => {
  const sh = buildTelemetryShadow(CALIB);
  for (let k = 0; k < 12; k++) sh.ingest(ON);
  const rep = sh.ingest([ON[0] + 1.5, ON[1] + 5]); // sharp burst in source 1
  assert.ok(rep.subsystemFault >= 0, `burst must localize, got ${rep.subsystemFault}`);
});

test('RED: width-mismatched row is rejected (not silently mis-scored)', () => {
  const sh = buildTelemetryShadow(CALIB);
  assert.throws(() => sh.ingest([1, 2, 3]), /width mismatch/, 'wrong-width row must throw');
});

test('RED: slow structural drift of the telemetry mean flips drift past driftK·σ', () => {
  const sh = buildTelemetryShadow(CALIB, { driftK: 3 });
  // feed stable on-manifold rows to settle the EMA baseline
  for (let k = 0; k < 30; k++) sh.ingest(ON);
  assert.equal(sh.ingest(ON).drift, false, 'steady state: no drift yet');
  // now slowly shift the whole telemetry mean upward by ~6σ over many rows
  for (let k = 0; k < 200; k++) sh.ingest([ON[0] + 6, ON[1] + 6]);
  const rep = sh.ingest([ON[0] + 6, ON[1] + 6]);
  assert.equal(rep.drift, true, 'a sustained mean shift must be caught as drift');
  assert.ok(rep.driftSeverity > 0, 'drift must carry a severity');
});

test('RED: empty calibration is rejected (cannot base a gate on nothing)', () => {
  assert.throws(() => buildTelemetryShadow([]), /empty calibration/, 'empty calibration must throw');
});
