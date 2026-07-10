/**
 * telemetry-shadow.ts — Dowiz telemetry SHADOW harness (operator directive 2026-07-08/09).
 *
 * A shadow consumer of live/staging Dowiz telemetry: it ingests raw telemetry rows, runs them
 * through the calibrated ICA+cycle-consistency pipeline, and reports (a) the localized SUBSYSTEM
 * fault and (b) slow STRUCTURAL DRIFT (distribution shift of the telemetry vs the calibration
 * baseline). It NEVER acts on the governor — it only reports, so it can run live beside production
 * and be promoted later. The live connector (pulling rows from Dowiz's telemetry store) is
 * operator-wired in apps/api; this module is the math, proven by the RED+GREEN tests below.
 *
 * Verified-by-Math: every exported function has a falsifiable RED+GREEN test (telemetry-shadow.test.ts).
 */

import { buildTelemetryICAPipeline, scoreTelemetrySample, type TelemetryICAPipeline } from './telemetry-ica-loop.ts';

export interface ShadowReport {
  /** localized subsystem fault index from the ICA pipeline, or -1 when below the fault gate. */
  subsystemFault: number;
  /** symmetry-gap reconstruction error of the latest row. */
  reconError: number;
  /** structural drift detected (telemetry distribution shifted vs calibration baseline). */
  drift: boolean;
  /** 0..1 severity of the drift (0 = on baseline, →1 = far). */
  driftSeverity: number;
}

export interface TelemetryShadow {
  pipeline: TelemetryICAPipeline;
  /** ingest one telemetry row (already a numeric feature vector), return a report. */
  ingest(row: number[]): ShadowReport;
}

/**
 * Build a shadow harness from a calibration batch of known-good telemetry rows.
 * `driftK` = how many calibration-STD the running mean may drift before `drift` flips (default 3).
 */
export function buildTelemetryShadow(calib: number[][], opts: { driftK?: number; faultError?: number } = {}): TelemetryShadow {
  if (calib.length === 0) throw new Error('buildTelemetryShadow: empty calibration');
  const d = calib[0].length;
  for (const r of calib) if (r.length !== d) throw new Error('buildTelemetryShadow: ragged calibration');
  const pipeline = buildTelemetryICAPipeline(calib);
  const driftK = opts.driftK ?? 3;
  const faultError = opts.faultError ?? 1.0;
  // calibration mean + std per feature (drift baseline + tolerance)
  const mean = new Array(d).fill(0);
  for (const r of calib) for (let i = 0; i < d; i++) mean[i] += r[i] ?? 0;
  for (let i = 0; i < d; i++) mean[i] /= calib.length;
  const std = new Array(d).fill(0);
  for (const r of calib) for (let i = 0; i < d; i++) { const di = (r[i] ?? 0) - mean[i]; std[i] += di * di; }
  for (let i = 0; i < d; i++) std[i] = Math.sqrt(std[i] / calib.length) || 1e-6;
  // running EMA of each feature (starts at baseline, adapts slowly)
  const ema = mean.slice();
  const alpha = 0.05;

  return {
    pipeline,
    ingest(row: number[]): ShadowReport {
      if (row.length !== d) throw new Error('buildTelemetryShadow: row width mismatch');
      const r = scoreTelemetrySample(pipeline, row);
      for (let i = 0; i < d; i++) ema[i] += alpha * ((row[i] ?? 0) - ema[i]);
      // drift = any feature's EMA moved > driftK·std from the calibration mean
      let maxZ = 0;
      for (let i = 0; i < d; i++) {
        const z = Math.abs(ema[i] - mean[i]) / std[i];
        if (z > maxZ) maxZ = z;
      }
      const drift = maxZ > driftK;
      const severity = Math.min(1, Math.max(0, (maxZ - driftK) / driftK));
      return { subsystemFault: r.breakAt >= 0 && r.error > faultError ? r.breakAt : -1, reconError: r.error, drift, driftSeverity: severity };
    },
  };
}
