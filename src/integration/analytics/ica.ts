/**
 * ica.ts — FastICA (fixed-point, deflation) for blind source separation of
 * mixed-signal logs. Deterministic: NO RNG. The unmixing is initialized from
 * the CANONICAL basis (e_k), not a random vector, so the result is bit-stable
 * across runs — required by the sovereign-core "no runtime randomness" rule.
 *
 * Use case (from the brief): Dowiz telemetry/logs are a MIXTURE of latent
 * signal sources (e.g. navigation noise + comms burst + battery sag). ICA
 * recovers the independent sources so each can be monitored / cycle-consistency
 * checked on its OWN axis instead of the entangled raw stream.
 *
 * MATH (Hyvärinen & Oja 2000, fixed-point):
 *   Given centered data Xc (m×n), whiten: Xw = Xc·W,  W = V·diag(1/√s)·Vᵀ
 *   (SVD of cov C = XcᵀXc/(m−1)). Then iterate per component k:
 *       w ← E{Xw · g(wᵀXw)} − E{g'(wᵀXw)} · w      (g = x³, g' = 3x²)
 *       w ← w / ‖w‖
 *       w ← w − Σ_{j<k} (wᵀw_j) w_j               (deflation orthogonalization)
 *   Sources S = W·Xwᵀ (each row an independent component). Converges because
 *   the fixed point of that iteration is a local max of non-Gaussianity (the
 *   contrast E{G(wᵀXw)}), which is what makes the separated sources independent.
 *
 * BLIND SPOT (proven, not hidden — mirrors cycle-consistency): ICA cannot
 * separate GAUSSIAN sources. Two mixed Gaussians are rotationally symmetric, so
 * no contrast function distinguishes them — the unmixing is undefined up to an
 * orthogonal rotation. The RED test feeds two Gaussians and asserts the
 * recovered sources FAIL to match the originals (|corr| not ≈ 1), while two
 * non-Gaussian sources (one sub-Gaussian uniform + one super-Gaussian Laplace)
 * DO recover (|corr| ≈ 1 up to permutation/sign). See ica.test.ts.
 *
 * Deterministic, no RNG/Date/network. Falsifiable RED+GREEN (Verified-by-Math).
 */

import { svd, transpose, type Mat, type SVD } from './matrix.ts';

/** small self-contained linear helpers (matrix.ts exports svd/transpose only) */
function matMul(A: Mat, B: Mat): Mat {
  const n = B[0].length;
  return A.map((row) => Array.from({ length: n }, (_, j) => row.reduce((s, _, i) => s + row[i] * B[i][j], 0)));
}
function dot(a: number[], b: number[]): number {
  let s = 0;
  for (let i = 0; i < a.length; i++) s += a[i] * b[i];
  return s;
}
function colMeans(X: Mat): number[] {
  const n = X[0].length;
  const m = X.length;
  const out = new Array(n).fill(0);
  for (const row of X) for (let j = 0; j < n; j++) out[j] += row[j];
  return out.map((v) => v / m);
}

export type Nonlinearity = 'pow3' | 'tanh' | 'gauss';
/** g(wᵀx) and its derivative g'(wᵀx), for the chosen contrast function. */
function gfun(u: number[], kind: Nonlinearity): { g: number[]; gp: number[] } {
  if (kind === 'tanh') {
    const g = u.map((x) => Math.tanh(x));
    const gp = u.map((x) => 1 - Math.tanh(x) ** 2);
    return { g, gp };
  }
  if (kind === 'gauss') {
    const g = u.map((x) => x * Math.exp(-(x * x) / 2));
    const gp = u.map((x) => (1 - x * x) * Math.exp(-(x * x) / 2));
    return { g, gp };
  }
  // default 'pow3' (x^3), g' = 3x^2
  const g = u.map((x) => x ** 3);
  const gp = u.map((x) => 3 * x * x);
  return { g, gp };
}

export interface ICAOptions {
  /** number of components to extract (default: all = min(m,n) cols). */
  nComponents?: number;
  /** contrast nonlinearity (default 'pow3'). */
  nonlinearity?: Nonlinearity;
  /** fixed-point iterations (default 200). */
  maxIter?: number;
  /** convergence tolerance on ‖w − w_prev‖ (default 1e-6). */
  tol?: number;
  /** deterministic init vectors (length nComponents, each length n). Any valid
   *  orthonormal-ish start works; default = canonical basis e_k. Exposed so tests
   *  can prove the GAUSSIAN blind spot (non-uniqueness ⇒ different inits ⇒
   *  different valid unmixings). */
  init?: Mat;
}

export interface ICAModel {
  /** whitening matrix W (n×n). */
  K: Mat;
  /** unmixing matrix Wica (nComponents×n). */
  W: Mat;
  /** per-column mean subtracted from input. */
  mean: number[];
  nComponents: number;
  /** the contrast nonlinearity used (for replay via applyICA). */
  nonlinearity: Nonlinearity;
}

export interface ICAResult extends ICAModel {
  /** estimated sources: nComponents rows × m cols (each row = one source). */
  S: Mat;
  /** convergence flag per component. */
  converged: boolean[];
}

/**
 * Fit FastICA via deflation (canonical init ⇒ deterministic).
 * X: m×n (m samples, n mixed signals).
 */
export function fastICA(X: Mat, opts: ICAOptions = {}): ICAResult {
  const m = X.length;
  const n = X[0].length;
  const nComp = Math.min(opts.nComponents ?? n, n);
  const nl = opts.nonlinearity ?? 'pow3';
  const maxIter = opts.maxIter ?? 200;
  const tol = opts.tol ?? 1e-6;

  // center
  const mean = colMeans(X);
  const Xc = X.map((row) => row.map((v, j) => v - mean[j]));

  // whiten: SVD of covariance
  const cov = matMul(transpose(Xc), Xc);
  for (let i = 0; i < n; i++) for (let j = 0; j < n; j++) cov[i][j] /= Math.max(1, m - 1);
  const { V, S: sing }: SVD = svd(cov);
  const d = sing.map((s) => (s < 1e-12 ? 0 : 1 / Math.sqrt(s)));
  // K = V · diag(d)
  const K: Mat = V.map((row) => row.map((v, j) => v * d[j]));
  // Xw = Xc · K  (m×n)
  const Xw = matMul(Xc, K);

  const W: Mat = [];
  const converged: boolean[] = [];
  const Wrows = W as number[][]; // accumulating unmixing rows
  for (let k = 0; k < nComp; k++) {
    // init: explicit deterministic vector if provided, else canonical e_k (no RNG)
    let w: number[];
    if (opts.init && opts.init[k]) {
      w = opts.init[k].slice();
    } else {
      w = new Array(n).fill(0);
      w[k] = 1;
    }
    let ok = false;
    for (let it = 0; it < maxIter; it++) {
      // u = wᵀ Xw  (length m)
      const u = Xw.map((row) => dot(w, row));
      const { g, gp } = gfun(u, nl);
      // E{Xw · g(u)}  =  (1/m) Σ_i Xw[i] * g(u[i])   → vector length n
      const eg = new Array(n).fill(0);
      for (let i = 0; i < m; i++) for (let j = 0; j < n; j++) eg[j] += Xw[i][j] * g[i];
      for (let j = 0; j < n; j++) eg[j] /= m;
      const egp = gp.reduce((s, v) => s + v, 0) / m; // E{g'(u)}
      // w_new = eg − egp·w
      const wNew = w.map((v, j) => eg[j] - egp * v);
      // deflation orthogonalization against previous rows
      for (const wj of Wrows) {
        const c = dot(wNew, wj);
        for (let j = 0; j < n; j++) wNew[j] -= c * wj[j];
      }
      // normalize
      const norm = Math.sqrt(wNew.reduce((s, v) => s + v * v, 0)) || 1;
      for (let j = 0; j < n; j++) wNew[j] /= norm;
      // sign-ambiguity: fixed-point may converge to −w, flipping each iter.
      // Align sign to previous w so the delta reflects true convergence.
      if (dot(wNew, w) < 0) for (let j = 0; j < n; j++) wNew[j] = -wNew[j];
      const delta = Math.sqrt(wNew.reduce((s, v, j) => s + (v - w[j]) ** 2, 0));
      w = wNew;
      if (delta < tol) { ok = true; break; }
    }
    Wrows.push(w);
    converged.push(ok);
  }

  // sources: S = W · Xwᵀ   → nComp × m
  const XwT = transpose(Xw);
  const S = matMul(W, XwT);

  return { K, W, mean, nComponents: nComp, nonlinearity: nl, S, converged };
}

/** Apply a fitted ICA model to NEW data (same dimensionality). */
export function applyICA(model: ICAModel, X: Mat): Mat {
  const Xc = X.map((row) => row.map((v, j) => v - model.mean[j]));
  const Xw = matMul(Xc, model.K);
  return matMul(model.W, transpose(Xw));
}

/**
 * Max abs correlation between recovered sources and true sources, allowing for
 * PERMUTATION and SIGN flip (ICA is only identifiable up to those). Returns the
 * best-matching score per true source (1 = perfect recovery).
 */
export function recoveryScore(Sest: Mat, Strue: Mat): number[] {
  const k = Math.min(Sest.length, Strue.length);
  const score: number[] = [];
  for (let i = 0; i < k; i++) {
    let best = 0;
    for (let j = 0; j < k; j++) {
      const a = Sest[i], b = Strue[j];
      const ma = a.reduce((s, v) => s + v, 0) / a.length;
      const mb = b.reduce((s, v) => s + v, 0) / b.length;
      let num = 0, da = 0, db = 0;
      for (let t = 0; t < a.length; t++) { num += (a[t] - ma) * (b[t] - mb); da += (a[t] - ma) ** 2; db += (b[t] - mb) ** 2; }
      const c = Math.abs(num / Math.sqrt(da * db + 1e-18));
      if (c > best) best = c;
    }
    score.push(best);
  }
  return score;
}
