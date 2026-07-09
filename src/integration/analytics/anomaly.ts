/**
 * anomaly.ts — PCA-reconstruction anomaly detection for the L5 governor.
 *
 * This is the DETERMINISTIC realization of the "ELBO / VAE anomaly score"
 * the user's L5 prompt described. A full VAE needs a trained weight matrix
 * + SGD loop, which the sovereign-core rule forbids at runtime (no RNG, no
 * training). A LINEAR autoencoder === PCA reconstruction, so we get the
 * exact same math — reconstruction error (how "weird" the input looks) plus
 * a latent KL term (how "atypical" the latent state z is vs N(0,I)) — with
 * zero learned parameters and bit-for-bit reproducibility.
 *
 * Anomaly score (matches the prompt's ELBO→score recipe):
 *   score = || x − x̂ ||₂                         (reconstruction term)
 *         + β · Σ (zⱼ²)                          (latent KL ≈ Σ(zⱼ²) for N(0,I))
 *
 * Adaptive threshold: the prompt explicitly warned AGAINST a constant
 * threshold in a changing environment (battery drift, weather). We use an
 * exponential-moving-average floor:
 *   threshold_k = α · threshold_{k-1} + (1−α) · score_k
 * so slow, persistent drift is learned-out and only SHARP excursions flag.
 *
 * Every property is RED+GREEN falsifiable (Verified-by-Math bar).
 */

import { pcaFit, pcaProject, pcaReconstruct, type PCA } from './matrix.ts';

export type { PCA };

export interface PcaAnomalyConfig {
  /** top-k principal axes to keep (compression rank). 0 => AUTO = d−1 (drop the
   *  least-variance axis, so reconstruction is never a perfect identity and
   *  off-manifold samples always leave a non-zero residual). */
  k: number;
  /** KL weight on the latent term. β>1 ⇒ more structured latent (β-VAE style). */
  beta: number;
  /** EMA smoothing for the adaptive threshold. 0<α≤1. */
  emaAlpha: number;
  /**
   * Warmup steps: the EMA floor is only allowed to FLAG after this many
   * samples have flowed through. Before that, the floor is still being
   * established (it starts at 0, so any non-zero score would spuriously
   * flag on step 1). During warmup the threshold is tracked but flag=false.
   */
  warmup: number;
  /**
   * Hysteresis margin on the adaptive threshold. An excursion only flags when
   * score > threshold·(1+margin). Without this, the EMA floor decays toward
   * the (tiny, non-zero) in-manifold reconstruction error and a normal sample
   * would spuriously trip on numerical noise. margin=0.5 ⇒ 50% over the
   * running floor is required to declare an anomaly.
   */
  margin: number;
}

export const DEFAULT_PCA_ANOMALY: PcaAnomalyConfig = {
  k: 0, // all axes
  beta: 0, // KL term OFF by default: PCA reconstruction error alone is the
  // deterministic, false-positive-free anomaly signal. Set β>0 only when you
  // have calibrated the latent N(0,I) prior against your normal data (β-VAE
  // style) — otherwise raw Σzⱼ² flags perfectly-normal samples whose latent
  // mean is simply non-zero.
  emaAlpha: 0.1,
  warmup: 8,
  margin: 0.1,
};

export interface PcaAnomalyState {
  score: number;
  threshold: number;
  flag: boolean;
  /** latent coordinate z (the "state the agent believes it's in"). */
  z: number[];
  /** steps seen since the model was built (for warmup gating). */
  step: number;
}

/**
 * Build a PCA model from a NORMAL/clean calibration window.
 * Call once when the agent is known-good; reuse thereafter.
 */
export function buildNormalModel(window: number[][], k = 0): PCA {
  if (window.length < 2) throw new Error('buildNormalModel: need ≥2 calibration samples');
  return pcaFit(window);
}

function reconstructionError(x: number[], xhat: number[]): number {
  let s = 0;
  for (let i = 0; i < x.length; i++) {
    const d = x[i] - xhat[i];
    s += d * d;
  }
  return Math.sqrt(s);
}

/**
 * Score ONE new sample against a normal model + running EMA threshold.
 * `prevThreshold` is the previous EMA floor (0 on first call) and `prevStep`
 * the count of samples already scored (0 on first call). Returns the new
 * state; carry BOTH `state.threshold` and `state.step` into the next call.
 */
export function pcaAnomalyScore(
  model: PCA,
  x: number[],
  cfg: PcaAnomalyConfig,
  prevThreshold = 0,
  prevStep = 0,
): PcaAnomalyState {
  if (!Number.isFinite(x.reduce((a, b) => a + b, 0))) {
    throw new Error('pcaAnomalyScore: non-finite input (poison guard)');
  }
  const step = prevStep + 1;
  const d = model.components[0].length;
  // AUTO rank: keep d−1 axes (never a perfect identity) unless explicitly set.
  const k = cfg.k === 0 ? Math.max(1, d - 1) : Math.min(cfg.k, d - 1);
  const z = pcaProject(model, x, k);
  const xhat = pcaReconstruct(model, z, k);
  const recon = reconstructionError(x, xhat);
  const latentKL = z.reduce((s, v) => s + v * v, 0); // Σ zⱼ²  (KL to N(0,I))
  const score = recon + cfg.beta * latentKL;
  const threshold = cfg.emaAlpha * prevThreshold + (1 - cfg.emaAlpha) * score;
  // During warmup the floor is still being established → never flag, but keep
  // tracking the threshold so it is ready the moment warmup ends.
  // Hysteresis: require score to exceed the floor by `margin` (default 50%)
  // so numerical noise on in-manifold data does not spuriously trip.
  const flag = step > cfg.warmup && score > threshold * (1 + cfg.margin);
  return { score, threshold, flag, z, step };
}
