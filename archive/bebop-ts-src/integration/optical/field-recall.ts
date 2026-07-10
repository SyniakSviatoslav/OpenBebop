// src/integration/optical/field-recall.ts
//
// WIRING: optical compute (SVETlANNa/Meep) as a field-search accelerator.
//
// The optical kernel computes G = FFT2D{ t(x,y) · E_in(x,y) } — a passive mask modulates a field
// (element-wise complex product), then free-space propagation performs the 2D Fourier transform
// (see optic.ts). A candidate's VSA projection is laid into a square field grid; the same mask +
// propagation applied to the query field yields a reference propagation. We rank candidates by
// correlation of their post-propagation field magnitudes with the query's post-propagation field.
//
// This is the SAME linear/spectral transform field.ts performs (a projection into the embedding
// plane), just computed on the optical primitive — a drop-in accelerator behind opticalMatmul.
//
// Off-chain: deterministic simulation (see optic NOTES.md). Use it to RANK candidates; the final
// accept still goes through the guard gate.

import { opticalMatmul, type OpticalMask } from './optic.ts';

/**
 * Rank candidate field-vectors by optical correlation with the query.
 * @param query  length n*n flat real vector (the field query, reshaped to n×n)
 * @param cands  m candidate vectors, each length n*n (same reshape)
 * @param mask   passive optical mask (n×n complex, |t|<=1)
 * @returns      indices of candidates sorted by DESCENDING correlation with the query's propagation
 */
export function opticalRecall(query: number[], cands: number[][], mask: OpticalMask): number[] {
  const n = mask.transmission.length;
  if (query.length !== n * n) throw new Error('opticalRecall: query must be n*n to match mask');
  for (const c of cands) if (c.length !== n * n) throw new Error('opticalRecall: candidate dim mismatch');

  const qGrid = toGrid(query, n);
  const qOut = opticalMatmul(qGrid, mask); // Complex[][], n×n propagated field
  const qMag = magnitudeField(qOut);
  const qPhase = phaseField(qOut);

  const scores = cands.map((c) => {
    const out = opticalMatmul(toGrid(c, n), mask);
    const mag = magnitudeField(out);
    const phase = phaseField(out);
    // Correlation = magnitude similarity AND phase alignment (the phase carries the field's
    // location; two identical fields share both → maximal score).
    return correlate(qMag, mag) * correlate(qPhase, phase);
  });

  return scores
    .map((s, i) => [i, s] as [number, number])
    .sort((a, b) => b[1] - a[1])
    .map(([i]) => i);
}

function toGrid(flat: number[], n: number): number[][] {
  const g: number[][] = Array.from({ length: n }, () => new Array<number>(n));
  for (let i = 0; i < n; i++) for (let j = 0; j < n; j++) g[i][j] = flat[i * n + j];
  return g;
}

function magnitudeField(c: [number, number][][]): number[][] {
  return c.map((row) => row.map(([re, im]) => Math.hypot(re, im)));
}

function phaseField(c: [number, number][][]): number[][] {
  return c.map((row) => row.map(([re, im]) => Math.atan2(im, re)));
}

/** Sum of element-wise products (cosine-like correlation on the magnitude fields). */
function correlate(a: number[][], b: number[][]): number {
  let s = 0;
  for (let i = 0; i < a.length; i++) for (let j = 0; j < a.length; j++) s += a[i][j] * b[i][j];
  return s;
}
