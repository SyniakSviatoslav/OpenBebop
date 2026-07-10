/**
 * loss.test.ts — RED+GREEN falsifiable tests for the robust-loss primitives
 * (Huber / MSE / Quantile / Focal). These are the building blocks the L5
 * prompt asked for; deterministic, no deps.
 *
 * GREEN: each loss has the textbook value on a known input.
 * RED:   out-of-domain input (NaN, δ≤0, τ∉(0,1), p∉[0,1]) is rejected.
 */

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { huber, mse, quantileLoss, focalLoss } from './loss.ts';

// ── Huber ──
test('GREEN: huber = ½e² inside the δ band (e=0.5, δ=1)', () => {
  assert.ok(Math.abs(huber(0.5, 1) - 0.125) < 1e-12);
});
test('GREEN: huber = δ(|e|−½δ) outside the band (e=3, δ=1)', () => {
  assert.ok(Math.abs(huber(3, 1) - 1 * (3 - 0.5)) < 1e-12); // = 2.5
});
test('RED: huber rejects non-finite error', () => {
  assert.throws(() => huber(NaN));
});
test('RED: huber rejects δ≤0', () => {
  assert.throws(() => huber(1, 0));
});

// ── MSE ──
test('GREEN: mse of [1,−1,0] = 2/3', () => {
  assert.ok(Math.abs(mse([1, -1, 0]) - 2 / 3) < 1e-12);
});
test('RED: mse of empty throws', () => {
  assert.throws(() => mse([]));
});

// ── Quantile (pinball) ──
test('GREEN: quantile loss at τ=0.5 equals ½|e| (symmetric)', () => {
  assert.ok(Math.abs(quantileLoss(10, 8, 0.5) - 1) < 1e-12); // e=2, τ·e=1
  assert.ok(Math.abs(quantileLoss(8, 10, 0.5) - 1) < 1e-12); // e=−2, (τ−1)·e=1
});
test('GREEN: τ>0.5 penalizes under-prediction harder', () => {
  const under = quantileLoss(10, 8, 0.9); // actual>pred, e=+2
  const over = quantileLoss(8, 10, 0.9); // actual<pred, e=−2
  assert.ok(under > over, 'under-prediction should cost more at τ=0.9');
});
test('RED: quantile rejects τ outside (0,1)', () => {
  assert.throws(() => quantileLoss(1, 1, 0));
  assert.throws(() => quantileLoss(1, 1, 1));
});

// ── Focal ──
test('GREEN: focal loss → 0 as p→1 (confident correct)', () => {
  assert.ok(Math.abs(focalLoss(0.999, 2)) < 1e-3);
});
test('GREEN: focal loss is HIGHER for a less-confident correct class (γ focus)', () => {
  const easy = focalLoss(0.9, 2);
  const hard = focalLoss(0.6, 2);
  assert.ok(hard > easy, 'focal should weight the hard (low-p) example more');
});
test('RED: focal rejects p outside [0,1]', () => {
  assert.throws(() => focalLoss(1.5, 2));
});
test('RED: focal = +∞ when pTrueClass = 0 (certain wrong)', () => {
  assert.equal(focalLoss(0, 2), Infinity);
});
