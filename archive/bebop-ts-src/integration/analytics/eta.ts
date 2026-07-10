/**
 * eta.ts — Dowiz ETA forecaster with PREDICTION INTERVALS (operator directive 2026-07-08).
 *
 * Plain point-ETA (MSE) lies: a single number hides the spread. Quantile regression gives
 * honest INTERVALS — "won't exceed" bounds for ops. Huber gives a robust point estimate that
 * shrugs off telemetry spikes (one bad sensor reading must not yank the whole model).
 *
 * The model is a LINEAR quantile regressor, fit by FULL-BATCH gradient descent on the total
 * quantile loss. Deterministic: no RNG, no Date, fixed step + bounded epochs — same input ⇒
 * same weights. Features are standardized internally so GD is stable. The live seam
 * (`fitETAInterval`) is what Dowiz's apps/api calls; this module is the math, already proven
 * against reality by the RED+GREEN tests below.
 *
 * Verified-by-Math: every exported function has a falsifiable RED+GREEN test (eta.test.ts).
 */

import { huber, quantileLoss } from './loss.ts';

export interface ETAFeatures {
  /** descriptive features (e.g. queue depth, agent load, payload bytes). */
  x: number[];
}

export interface ETAModel {
  w: number[]; // slope per feature (in standardized space)
  b: number; // intercept (in standardized space)
  /** feature standardization (mean/std) for stable GD; predict re-standards. */
  mu: number[];
  sigma: number[];
  /** the quantile this model was fit to (the point is the τ-th conditional quantile). */
  tau: number;
}

export interface ETAInterval {
  point: number; // median (τ=0.5) point estimate
  lo: number; // lower bound (τ=0.05 fit)
  hi: number; // upper bound (τ=0.95 fit)
  /** true when `lo <= point <= hi` (a malformed fit can invert this — the test asserts it). */
  ordered: boolean;
}

function dot(w: number[], x: number[]): number {
  let s = 0;
  for (let i = 0; i < w.length; i++) s += w[i] * (x[i] ?? 0);
  return s;
}

function standardizeRow(x: number[], mu: number[], sigma: number[]): number[] {
  return x.map((v, i) => (sigma[i] > 1e-12 ? (v - mu[i]) / sigma[i] : 0));
}

function computeStats(pairs: { x: number[]; y: number }[]): { mu: number[]; sigma: number[] } {
  const d = pairs[0].x.length;
  const mu = new Array(d).fill(0);
  const sigma = new Array(d).fill(0);
  for (const p of pairs) for (let i = 0; i < d; i++) mu[i] += p.x[i] ?? 0;
  for (let i = 0; i < d; i++) mu[i] /= pairs.length;
  for (const p of pairs) for (let i = 0; i < d; i++) { const di = (p.x[i] ?? 0) - mu[i]; sigma[i] += di * di; }
  for (let i = 0; i < d; i++) sigma[i] = Math.sqrt(sigma[i] / pairs.length);
  return { mu, sigma };
}

/**
 * Fit ONE quantile-regression model by deterministic full-batch gradient descent on total
 * quantile loss. `lr` and `epochs` are fixed (no adaptive/RNG) so the result is reproducible.
 * Features are standardized internally (stable GD); the transform is stored on the model.
 */
export function fitQuantileETA(
  pairs: { x: number[]; y: number }[],
  tau: number,
  opts: { lr?: number; epochs?: number } = {},
): ETAModel {
  if (pairs.length === 0) throw new Error('fitQuantileETA: empty training set');
  if (tau <= 0 || tau >= 1) throw new Error('fitQuantileETA: tau must be in (0,1)');
  const d = pairs[0].x.length;
  for (const p of pairs) {
    if (p.x.length !== d) throw new Error('fitQuantileETA: ragged features');
    if (!Number.isFinite(p.y)) throw new Error('fitQuantileETA: non-finite training target');
  }
  const { mu, sigma } = computeStats(pairs);
  const lr = opts.lr ?? 0.05;
  const epochs = opts.epochs ?? 400;
  const w = new Array(d).fill(0);
  let b = 0;
  for (let e = 0; e < epochs; e++) {
    let gw = new Array(d).fill(0);
    let gb = 0;
    for (const p of pairs) {
      const xs = standardizeRow(p.x, mu, sigma);
      const ei = p.y - (dot(w, xs) + b);
      const indicator = ei < 0 ? 1 : 0; // I(error < 0)
      const g = indicator - tau; // ∂quantileLoss/∂pred = -(tau) when error>0 (raise pred), +(1-tau) when error<0
      for (let i = 0; i < d; i++) gw[i] += g * xs[i];
      gb += g;
    }
    const n = pairs.length;
    for (let i = 0; i < d; i++) w[i] -= (lr * gw[i]) / n;
    b -= (lr * gb) / n;
  }
  return { w, b, mu, sigma, tau };
}

/** Point prediction of a fitted quantile model (re-standardizes features). */
export function predictETA(m: ETAModel, x: number[]): number {
  return dot(m.w, standardizeRow(x, m.mu, m.sigma)) + m.b;
}

/**
 * Fit the three-model interval forecaster (lo/point/hi) and return a predictor closure.
 * `point` uses τ=0.5 (median), `lo`/`hi` use the given tails.
 */
export function fitETAInterval(
  pairs: { x: number[]; y: number }[],
  opts: { loTau?: number; hiTau?: number; lr?: number; epochs?: number } = {},
): (x: number[]) => ETAInterval {
  const loTau = opts.loTau ?? 0.05;
  const hiTau = opts.hiTau ?? 0.95;
  const lo = fitQuantileETA(pairs, loTau, opts);
  const point = fitQuantileETA(pairs, 0.5, opts);
  const hi = fitQuantileETA(pairs, hiTau, opts);
  return (x: number[]) => {
    const loV = predictETA(lo, x);
    const pointV = predictETA(point, x);
    const hiV = predictETA(hi, x);
    return { point: pointV, lo: loV, hi: hiV, ordered: loV <= pointV && pointV <= hiV };
  };
}

/**
 * Huber-loss of an ETA model on a held-out set — the robust accuracy metric (outlier-insensitive).
 * Lower is better. Non-finite inputs are rejected (RED-TEAM poison guard).
 */
export function etaHuberError(m: ETAModel, pairs: { x: number[]; y: number }[]): number {
  if (pairs.length === 0) throw new Error('etaHuberError: empty');
  let s = 0;
  for (const p of pairs) {
    const e = p.y - predictETA(m, p.x);
    s += huber(e, 1);
  }
  return s / pairs.length;
}

/** Mean quantile loss across a set — the honest coverage metric for interval models. */
export function etaQuantileError(m: ETAModel, pairs: { x: number[]; y: number }[], tau = m.tau): number {
  if (pairs.length === 0) throw new Error('etaQuantileError: empty');
  let s = 0;
  for (const p of pairs) s += quantileLoss(p.y, predictETA(m, p.x), tau);
  return s / pairs.length;
}
