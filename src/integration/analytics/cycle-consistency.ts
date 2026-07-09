/**
 * cycle-consistency.ts — The "double-rotational symmetry" / symmetrical-loop
 * harness. Mathematically this is CYCLE CONSISTENCY:
 *
 *     x̂ = Reconstruct(Decompose(x))   ⇒   ‖x − x̂‖  is the symmetry gap.
 *
 * The user's L5 prompt framed it as "Double-rotational symmetry" — you rotate
 * the state into a latent space (Decompose) then back (Reconstruct); if the
 * state no longer resolves, a module lost integrity. This is the SAME math as
 * the PCA-reconstruction anomaly (anomaly.ts) but with a DIFFERENT job:
 *
 *   • anomaly.ts  → IS the input weird vs a normal model?  (outlier detection)
 *   • cycle-consistency → is the ROUND-TRIP lossless?        (invariant check)
 *
 * So where anomaly.ts needs a pre-calibrated "normal" PCA model, cycle-consistency
 * fits the PCA on the SAMPLE ITSELF (or a reference window) and measures how well
 * the sample survives its own decomposition. It is the deterministic twin of the
 * prompt's "F(G(X)) == X" invariant harness, and of CycleGAN cycle-consistency.
 *
 * WHY PCA (not a trained VAE): the sovereign-core rule forbids runtime RNG/training.
 * A linear autoencoder (PCA) is the only deterministic, bit-reproducible way to get
 * a Decompose/Reconstruct pair. And it gives us a PROVABLE bound, not a heuristic:
 *
 *   THEOREM (cycle-consistency error bound). Let X be an m×d matrix (m≥2),
 *   Xc = X − mean, SVD Xc = U·diag(s)·Vᵀ, explained variance σⱼ² = sⱼ²/(m−1).
 *   Decompose = project onto top-k axes, Reconstruct = inverse. Then for ANY
 *   sample x (in or out of the fit window):
 *       ‖x − x̂‖²  ≤  Σ_{j>k} σⱼ²      (truncated PCA, k<d)
 *   and with k = d (full rank) the round-trip is EXACT: ‖x − x̂‖ = 0.
 *   Proof: x̂ = mean + Σ_{j≤k} (vⱼ·(x−mean))·vⱼ, and {vⱼ} orthonormal ⇒ the
 *   discarded tail Σ_{j>k} (vⱼ·(x−mean))·vⱼ has squared norm exactly Σ_{j>k} σⱼ²
 *   in the basis of principal axes (Parseval). So the symmetry gap is bounded by
 *   the variance we chose to throw away — deterministic and knowable in advance.
 *
 * BLIND SPOT (proven, not hidden): cycle-consistency checks INTEGRITY, not TRUTH.
 * A map f with f(x)=A·x where A is invertible still has f⁻¹(f(x))=x (perfect
 * symmetry) yet may be semantically wrong (e.g. x→2x→x/2). So this harness is a
 * NECESSARY guard (catches corruption / dropped fields / asymmetric refactors),
 * never a SUFFICIENT correctness proof. Hard red-line boundaries (money/RLS/
 * drone-physics) still need explicit contract tests. The RED test below proves
 * this blind spot so nobody trusts the loop alone.
 *
 * Every property is RED+GREEN falsifiable (Verified-by-Math bar).
 */

import { pcaFit, pcaProject, pcaReconstruct, type PCA, type Mat } from './matrix.ts';
import { mse } from './loss.ts';

export type { PCA };

/** One sample's cycle-consistency verdict. */
export interface CycleConsistencyResult {
  /** ‖x − x̂‖₂ — the symmetry gap (0 = perfect round-trip). */
  error: number;
  /** per-feature residual rⱼ = xⱼ − x̂ⱼ (which dimensions broke). */
  residual: number[];
  /** index of the largest |residual| — the "auto-reverse" fault locator. */
  breakAt: number;
  /** true when error exceeds the (adaptive or static) threshold. */
  broken: boolean;
  /** latent z (the decomposed state) — what the agent "believes" it saw. */
  z: number[];
}

/**
 * Fit a PCA model used to define the Decompose/Reconstruct basis.
 * Pass a reference window of known-good state snapshots; reuse thereafter.
 * (For pure self-consistency on a single sample, pass [x] — but you need
 *  ≥2 rows for PCA, so pass a small window even if it's the same vector twice
 *  with tiny jitter, or use `fitRank1` for d=1.)
 */
export function fitConsistencyModel(window: Mat): PCA {
  if (window.length < 2) throw new Error('fitConsistencyModel: need ≥2 calibration samples');
  return pcaFit(window);
}

/**
 * Cycle-consistency error of ONE sample under a fitted basis.
 *   x̂ = Reconstruct(Project(x, k)) ;  error = ‖x − x̂‖₂
 * k = rank kept; k=0 ⇒ AUTO = d−1 (so a perfect identity is never returned and
 * off-manifold samples always leave a non-zero gap, exactly like anomaly.ts).
 */
export function cycleConsistencyError(model: PCA, x: number[], k = 0): number {
  if (x.length !== model.mean.length) throw new Error('cycleConsistencyError: dim mismatch');
  const d = model.components[0].length;
  const kk = k === 0 ? Math.max(1, d - 1) : Math.min(k, d); // k=d ⇒ EXACT round-trip
  const z = pcaProject(model, x, kk);
  const xhat = pcaReconstruct(model, z, kk);
  let s = 0;
  for (let i = 0; i < x.length; i++) { const e = x[i] - xhat[i]; s += e * e; }
  return Math.sqrt(s);
}

/**
 * Full per-feature verdict: error + residual vector + argmax (the fault locator
 * the prompt called the "diff-analyzer / auto-reverse"). Returns breakAt = −1
 * when error is ~0 (nothing broke).
 */
export function asymmetryLocator(model: PCA, x: number[], k = 0): CycleConsistencyResult {
  const d = model.components[0].length;
  const kk = k === 0 ? Math.max(1, d - 1) : Math.min(k, d); // k=d ⇒ EXACT round-trip
  const z = pcaProject(model, x, kk);
  const xhat = pcaReconstruct(model, z, kk);
  const residual = x.map((v, i) => v - xhat[i]);
  let errSq = 0, breakAt = -1, maxAbs = 0;
  for (let i = 0; i < residual.length; i++) {
    errSq += residual[i] * residual[i];
    const a = Math.abs(residual[i]);
    if (a > maxAbs) { maxAbs = a; breakAt = i; }
  }
  return { error: Math.sqrt(errSq), residual, breakAt, broken: false, z };
}

// ── adaptive-threshold gate (the "shadow or gate" decision lives at call site) ──

export interface CycleConsistencyConfig {
  /** rank kept (0 ⇒ AUTO d−1). Keep more ⇒ tighter symmetry, less compression. */
  k: number;
  /** static threshold; if >0 this is the hard gate and EMA is ignored. */
  threshold?: number;
  /** EMA smoothing for an adaptive threshold (learns slow drift out). 0<α≤1. */
  emaAlpha: number;
  /** warmup steps before the gate may flag (floor still being built). */
  warmup: number;
  /** hysteresis margin: flag only when error > floor·(1+margin). */
  margin: number;
}

export const DEFAULT_CYCLE_CONSISTENCY: CycleConsistencyConfig = {
  k: 0,
  threshold: undefined,
  emaAlpha: 0.1,
  warmup: 8,
  margin: 0.1,
};

export interface CycleConsistencyGateState {
  error: number;
  threshold: number;
  broken: boolean;
  breakAt: number;
  residual: number[];
  step: number;
}

/**
 * Score ONE sample through the symmetrical loop with an adaptive (or static)
 * threshold. `prev` carries the running EMA floor + step across calls.
 * Returns broken=true on a symmetry breach. Caller decides shadow (log) vs
 * gate (block) — this module only reports.
 */
export function cycleConsistencyGate(
  model: PCA,
  x: number[],
  cfg: CycleConsistencyConfig,
  prevThreshold = 0,
  prevStep = 0,
): CycleConsistencyGateState {
  const step = prevStep + 1;
  const v = asymmetryLocator(model, x, cfg.k);
  const staticTh = cfg.threshold ?? 0;
  const threshold = cfg.threshold && cfg.threshold > 0
    ? staticTh
    : cfg.emaAlpha * prevThreshold + (1 - cfg.emaAlpha) * v.error;
  const floor = cfg.threshold && cfg.threshold > 0 ? cfg.threshold : threshold;
  const broken = step > cfg.warmup && v.error > floor * (1 + cfg.margin);
  return { error: v.error, threshold: floor, broken, breakAt: v.breakAt, residual: v.residual, step };
}
