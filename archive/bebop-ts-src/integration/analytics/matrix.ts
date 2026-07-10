/**
 * matrix.ts — Deterministic linear-algebra primitives for the analytics layer.
 *
 * Reverse-engineered from the LINPACK/LAPACK-class decomposition maths
 * (Jacobi eigenvalue algorithm + two-sided SVD via the symmetric EVD of
 * AᵀA / AAᵀ). These are the foundation the user's L5 prompt asked for
 * (SVD / PCA / ICA) and they fit the sovereign-node rule: pure Float64
 * math, NO RNG, NO Date, NO network — verifiable bit-for-bit.
 *
 * Why these primitives (max-EV seams in our stack):
 *   1. PCA reconstruction  → linear, deterministic stand-in for the VAE
 *      "anomaly score" the prompt described (no training loop → no
 *      deterministic-core violation). Used by governor anomaly upgrade.
 *   2. SVD of an adjacency matrix → "architecture mining" (latent module
 *      clusters / coupling) — the reverse-engineering harness seam.
 *   3. PCA projection of VSA/embedding spaces → RAG-noise cleaning.
 *
 * Honest limit: this is a compact, numerically-stable-for-small-matrices
 * implementation (Jacobi, O(n³) with a fixed sweep cap). It is NOT a
 * BLAS-grade library; it targets the small matrices our telemetry /
 * architecture graphs actually produce (n ≤ a few hundred). For larger
 * inputs, swap in a WASM BLAS twin without changing the signatures.
 */

export type Mat = number[][]; // row-major: m[i][j]
export type Vec = number[];

// ── small helpers ───────────────────────────────────────────────────────────

/** Transpose. */
export function transpose(a: Mat): Mat {
  const n = a.length;
  const m = a[0].length;
  const t: Mat = Array.from({ length: m }, () => new Array(n).fill(0));
  for (let i = 0; i < n; i++) for (let j = 0; j < m; j++) t[j][i] = a[i][j];
  return t;
}

/** Matrix × matrix. */
export function matmul(a: Mat, b: Mat): Mat {
  const n = a.length, k = b.length, m = b[0].length;
  const out: Mat = Array.from({ length: n }, () => new Array(m).fill(0));
  for (let i = 0; i < n; i++)
    for (let p = 0; p < k; p++) {
      const aip = a[i][p];
      if (aip === 0) continue;
      const brow = b[p];
      const orow = out[i];
      for (let j = 0; j < m; j++) orow[j] += aip * brow[j];
    }
  return out;
}

/** Multiply each column j of a by s[j]. */
function scaleCols(a: Mat, s: Vec): Mat {
  return a.map((row) => row.map((v, j) => v * s[j]));
}

function matFinite(a: Mat): boolean {
  for (const row of a) for (const v of row) if (!Number.isFinite(v)) return false;
  return true;
}

// ── Jacobi eigenvalue decomposition of a SYMMETRIC matrix ──────────────────────
// A = V · diag(λ) · Vᵀ, V orthogonal. One-sided Jacobi rotation sweeps.

export interface EVD {
  values: Vec; // eigenvalues λ (unsorted)
  vectors: Mat; // V, column j = eigenvector for values[j]
}

export function jacobiEVD(A: Mat, sweeps = 32): EVD {
  const n = A.length;
  if (n === 0) throw new Error('jacobiEVD: empty matrix');
  for (const row of A) if (row.length !== n) throw new Error('jacobiEVD: non-square input');
  if (!matFinite(A)) throw new Error('jacobiEVD: non-finite input');

  // work on a copy; build eigenvector matrix V = I
  const a = A.map((r) => r.slice());
  const V: Mat = Array.from({ length: n }, (_, i) => {
    const r = new Array(n).fill(0);
    r[i] = 1;
    return r;
  });

  for (let sweep = 0; sweep < sweeps; sweep++) {
    let off = 0;
    for (let p = 0; p < n; p++)
      for (let q = p + 1; q < n; q++) off += a[p][q] * a[p][q];
    if (off < 1e-18) break; // converged (off-diagonal Frobenius norm ~ 0)

    for (let p = 0; p < n; p++) {
      for (let q = p + 1; q < n; q++) {
        const apq = a[p][q];
        if (Math.abs(apq) < 1e-300) continue;
        const app = a[p][p];
        const aqq = a[q][q];
        // Jacobi rotation angle: tan(2θ) = 2·apq / (aqq − app)
        const phi = (aqq - app) / (2 * apq);
        const t = Math.sign(phi || 1) / (Math.abs(phi) + Math.sqrt(phi * phi + 1));
        const c = 1 / Math.sqrt(t * t + 1);
        const s = t * c;
        // apply rotation to A (symmetric) and to V
        for (let i = 0; i < n; i++) {
          const aip = a[i][p], aiq = a[i][q];
          a[i][p] = c * aip - s * aiq;
          a[i][q] = s * aip + c * aiq;
        }
        for (let i = 0; i < n; i++) {
          const aip = a[p][i], aiq = a[q][i];
          a[p][i] = c * aip - s * aiq;
          a[q][i] = s * aip + c * aiq;
        }
        for (let i = 0; i < n; i++) {
          const vip = V[i][p], viq = V[i][q];
          V[i][p] = c * vip - s * viq;
          V[i][q] = s * vip + c * viq;
        }
      }
    }
  }

  const values = new Array(n);
  for (let i = 0; i < n; i++) values[i] = a[i][i];
  return { values, vectors: V };
}

// ── Symmetric, positive-definite square-root inverse (for whitening) ──────────

/** Diag(λ)^(-1/2); eigenvalues ≤ eps are treated as 0 (no division by ~0). */
export function invSqrtDiag(values: Vec, eps = 1e-12): Vec {
  return values.map((v) => (v > eps ? 1 / Math.sqrt(v) : 0));
}

// ── Two-sided SVD via the symmetric EVDs of AᵀA and AAᵀ ───────────────────────
// For an m×n matrix A, A = U · S · Vᵀ.
//   - if m ≥ n: AᵀA = V · S² · Vᵀ   → V from jacobiEVD(AᵀA); U = A·V·S⁻¹
//   - if m <  n: AAᵀ = U · S² · Uᵀ   → U from jacobiEVD(AAᵀ); V = Aᵀ·U·S⁻¹
// Singular values are the square roots of the non-negative eigenvalues (clamped).

export interface SVD {
  U: Mat;
  S: Vec; // singular values, sorted descending
  V: Mat; // right singular vectors, column j = V[:,j]
}

export function svd(A: Mat): SVD {
  const m = A.length;
  if (m === 0) throw new Error('svd: empty matrix');
  const n = A[0].length;
  for (const row of A) if (row.length !== n) throw new Error('svd: ragged matrix');
  if (!matFinite(A)) throw new Error('svd: non-finite input');

  let U: Mat, V: Mat, svals: Vec;
  if (m >= n) {
    const AtA = matmul(transpose(A), A); // n×n
    const evd = jacobiEVD(AtA);
    V = evd.vectors;
    // singular values = sqrt(clamped eigenvalues)
    svals = evd.values.map((l) => Math.sqrt(Math.max(0, l)));
    // U = A · V · S⁻¹
    const AV = matmul(A, V);
    U = AV.map((row) => row.map((v, j) => (svals[j] > 1e-12 ? v / svals[j] : 0)));
  } else {
    const AAt = matmul(A, transpose(A)); // m×m
    const evd = jacobiEVD(AAt);
    U = evd.vectors;
    svals = evd.values.map((l) => Math.sqrt(Math.max(0, l)));
    const AtU = matmul(transpose(A), U);
    V = AtU.map((row) => row.map((v, j) => (svals[j] > 1e-12 ? v / svals[j] : 0)));
  }

  // sort descending by singular value (stable insertion sort — small n)
  const order = svals.map((_, i) => i).sort((i, j) => svals[j] - svals[i]);
  const S = order.map((i) => svals[i]);
  const Uo = U.map((row) => order.map((i) => row[i]));
  const Vo = V.map((row) => order.map((i) => row[i]));
  return { U: Uo, S, V: Vo };
}

// ── PCA (built on SVD) ────────────────────────────────────────────────────────

export interface PCA {
  mean: Vec; // per-feature mean
  components: Mat; // rows = principal axes (descending variance), each length = nFeat
  explainedVariance: Vec; // singular-value² / (n-1), per axis (descending)
  singularValues: Vec;
}

/**
 * Fit PCA on row-samples X (each row = one observation of nFeat features).
 * Deterministic: centers by the exact sample mean, decomposes the centered
 * matrix via SVD. No sklearn/RNG dependency.
 */
export function pcaFit(X: Mat): PCA {
  const n = X.length;
  if (n < 2) throw new Error('pcaFit: need ≥2 samples');
  const d = X[0].length;
  for (const row of X) if (row.length !== d) throw new Error('pcaFit: ragged X');
  if (!matFinite(X)) throw new Error('pcaFit: non-finite X');

  const mean = new Array(d).fill(0);
  for (const row of X) for (let j = 0; j < d; j++) mean[j] += row[j];
  for (let j = 0; j < d; j++) mean[j] /= n;

  const Xc: Mat = X.map((row) => row.map((v, j) => v - mean[j]));
  const { U, S, V } = svd(Xc); // Xc = U·diag(S)·Vᵀ
  // principal axes = feature-space vectors (length d). svd returns V as the
  // right singular matrix: for m≥n it is n×n with axes in its ROWS; for m<n
  // it is d×k with axes in its COLUMNS. Normalize to rows-of-length-d.
  const components = Xc.length >= Xc[0].length ? V : transpose(V);
  const explainedVariance = S.map((s) => (s * s) / (n - 1));
  return { mean, components, explainedVariance, singularValues: S };
}

/** Project a sample onto the top-k principal axes (returns the k-dim latent). */
export function pcaProject(pca: PCA, x: Vec, k?: number): Vec {
  if (x.length !== pca.mean.length) throw new Error('pcaProject: dim mismatch');
  const kk = k ?? pca.components.length;
  const centered = x.map((v, j) => v - pca.mean[j]);
  const out: Vec = [];
  for (let i = 0; i < kk; i++) {
    const axis = pca.components[i];
    let dot = 0;
    for (let j = 0; j < axis.length; j++) dot += axis[j] * centered[j];
    out.push(dot);
  }
  return out;
}

/** Reconstruct a sample from its top-k latent (inverse projection + mean). */
export function pcaReconstruct(pca: PCA, latent: Vec, k?: number): Vec {
  const kk = k ?? latent.length;
  const d = pca.mean.length;
  const recon = new Array(d).fill(0);
  for (let i = 0; i < kk; i++) {
    const axis = pca.components[i];
    const w = latent[i];
    for (let j = 0; j < d; j++) recon[j] += w * axis[j];
  }
  for (let j = 0; j < d; j++) recon[j] += pca.mean[j];
  return recon;
}
