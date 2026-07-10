/**
 * telemetry-ica-loop.ts — ICA → cycle-consistency telemetry pipeline.
 *
 * The EV: Dowiz telemetry arrives as a MIXTURE of latent physical sources
 * (navigation drift + comms burst + battery sag + ...). Running a symmetrical
 * loop (cycle-consistency, F(G(x))≈x) on the RAW channel vector is weak: a
 * fault in subsystem #3 smears across every raw channel (because the raw mix
 * is A·s), so the fault locator points at raw channels, NOT at subsystem #3.
 *
 * ICA first unmixes the stream into independent sources S = W·Xwᵀ (each row =
 * one physical subsystem, clean). We then run cycle-consistency on the SEPARATED
 * state vector (the columns of S, one time-sample per row). Now a fault in
 * subsystem #3 lands purely in source #3 → the locator's `breakAt` == 3 →
 * exact fault localization. That is the measurable EV over the naive raw loop.
 *
 * IMPORTANT degeneracy avoided: a single raw CHANNEL is 1-D; cycle-consistency
 * with k=AUTO=d−1=0 returns error≡0 (identity round-trip) — useless. The ICA
 * step turns the problem into a d≥2 state, restoring a real symmetry gap.
 *
 * BLIND SPOT (inherited, proven): ICA cannot separate GAUSSIAN sources, so the
 * pipeline mis-localizes when the underlying sources are Gaussian. The RED test
 * feeds Gaussian sources and asserts the locator does NOT recover the true
 * subsystem — proving the bound, not hiding it.
 *
 * Deterministic, no RNG/Date/network. RED+GREEN falsifiable (Verified-by-Math).
 */

import { transpose, type Mat } from './matrix.ts';
import { fastICA, applyICA, type ICAModel, type ICAOptions } from './ica.ts';
import {
  fitConsistencyModel,
  asymmetryLocator,
  type CycleConsistencyResult,
  type PCA,
} from './cycle-consistency.ts';

/** Fitted ICA+cycle-consistency telemetry pipeline. */
export interface TelemetryICAPipeline {
  /** the unmixing (whitening + Wica) that separates raw telemetry. */
  ica: ICAModel;
  /** the cycle-consistency model fit on the SEPARATED source window. */
  cc: PCA;
  /** number of independent subsystems (dimensions of the separated state). */
  dims: number;
}

/**
 * Build the pipeline from a calibration batch of known-good telemetry.
 * `calibration`: m×d raw samples (rows = time, cols = mixed channels).
 * Fits ICA, then fits cycle-consistency on the separated multi-dim state.
 */
export function buildTelemetryICAPipeline(
  calibration: Mat,
  icaOpts: ICAOptions = {},
): TelemetryICAPipeline {
  const ica = fastICA(calibration, icaOpts); // ICAResult is assignable to ICAModel
  if (ica.nComponents < 2) {
    throw new Error('buildTelemetryICAPipeline: need ≥2 independent subsystems for a real symmetry gap');
  }
  // separated state: rows = time samples, cols = clean subsystems
  const separated: Mat = transpose(ica.S);
  const cc = fitConsistencyModel(separated);
  return { ica, cc, dims: ica.nComponents };
}

/**
 * Score ONE new raw telemetry sample through ICA → cycle-consistency.
 * Returns the verdict with `breakAt` = the INDEX OF THE BROKEN SUBSYSTEM
 * (after unmixing), not a raw channel. `error` = symmetry gap; `residual`
 * is per-subsystem.
 */
export function scoreTelemetrySample(
  pipeline: TelemetryICAPipeline,
  raw: number[],
): CycleConsistencyResult {
  const sep = applyICA(pipeline.ica, [raw]); // nComps × 1
  const vec = sep.map((r) => r[0]); // nComps state vector (clean subsystems)
  return asymmetryLocator(pipeline.cc, vec);
}

/** Convenience: which subsystem (0-indexed) broke, or −1 if none. */
export function localizeFault(pipeline: TelemetryICAPipeline, raw: number[]): number {
  return scoreTelemetrySample(pipeline, raw).breakAt;
}
