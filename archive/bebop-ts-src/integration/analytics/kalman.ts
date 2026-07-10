/**
 * N8a (2026-07-09): Deterministic 1-D Kalman filter for telemetry smoothing + anomaly.
 *
 * The dump's "degradation early-warning" and "next-state prediction" asks both collapse to
 * ONE honest primitive: a Kalman filter. It is fully DETERMINISTIC (no RNG, no training, no Date)
 * — the process/measurement noise are fixed matrices supplied by the caller. This is the textbook
 * sensor-fusion tool, reused here to:
 *   1. smooth the governor's per-step hallucinationRate into a variance-aware trend (N7++), and
 *   2. expose the *innovation* (measurement residual y − H·x̂) as a principled anomaly signal
 *      (large innovation = the world diverged from the model's prediction — the dump's
 *      "tell me it's degrading before it fails").
 *
 * No stochastic extension, no learning loop. A Kalman filter is a recursive Least-Squares update;
 * it is math, not ML. Falsifiable RED+GREEN tests in kalman.test.ts.
 */

export interface Kalman1DState {
  x: number; // posterior estimate
  P: number; // posterior estimate covariance
}

export interface Kalman1DConfig {
  q: number; // process noise variance (how much the true value is allowed to drift per step)
  r: number; // measurement noise variance (how noisy the observation is)
}

/** Prior for a never-seen signal. x=0, P=large (maximal uncertainty). */
export function kalman1dInit(): Kalman1DState {
  return { x: 0, P: 1e6 };
}

/**
 * One Kalman predict+update step for a scalar signal with identity observation (H=1).
 * - predict: x⁻ = x, P⁻ = P + q
 * - update:  K = P⁻ / (P⁻ + r); x = x⁻ + K·(z − x⁻); P = (1 − K)·P⁻
 * Returns the new state AND the innovation (residual) `z − x⁻`, which is the anomaly signal.
 */
export function kalman1dStep(
  st: Kalman1DState,
  z: number,
  cfg: Kalman1DConfig,
): { state: Kalman1DState; innovation: number; gain: number } {
  // predict
  const Pmin = st.P + cfg.q;
  // update
  const K = Pmin / (Pmin + cfg.r);
  const innovation = z - st.x; // measurement residual before correction
  const x = st.x + K * innovation;
  const P = (1 - K) * Pmin;
  return { state: { x, P }, innovation, gain: K };
}

/**
 * Kalman-based anomaly: a measurement is anomalous when its innovation exceeds `k` standard
 * deviations of the CURRENT measurement-noise belief (√r). Deterministic, no k-sigma history needed.
 * GREEN: smooth measurement → innovation ≈ 0 → not anomalous.
 * RED: a sudden jump → innovation spikes → anomalous (the world diverged from the model).
 */
export function kalmanAnomaly(
  st: Kalman1DState,
  z: number,
  cfg: Kalman1DConfig,
  k = 3,
): { state: Kalman1DState; anomalous: boolean; innovation: number } {
  const { state, innovation } = kalman1dStep(st, z, cfg);
  const sigma = Math.sqrt(cfg.r);
  const anomalous = sigma > 0 && Math.abs(innovation) > k * sigma;
  return { state, anomalous, innovation };
}
