/**
 * loss.ts — Robust regression losses for the analytics layer.
 *
 * These answer the user's L5 prompt directly: plain MSE is fragile to
 * telemetry "spikes" (one bad sensor reading yanks the whole model). Huber
 * blends MSE (fast convergence, small error) with MAE (linear, bounded
 * gradient, large error) at a threshold δ. Quantile loss gives prediction
 * INTERVALS (e.g. Dowiz ETA confidence), and Focal loss re-weights rare
 * classes (e.g. incident vs nominal). All deterministic, no deps.
 *
 * Per the repo's Verified-by-Math bar every function has a RED+GREEN test.
 */

/**
 * Huber loss: ½·e²  for |e|≤δ ;  δ·(|e| − ½δ)  for |e|>δ.
 * Gradient is linear in e for large errors → robust to outliers.
 */
export function huber(error: number, delta = 1): number {
  if (!Number.isFinite(error)) throw new Error('huber: non-finite error');
  if (!Number.isFinite(delta) || delta <= 0) throw new Error('huber: delta must be finite > 0');
  const a = Math.abs(error);
  return a <= delta ? 0.5 * error * error : delta * (a - 0.5 * delta);
}

/** MSE = mean of squared errors (the baseline the prompt warns about). */
export function mse(errors: number[]): number {
  if (errors.length === 0) throw new Error('mse: empty');
  let s = 0;
  for (const e of errors) s += e * e;
  return s / errors.length;
}

/**
 * Quantile (pinball) loss for a single observation.
 *   τ>0.5 penalizes under-prediction harder (good for "ETA won't exceed" bounds);
 *   τ<0.5 penalizes over-prediction harder.
 */
export function quantileLoss(actual: number, pred: number, tau: number): number {
  if (!Number.isFinite(actual) || !Number.isFinite(pred)) throw new Error('quantileLoss: non-finite');
  if (tau <= 0 || tau >= 1) throw new Error('quantileLoss: tau must be in (0,1)');
  const e = actual - pred;
  return e >= 0 ? tau * e : (tau - 1) * e;
}

/**
 * Focal loss for a binary/softmax probability `p` of the TRUE class.
 * γ>0 down-weights easy (high-p) examples, focusing learning on hard ones.
 *   FL = −(1−p)^γ · log(p)
 */
export function focalLoss(pTrueClass: number, gamma = 2): number {
  if (!Number.isFinite(pTrueClass)) throw new Error('focalLoss: non-finite');
  if (pTrueClass < 0 || pTrueClass > 1) throw new Error('focalLoss: p must be in [0,1]');
  if (pTrueClass <= 0) return Infinity; // log(0) = −∞ ⇒ FL = +∞
  if (gamma < 0) throw new Error('focalLoss: gamma must be ≥ 0');
  const weight = (1 - pTrueClass) ** gamma;
  return -weight * Math.log(pTrueClass);
}
