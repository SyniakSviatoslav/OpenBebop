// N8a Kalman filter — deterministic state estimation (RED+GREEN).

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { kalman1dStep, kalman1dInit, kalmanAnomaly } from './kalman.ts';

test('GREEN: a steady measurement is absorbed (innovation ≈ 0, no anomaly)', () => {
  let st = kalman1dInit();
  const cfg = { q: 0.01, r: 1 };
  for (let i = 0; i < 20; i++) st = kalman1dStep(st, 0.5, cfg).state;
  const { innovation } = kalman1dStep(st, 0.5, cfg);
  assert.ok(Math.abs(innovation) < 1e-6, `steady innovation should be ~0, got ${innovation}`);
  const an = kalmanAnomaly(st, 0.5, cfg, 3);
  assert.equal(an.anomalous, false);
});

test('RED: a sudden jump spikes the innovation → flagged anomalous', () => {
  let st = kalman1dInit();
  const cfg = { q: 0.01, r: 1 };
  for (let i = 0; i < 30; i++) st = kalman1dStep(st, 0.5, cfg).state; // settle around 0.5
  const settled = st.x;
  const an = kalmanAnomaly(st, settled + 10, cfg, 3); // jump +10σ
  assert.equal(an.anomalous, true, 'a 10σ jump must be flagged');
  assert.ok(Math.abs(an.innovation) > 3 * Math.sqrt(cfg.r), `innovation ${an.innovation} should exceed 3σ`);
});

test('GREEN: filter converges toward the true mean of a constant signal', () => {
  let st = kalman1dInit();
  const cfg = { q: 0.1, r: 4 };
  for (let i = 0; i < 200; i++) st = kalman1dStep(st, 3.0, cfg).state;
  assert.ok(Math.abs(st.x - 3.0) < 0.1, `posterior ${st.x} should converge to 3.0`);
});
