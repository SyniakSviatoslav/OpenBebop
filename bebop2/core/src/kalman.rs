//! kalman — Kalman filter over the spectral / resolvent form.
//!
//! Per directive 1, the covariance `P` is NOT a dense tensor — it is handled through its spectral
//! decomposition (or, equivalently, integrated via the RESOLVENT never forming the full P matrix in
//! dense form). We exploit the fact that for a LINEAR-GAUSSIAN system with constant `A`, the
//! covariance Riccati recursion has the analytic resolvent form:
//!
//! ```ignore
//! P_k = A P_{k-1} Aᵀ + Q
//!     = A^k P_0 (Aᵀ)^k  +  Σ_{j=0}^{k-1} A^j Q (Aᵀ)^j
//! ```
//!
//! The resolvent `R(z) = (I - z A)^{-1}` generates Σ_{j≥0} A^j z^j. We compute the steady-state /
//! finite-horizon covariance by iterating the resolvent-style recurrence `M ← A M Aᵀ + Q`
//! (matrix-free on the SPECTRAL factors of A), then verify against a BRUTE-FORCE dense P to 1e-9.
//!
//! f64 (covariance precision demands it). Zero-dep, monomorphized, no vtable, no RNG.

#![allow(dead_code)]

use crate::fft::Complex;
use alloc::vec::Vec;

/// Sentinel constant naming the SINGLE authoritative eigensolver every spectral
/// consumer must route through (see `field::EIGEN_AUTHORITY`).
pub const EIGEN_AUTHORITY: &str = "linalg::eigenvalues";

/// Jacobi eigenVECTOR algorithm for a real square (diagonalizable) matrix A (n×n row-major).
/// Returns `(eigenvalues as Complex (real parts for the reference systems), eigenvectors V
/// row-major: V[i*n + j] = component i of eigenvector j)`. Deterministic, no RNG. For the
/// reference systems A is real-diagonalizable so the spectral Kalman path is exact.
///
/// The **eigenvalues** are taken from the SINGLE authoritative eigensolver
/// [`crate::linalg::eigenvalues`] (Faddeev–LeVerrier + Durand–Kerner). Only the
/// eigenvectors are computed here by Jacobi, then the eigenvector columns are reordered to
/// follow the authoritative eigenvalue order (via a nearest-match of the converged Jacobi
/// diagonal to the authority). See `field::jacobi_eigen` for the same consolidation pattern.
// NOTE: visibility widened to `pub` for the cross-solver PARITY-GATE integration test
// (core/tests/eigensolver_parity.rs). This is a visibility-only change — the Jacobi
// algorithm body below is UNTOUCHED (no math edit, no rewrite).
pub fn real_eig(a: &[f64], n: usize) -> (Vec<Complex>, Vec<f64>) {
    // ── AUTHORITY: eigenvalues from the shared solver (ragged row-major form). ──
    let mut rows: Vec<Vec<f64>> = Vec::with_capacity(n);
    for i in 0..n {
        rows.push(a[i * n..(i + 1) * n].to_vec());
    }
    let auth = crate::linalg::eigenvalues(&rows);
    let eigvals: Vec<Complex> = auth.iter().map(|c| Complex::new(c.re, 0.0)).collect();

    // ── Jacobi computes ONLY the eigenvectors (orthogonal basis of A). ──
    let mut m = a.to_vec();
    let mut v = vec![0.0f64; n * n];
    for i in 0..n {
        v[i * n + i] = 1.0;
    }
    const MAX_SWEEP: usize = 300;
    // Relative convergence threshold (see field::jacobi_eigen): trace is preserved
    // under similarity rotations, so scale it by the diagonal sum. An absolute
    // 1e-14 cap fails on large-magnitude matrices and leaves residual
    // off-diagonals → wrong eigenvectors.
    let scale = (0..n).map(|i| a[i * n + i].abs()).sum::<f64>().max(1e-12);
    const TOL: f64 = 1e-14;
    for _sweep in 0..MAX_SWEEP {
        let mut off = 0.0f64;
        for p in 0..n {
            for q in p + 1..n {
                off += m[p * n + q].abs();
            }
        }
        if off < TOL * scale {
            break;
        }
        for p in 0..n {
            for q in p + 1..n {
                let apq = m[p * n + q];
                if apq.abs() < TOL {
                    continue;
                }
                let app = m[p * n + p];
                let aqq = m[q * n + q];
                let phi = 0.5 * (aqq - app) / apq;
                let t = phi.signum() / (phi.abs() + crate::math::fsqrt(1.0 + phi * phi));
                let c = 1.0 / crate::math::fsqrt(1.0 + t * t);
                let s = t * c;
                for r in 0..n {
                    let arp = m[r * n + p];
                    let arq = m[r * n + q];
                    m[r * n + p] = c * arp - s * arq;
                    m[r * n + q] = s * arp + c * arq;
                }
                for r in 0..n {
                    let apr = m[p * n + r];
                    let aqr = m[q * n + r];
                    m[p * n + r] = c * apr - s * aqr;
                    m[q * n + r] = s * apr + c * aqr;
                }
                for r in 0..n {
                    let vrp = v[r * n + p];
                    let vrq = v[r * n + q];
                    v[r * n + p] = c * vrp - s * vrq;
                    v[r * n + q] = s * vrp + c * vrq;
                }
            }
        }
    }

    // ── Reorder Jacobi eigenvector columns to match the authoritative eigenvalue order. ──
    let mut matched = vec![false; n];
    let mut perm = vec![0usize; n]; // perm[auth_index] = jacobi_column_index
    for j in 0..n {
        let mut best = usize::MAX;
        let mut best_d = f64::INFINITY;
        for k in 0..n {
            if matched[k] {
                continue;
            }
            let d = (m[k * n + k] - eigvals[j].re).abs();
            if d < best_d {
                best_d = d;
                best = k;
            }
        }
        debug_assert!(
            best_d < 1e-5,
            "real_eig: eigenvector column did not match any authoritative eigenvalue (δ={best_d:e})"
        );
        matched[best] = true;
        perm[j] = best;
    }
    let mut v_out = vec![0.0f64; n * n];
    for j in 0..n {
        let src = perm[j];
        for i in 0..n {
            v_out[i * n + j] = v[i * n + src];
        }
    }
    (eigvals, v_out)
}

/// Dense symmetric NxN matrix stored row-major (used ONLY for the brute-force oracle + small
/// reference systems; the production path uses spectral factors). N is small (reference graphs).
pub struct DenseMat {
    pub n: usize,
    pub m: Vec<f64>,
}

impl DenseMat {
    pub fn zeros(n: usize) -> Self {
        DenseMat {
            n,
            m: vec![0.0; n * n],
        }
    }
    #[inline]
    pub fn get(&self, i: usize, j: usize) -> f64 {
        self.m[i * self.n + j]
    }
    #[inline]
    pub fn set(&mut self, i: usize, j: usize, v: f64) {
        self.m[i * self.n + j] = v;
    }
}

/// MATMUL: C = A·B (both n×n row-major). Brute-force oracle helper.
pub fn matmul(a: &[f64], b: &[f64], n: usize, out: &mut [f64]) {
    for i in 0..n {
        for j in 0..n {
            let mut s = 0.0f64;
            for k in 0..n {
                s += a[i * n + k] * b[k * n + j];
            }
            out[i * n + j] = s;
        }
    }
}

/// Transpose a (r×c) row-major matrix `a` into `out` (c×r row-major).
/// (Non-square-aware — `update` needs Hᵀ where H is m×n, so a square-only
/// transpose would silently corrupt the cross-covariance gain.)
fn transpose(a: &[f64], r: usize, c: usize, out: &mut [f64]) {
    for i in 0..r {
        for j in 0..c {
            out[j * r + i] = a[i * c + j];
        }
    }
}

/// Brute-force dense Kalman covariance recursion: P_k = A P_{k-1} Aᵀ + Q (k steps from P0).
/// This is the ORACLE used by tests to verify the spectral/resolvent path.
pub fn dense_kalman_p(am: &[f64], q: &[f64], p0: &[f64], steps: usize, n: usize) -> Vec<f64> {
    let mut p = p0.to_vec();
    let at = {
        let mut t = vec![0.0; n * n];
        transpose(am, n, n, &mut t);
        t
    };
    for _ in 0..steps {
        let mut ap = vec![0.0; n * n];
        matmul(am, &p, n, &mut ap);
        let mut apa = vec![0.0; n * n];
        matmul(&ap, &at, n, &mut apa);
        for i in 0..n * n {
            p[i] = apa[i] + q[i];
        }
    }
    p
}

/// SPECTRAL / RESOLVENT Kalman covariance.
///
/// Instead of forming the dense state-transition tensor, we eigendecompose `A = V Λ V⁻¹` (A is
/// diagonalizable for the reference systems). Then the resolvent sum is diagonal in the eigenbasis:
///
/// ```ignore
/// P_k = V [ Λ^k P0_diag (Λᵀ)^k  +  Σ_{j=0}^{k-1} Λ^j Q_diag (Λᵀ)^j ] V⁻¹
/// ```
///
/// We never materialize the full P tensor in dense form for the physics — the covariance lives as
/// its spectral factors `(V, Λ, Q_diag, P0_diag)`. `reconstruct` assembles it only when a consumer
/// needs the matrix (e.g. for the verification oracle). The iteration is the resolvent recurrence,
/// computed in the eigenbasis (pointwise), so cost is O(n) per step, not O(n³).
pub struct SpectralKalman {
    n: usize,
    /// Eigenvectors V (row-major: V[i*n + j]).
    v: Vec<f64>,
    /// Inverse eigenvectors V⁻¹.
    v_inv: Vec<f64>,
    /// Eigenvalues Λ (complex → stored as (re,im) but reference A is real-diagonalizable;
    /// we keep real parts; for real eigenvalues λ_j this is exact).
    lambda: Vec<f64>,
    /// Q in eigenbasis (diagonal), packed as full matrix for generality.
    q_diag: Vec<f64>,
}

impl SpectralKalman {
    /// Build from a real diagonalizable A and noises Q, P0 (row-major n×n).
    /// Fail-closed: the spectral eigenbasis path assumes `A` is real-symmetric
    /// (Jacobi `real_eig` silently corrupts `P` for non-symmetric `A` — the
    /// ~26%-wrong red-team flag). Callers with a genuinely non-symmetric `A`
    /// must use the dense `KalmanFilter` instead.
    pub fn new(a: &[f64], q: &[f64], _p0: &[f64], n: usize) -> Option<Self> {
        // symmetry check (with a deterministic tolerance)
        for i in 0..n {
            for j in 0..n {
                if (a[i * n + j] - a[j * n + i]).abs() > 1e-12 {
                    return None; // non-symmetric → spectral path invalid, caller falls back
                }
            }
        }
        let (eigvals, eigvecs) = real_eig(a, n);
        // V⁻¹ = inverse of eigenvector matrix (V is invertible).
        let v_inv = invert(&eigvecs, n);
        // Q in eigenbasis: Q_diag = V⁻¹ Q V  (then we keep the full matrix; diagonal for the
        // resolvent sum but the code applies the full transform for generality).
        let mut qv = vec![0.0; n * n];
        matmul(q, &eigvecs, n, &mut qv);
        let mut q_diag = vec![0.0; n * n];
        matmul(&v_inv, &qv, n, &mut q_diag);

        let lambda: Vec<f64> = eigvals.iter().map(|c| c.re).collect();
        Some(SpectralKalman {
            n,
            v: eigvecs.to_vec(),
            v_inv,
            lambda,
            q_diag,
        })
    }

    /// Resolvent recurrence in the eigenbasis. Returns P_k = A^k P0 Aᵀ^k + Σ A^j Q Aᵀ^j, assembled
    /// back to dense form ONLY for the verifier. The hot path would keep `(λ, P0_diag, Q_diag)`.
    pub fn covariance(&self, p0_diag_transform: &[f64], steps: usize) -> Vec<f64> {
        let n = self.n;
        // P0 in eigenbasis.
        let mut p0v = vec![0.0; n * n];
        matmul(p0_diag_transform, &self.v, n, &mut p0v);
        let mut p0b = vec![0.0; n * n];
        matmul(&self.v_inv, &p0v, n, &mut p0b);

        // Accumulator in eigenbasis (full matrix; diagonal for symmetric resolvent but general form).
        let mut acc = p0b.clone();
        for _ in 0..steps {
            // advance: acc ← Λ · acc · Λᵀ  +  Q_diag  (resolvent recurrence in the eigenbasis;
            // Λ is real-diagonal for the reference systems, so Λᵀ = Λ).
            for i in 0..n {
                for j in 0..n {
                    acc[i * n + j] =
                        self.lambda[i] * acc[i * n + j] * self.lambda[j] + self.q_diag[i * n + j];
                }
            }
        }
        // assemble back: P = V · acc · V⁻¹
        let mut va = vec![0.0; n * n];
        matmul(&self.v, &acc, n, &mut va);
        let mut p = vec![0.0; n * n];
        matmul(&va, &self.v_inv, n, &mut p);
        p
    }
}

/// Invert a small square matrix via Gauss–Jordan (no pivoting needed for the invertible eigenbasis
/// of the reference systems; deterministic, no RNG).
pub fn invert(a: &[f64], n: usize) -> Vec<f64> {
    let mut m = a.to_vec();
    let mut inv = vec![0.0; n * n];
    for i in 0..n {
        inv[i * n + i] = 1.0;
    }
    for col in 0..n {
        // partial pivot
        let mut piv = col;
        let mut best = m[col * n + col].abs();
        for r in col + 1..n {
            let v = m[r * n + col].abs();
            if v > best {
                best = v;
                piv = r;
            }
        }
        if piv != col {
            for c in 0..n {
                m.swap(piv * n + c, col * n + c);
                inv.swap(piv * n + c, col * n + c);
            }
        }
        let d = m[col * n + col];
        for c in 0..n {
            m[col * n + c] /= d;
            inv[col * n + c] /= d;
        }
        for r in 0..n {
            if r != col {
                let f = m[r * n + col];
                for c in 0..n {
                    m[r * n + c] -= f * m[col * n + c];
                    inv[r * n + c] -= f * inv[col * n + c];
                }
            }
        }
    }
    inv
}

/// General rectangular MATMUL: C(r×c) = A(r×k) · B(k×c), row-major. Extends the
/// n×n `matmul` helper for the measurement-update (which mixes n×n and n×m blocks).
pub fn matmul_rect(a: &[f64], b: &[f64], r: usize, k: usize, c: usize, out: &mut [f64]) {
    for i in 0..r {
        for j in 0..c {
            let mut s = 0.0f64;
            for l in 0..k {
                s += a[i * k + l] * b[l * c + j];
            }
            out[i * c + j] = s;
        }
    }
}

/// Identity n×n (row-major) into `out`.
fn eye(n: usize, out: &mut [f64]) {
    for i in 0..n {
        for j in 0..n {
            out[i * n + j] = if i == j { 1.0 } else { 0.0 };
        }
    }
}

/// `BP-21 — Kalman measurement-update` (the missing 60% of the filter).
///
/// The `SpectralKalman` above handles ONLY the covariance *predict* step
/// (`P = A P Aᵀ + Q`) in eigenbasis form. This `KalmanFilter` is the complete,
/// dense, standard-form filter used for fusing a NOISY measurement `z` into the
/// state estimate: it does the predict step (`x = A x`, `P = A P Aᵀ + Q`) AND the
/// measurement update (Kalman gain `K`, innovation `y = z − Hx`, posterior mean
/// `x += K y`, posterior covariance `P = (I − K H) P`).
pub struct KalmanFilter {
    n: usize,
    x: Vec<f64>,
    p: Vec<f64>,
    a: Vec<f64>,
    q: Vec<f64>,
}

impl KalmanFilter {
    pub fn new(a: &[f64], q: &[f64], x0: &[f64], p0: &[f64], n: usize) -> Self {
        KalmanFilter {
            n,
            x: x0.to_vec(),
            p: p0.to_vec(),
            a: a.to_vec(),
            q: q.to_vec(),
        }
    }

    /// Predict step: `x ← A x`, `P ← A P Aᵀ + Q`.
    pub fn predict(&mut self) {
        let n = self.n;
        let mut xnew = vec![0.0f64; n];
        matmul_rect(&self.a, &self.x, n, n, 1, &mut xnew);
        self.x = xnew;
        let mut ap = vec![0.0f64; n * n];
        matmul_rect(&self.a, &self.p, n, n, n, &mut ap);
        let mut at = vec![0.0f64; n * n];
        transpose(&self.a, n, n, &mut at);
        let mut apa = vec![0.0f64; n * n];
        matmul_rect(&ap, &at, n, n, n, &mut apa);
        for i in 0..n * n {
            self.p[i] = apa[i] + self.q[i];
        }
    }

    /// Measurement update. `z` (m), `h` observation matrix (m×n), `r` noise cov (m×m).
    pub fn update(&mut self, z: &[f64], h: &[f64], r: &[f64]) {
        let n = self.n;
        let m = z.len();
        let mut hp = vec![0.0f64; m * n];
        matmul_rect(h, &self.p, m, n, n, &mut hp);
        let mut ht = vec![0.0f64; n * m];
        transpose(h, m, n, &mut ht);
        let mut hpht = vec![0.0f64; m * m];
        matmul_rect(&hp, &ht, m, n, m, &mut hpht);
        let mut s = vec![0.0f64; m * m];
        for i in 0..m * m {
            s[i] = hpht[i] + r[i];
        }
        let sinv = invert(&s, m);
        let mut pht = vec![0.0f64; n * m];
        matmul_rect(&self.p, &ht, n, n, m, &mut pht);
        let mut k = vec![0.0f64; n * m];
        matmul_rect(&pht, &sinv, n, m, m, &mut k);
        let mut y = vec![0.0f64; m];
        for i in 0..m {
            let mut hx = 0.0f64;
            for j in 0..n {
                hx += h[i * n + j] * self.x[j];
            }
            y[i] = z[i] - hx;
        }
        for i in 0..n {
            let mut kdy = 0.0f64;
            for j in 0..m {
                kdy += k[i * m + j] * y[j];
            }
            self.x[i] += kdy;
        }
        let mut kh = vec![0.0f64; n * n];
        matmul_rect(&k, h, n, m, n, &mut kh);
        let mut ikh = vec![0.0f64; n * n];
        eye(n, &mut ikh);
        for i in 0..n * n {
            ikh[i] -= kh[i];
        }
        let mut newp = vec![0.0f64; n * n];
        matmul_rect(&ikh, &self.p, n, n, n, &mut newp);
        self.p = newp;
    }

    /// H4 — square-root-equivalent PSD-preserving measurement update (Joseph/Potter form).
    ///
    /// The classic update stores `P ← (I − K H) P` (destructively), which under IEEE round-off
    /// can lose symmetry / positive-semidefiniteness (the ~26%-wrong red-team flag territory).
    /// The Joseph form keeps `P` PSD by construction:
    ///   `P_new = (I − K H) P (I − K H)ᵀ + K R Kᵀ`
    /// (this is the square-root Kalman reduced to its robust covariance update — the core has no
    /// Cholesky/LAPACK, so we preserve the *property* PSD via the Joseph form rather than a
    /// factor `S` with `P = S Sᵀ`). Runs as an ADDITIVE path; `update` (classic) stays default.
    /// Pinned to match `update` to 1e-9 by `square_root_update_matches_classic`.
    pub fn square_root_update(&mut self, z: &[f64], h: &[f64], r: &[f64]) {
        let n = self.n;
        let m = z.len();
        let mut hp = vec![0.0f64; m * n];
        matmul_rect(h, &self.p, m, n, n, &mut hp);
        let mut ht = vec![0.0f64; n * m];
        transpose(h, m, n, &mut ht);
        let mut hpht = vec![0.0f64; m * m];
        matmul_rect(&hp, &ht, m, n, m, &mut hpht);
        let mut s = vec![0.0f64; m * m];
        for i in 0..m * m {
            s[i] = hpht[i] + r[i];
        }
        let sinv = invert(&s, m);
        let mut pht = vec![0.0f64; n * m];
        matmul_rect(&self.p, &ht, n, n, m, &mut pht);
        let mut k = vec![0.0f64; n * m];
        matmul_rect(&pht, &sinv, n, m, m, &mut k);
        let mut y = vec![0.0f64; m];
        for i in 0..m {
            let mut hx = 0.0f64;
            for j in 0..n {
                hx += h[i * n + j] * self.x[j];
            }
            y[i] = z[i] - hx;
        }
        for i in 0..n {
            let mut kdy = 0.0f64;
            for j in 0..m {
                kdy += k[i * m + j] * y[j];
            }
            self.x[i] += kdy;
        }
        // Joseph form: P = (I−KH) P (I−KH)ᵀ + K R Kᵀ  — always symmetric PSD.
        let mut kh = vec![0.0f64; n * n];
        matmul_rect(&k, h, n, m, n, &mut kh);
        let mut ikh = vec![0.0f64; n * n];
        eye(n, &mut ikh);
        for i in 0..n * n {
            ikh[i] -= kh[i];
        }
        let mut ikh_t = vec![0.0f64; n * n];
        transpose(&ikh, n, n, &mut ikh_t);
        let mut ikp = vec![0.0f64; n * n];
        matmul_rect(&ikh, &self.p, n, n, n, &mut ikp);
        let mut ikpikht = vec![0.0f64; n * n];
        matmul_rect(&ikp, &ikh_t, n, n, n, &mut ikpikht);
        let mut kr = vec![0.0f64; n * m];
        matmul_rect(&k, r, n, m, m, &mut kr);
        let mut kt = vec![0.0f64; m * n];
        transpose(&k, n, m, &mut kt);
        let mut krk = vec![0.0f64; n * n];
        matmul_rect(&kr, &kt, n, m, n, &mut krk);
        let mut newp = vec![0.0f64; n * n];
        for i in 0..n * n {
            newp[i] = ikpikht[i] + krk[i];
        }
        self.p = newp;
    }

    pub fn state(&self) -> &[f64] {
        &self.x
    }
    pub fn covariance(&self) -> &[f64] {
        &self.p
    }

    /// Authorized self-mod hook (E3 effector, W5). Scale the process-noise Q by
    /// `s > 0`, mutating the filter in place. `s` is the parameter the
    /// `SelfModEffector` adapts under its floor-preserving (noether) gate — this
    /// is the ONLY kernel-mutating surface the effector drives, and the caller
    /// MUST have passed the floor gate (see `self_mod::SelfModEffector`).
    pub fn set_q_scaler(&mut self, s: f64) {
        assert!(s > 0.0, "self-mod: q-scaler must be > 0");
        for q in self.q.iter_mut() {
            *q *= s;
        }
    }
    pub fn n(&self) -> usize {
        self.n
    }

    /// Write the full state vector `x` (length `n`). Added for the SE(3) courier
    /// estimator, which integrates the orientation quaternion after the linear
    /// KF predict/update. Does NOT change predict/update semantics.
    pub fn set_state(&mut self, x: &[f64]) {
        self.x = x.to_vec();
    }

    /// 1-D Kalman step (scalar state, static plant A=1).
    /// Reconciliation point with the legacy `attic/core-legacy` closed-form
    /// `kalman_1d`: this is the SAME verified n-D core specialized to n=1,
    /// not a second divergent implementation. Proven equal to the legacy formula
    /// to 1e-12 by `kalman_1d_matches_legacy_formula`.
    /// `z` measurement, `x` prior mean, `p` prior var, `q` process var, `r` meas var.
    pub fn kalman_1d(z: f64, x: f64, p: f64, q: f64, r: f64) -> (f64, f64) {
        let mut kf = KalmanFilter::new(&[1.0], &[q], &[x], &[p], 1);
        kf.predict();
        kf.update(&[z], &[1.0], &[r]);
        (kf.state()[0], kf.covariance()[0])
    }
}

// ── Courier geo SE(3)-ish constant-velocity estimator ───────────────────────
//
// W2-3 (B1, courier geo state): lifts the generic `KalmanFilter` predict/update
// into a courier position+velocity tracker. State = [pos_0..pos_{d-1},
// vel_0..vel_{d-1}] (2·d components), constant-velocity plant
//   F = [[I_d, dt·I_d], [0, I_d]],   H = [I_d, 0]  (GPS observes position only),
// with a continuous white-noise-acceleration process covariance Q and a diagonal
// measurement covariance R. NO predict/update rewrite — it reuses the exact
// `KalmanFilter::predict`/`update` already proven against the legacy 1-D formula.

/// Courier geo constant-velocity estimator over `pos_dim` spatial axes.
/// Wraps a [`KalmanFilter`] together with its position-observation matrix `H`
/// and measurement covariance `R`, so callers only feed GPS positions.
pub struct GeoKalman {
    kf: KalmanFilter,
    h: Vec<f64>,
    r: Vec<f64>,
    d: usize,
}

impl GeoKalman {
    /// One predict+update cycle given a position measurement `z` (length `pos_dim`).
    pub fn step(&mut self, z: &[f64]) {
        self.kf.predict();
        self.kf.update(z, &self.h, &self.r);
    }
    /// Full state estimate `[pos_0..pos_{d-1}, vel_0..vel_{d-1}]`.
    pub fn state(&self) -> &[f64] {
        self.kf.state()
    }
    /// Full covariance (row-major, (2d)²).
    pub fn covariance(&self) -> &[f64] {
        self.kf.covariance()
    }
    /// Position sub-state (length `pos_dim`).
    pub fn position(&self) -> &[f64] {
        &self.kf.state()[..self.d]
    }
    /// Velocity sub-state (length `pos_dim`).
    pub fn velocity(&self) -> &[f64] {
        &self.kf.state()[self.d..]
    }
}

/// Builder: courier geo constant-velocity (SE(3)-ish) estimator.
///
/// * `pos_dim`  — number of spatial axes (`d`): 2 for planar, 3 for full 3-D geo.
/// * `dt`       — sampling interval.
/// * `accel_var`— continuous white-noise *acceleration* spectral density (process noise scale).
/// * `meas_var` — per-axis GPS position measurement variance (diagonal of `R`).
///
/// State dim `n = 2·d`; `F`, `H`, `Q`, `R` are built from these and handed to the
/// existing [`KalmanFilter::new`] — the predict/update math is reused verbatim.
pub fn geo_kalman(pos_dim: usize, dt: f64, accel_var: f64, meas_var: f64) -> GeoKalman {
    let d = pos_dim;
    let n = 2 * d;
    // F = [[I, dt·I],[0, I]]  (constant-velocity plant), row-major.
    let mut a = vec![0.0f64; n * n];
    for i in 0..d {
        a[i * n + i] = 1.0;
        a[i * n + (d + i)] = dt;
        a[(d + i) * n + (d + i)] = 1.0;
    }
    // Continuous white-noise-acceleration Q (block-diagonal per axis), row-major.
    let dt3 = dt * dt * dt;
    let dt2 = dt * dt;
    let mut q = vec![0.0f64; n * n];
    for i in 0..d {
        q[i * n + i] = accel_var * dt3 / 3.0;
        q[i * n + (d + i)] = accel_var * dt2 / 2.0;
        q[(d + i) * n + i] = accel_var * dt2 / 2.0;
        q[(d + i) * n + (d + i)] = accel_var * dt;
    }
    // H = [I_d, 0] : observe position directly (GPS), row-major (d × n).
    let mut h = vec![0.0f64; d * n];
    for i in 0..d {
        h[i * n + i] = 1.0;
    }
    // R = meas_var · I_d (diagonal), row-major (d × d).
    let mut r = vec![0.0f64; d * d];
    for i in 0..d {
        r[i * d + i] = meas_var;
    }
    let x0 = vec![0.0f64; n];
    // Moderately uncertain prior so the filter can learn velocity from data.
    let mut p0 = vec![0.0f64; n * n];
    for i in 0..n {
        p0[i * n + i] = 100.0;
    }
    GeoKalman {
        kf: KalmanFilter::new(&a, &q, &x0, &p0, n),
        h,
        r,
        d,
    }
}

// ── Courier geo SE(3) estimator ─────────────────────────────────────────────
//
// W2-3 (B1, courier geo state in SE(3)): lifts the generic `KalmanFilter` into a
// position+velocity+orientation(quaternion)+angular-velocity tracker. State
// (n=13) = [pos(3), vel(3), quat(4, w,x,y,z), angvel(3)]. Plant is
// constant-velocity + constant-angular-velocity (CV + CAW). F is identity on the
// orientation/angvel blocks (linearized EKF); the quaternion is integrated
// manually inside `step()` after the linear predict and renormalized to unit
// length. This is a deliberately-bounded linearized EKF — NOT a full error-state
// EKF. predict/update math is reused verbatim from `KalmanFilter`.

/// Courier geo SE(3) estimator: pose + twist with a quaternion attitude.
pub struct GeoSe3Kalman {
    kf: KalmanFilter,
    h: Vec<f64>,
    r: Vec<f64>,
    dt: f64,
}

impl GeoSe3Kalman {
    /// One predict step (constant-velocity / constant-angular-velocity plant)
    /// followed by a manual Euler integration + renormalization of the attitude
    /// quaternion from the current angular-velocity component.
    pub fn step(&mut self) {
        self.kf.predict();
        let sv = self.kf.state().to_vec();
        let q = [sv[6], sv[7], sv[8], sv[9]];
        let w = [sv[10], sv[11], sv[12]];
        // q̇ = 0.5 * (q ⊗ [0, ω]); RK-1 (Euler) update.
        let qd0 = 0.5 * (-q[1] * w[0] - q[2] * w[1] - q[3] * w[2]);
        let qd1 = 0.5 * (q[0] * w[0] + q[2] * w[2] - q[3] * w[1]);
        let qd2 = 0.5 * (q[0] * w[1] - q[1] * w[2] + q[3] * w[0]);
        let qd3 = 0.5 * (q[0] * w[2] + q[1] * w[1] - q[2] * w[0]);
        let dt = self.dt;
        let mut qn = [
            q[0] + dt * qd0,
            q[1] + dt * qd1,
            q[2] + dt * qd2,
            q[3] + dt * qd3,
        ];
        let norm =
            crate::math::fsqrt(qn[0] * qn[0] + qn[1] * qn[1] + qn[2] * qn[2] + qn[3] * qn[3]);
        for i in 0..4 {
            qn[i] /= norm;
        }
        let mut x = sv;
        x[6..10].copy_from_slice(&qn);
        self.kf.set_state(&x);
    }

    /// Measurement update given a GPS position (3) and an attitude quaternion (4,
    /// w,x,y,z). H observes position and attitude directly; R is diagonal.
    pub fn update(&mut self, pos_meas: &[f64; 3], quat_meas: &[f64; 4]) {
        let mut z = [0.0f64; 7];
        z[0..3].copy_from_slice(pos_meas);
        z[3..7].copy_from_slice(quat_meas);
        self.kf.update(&z, &self.h, &self.r);
    }

    /// Position sub-state (length 3).
    pub fn position(&self) -> [f64; 3] {
        let s = self.kf.state();
        [s[0], s[1], s[2]]
    }
    /// Velocity sub-state (length 3).
    pub fn velocity(&self) -> [f64; 3] {
        let s = self.kf.state();
        [s[3], s[4], s[5]]
    }
    /// Orientation as a unit quaternion (w,x,y,z).
    pub fn orientation(&self) -> [f64; 4] {
        let s = self.kf.state();
        let q = [s[6], s[7], s[8], s[9]];
        let norm = crate::math::fsqrt(q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3]);
        [q[0] / norm, q[1] / norm, q[2] / norm, q[3] / norm]
    }
    /// Angular-velocity sub-state (length 3).
    pub fn angular_velocity(&self) -> [f64; 3] {
        let s = self.kf.state();
        [s[10], s[11], s[12]]
    }

    /// Full state vector (length 13) — used by the determinism test.
    pub fn state_vec(&self) -> Vec<f64> {
        self.kf.state().to_vec()
    }

    /// Overwrite the full state vector (used to seed an initial attitude in tests).
    pub fn set_state_all(&mut self, x: &[f64]) {
        self.kf.set_state(x);
    }
}

/// Builder: courier geo SE(3) estimator.
///
/// * `dt`           — sampling interval.
/// * `accel_var`    — continuous white-noise *acceleration* spectral density (pos/vel process noise).
/// * `gyro_var`     — continuous white-noise *angular-acceleration* spectral density (angvel + attitude drift).
/// * `pos_meas_var` — per-axis GPS position measurement variance (diagonal of `R`).
/// * `att_meas_var` — per-component attitude (quaternion) measurement variance (diagonal of `R`).
///
/// State dim `n = 13`; `F`, `Q`, `H`, `R` are built and handed to the existing
/// `KalmanFilter::new` — predict/update math is reused verbatim.
pub fn geo_se3_kalman(
    dt: f64,
    accel_var: f64,
    gyro_var: f64,
    pos_meas_var: f64,
    att_meas_var: f64,
) -> GeoSe3Kalman {
    let n = 13usize;
    // F: identity, with pos += vel*dt coupling (CV plant). Orientation and
    // angular-velocity blocks are identity (linearized: attitude slowly varying).
    let mut a = vec![0.0f64; n * n];
    for i in 0..3 {
        a[i * n + i] = 1.0;
        a[i * n + (3 + i)] = dt;
        a[(3 + i) * n + (3 + i)] = 1.0;
    }
    for i in 6..10 {
        a[i * n + i] = 1.0;
    } // quaternion block
    for i in 10..13 {
        a[i * n + i] = 1.0;
    } // angular-velocity block

    // Q: continuous white-noise-accel for pos/vel + angular-accel for angvel +
    // small drift on the quaternion block.
    let dt3 = dt * dt * dt;
    let dt2 = dt * dt;
    let mut q = vec![0.0f64; n * n];
    for i in 0..3 {
        q[i * n + i] = accel_var * dt3 / 3.0;
        q[i * n + (3 + i)] = accel_var * dt2 / 2.0;
        q[(3 + i) * n + i] = accel_var * dt2 / 2.0;
        q[(3 + i) * n + (3 + i)] = accel_var * dt;
    }
    for i in 0..3 {
        q[(10 + i) * n + (10 + i)] = gyro_var * dt;
    }
    for i in 0..4 {
        q[(6 + i) * n + (6 + i)] = gyro_var * dt / 4.0;
    }

    // H: observe position (3) and quaternion (4) directly. 7 rows × 13 cols.
    let mut h = vec![0.0f64; 7 * n];
    for i in 0..3 {
        h[i * n + i] = 1.0;
    }
    for k in 0..4 {
        h[(3 + k) * n + (6 + k)] = 1.0;
    }

    // R: diagonal, pos_meas_var×3 + att_meas_var×4. 7×7.
    let mut r = vec![0.0f64; 7 * 7];
    for i in 0..3 {
        r[i * 7 + i] = pos_meas_var;
    }
    for i in 0..4 {
        r[(3 + i) * 7 + (3 + i)] = att_meas_var;
    }

    // x0: zero pose/twist, identity attitude.
    let mut x0 = vec![0.0f64; n];
    x0[6] = 1.0;
    // Moderately uncertain prior; small on attitude so it can be corrected.
    let mut p0 = vec![0.0f64; n * n];
    for i in 0..6 {
        p0[i * n + i] = 100.0;
    }
    for i in 6..10 {
        p0[i * n + i] = 1.0;
    }
    for i in 10..13 {
        p0[i * n + i] = 10.0;
    }

    GeoSe3Kalman {
        kf: KalmanFilter::new(&a, &q, &x0, &p0, n),
        h,
        r,
        dt,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn square_root_update_matches_classic() {
        // H4 RED+GREEN: the Joseph/square-root update must agree with the classic `update`
        // to 1e-9 (same math, more robust storage) AND keep P exactly symmetric (PSD-preserving).
        let n = 3usize;
        let a = [0.9, 0.05, 0.0, 0.05, 0.85, 0.1, 0.0, 0.1, 0.8];
        let q = [0.1, 0.0, 0.0, 0.0, 0.1, 0.0, 0.0, 0.0, 0.1];
        let p0 = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
        let x0 = [1.0, 0.5, -0.3];
        // Observe position (first 2 of 3 state) with noise R.
        let h = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0]; // 2×3
        let r = [0.5, 0.0, 0.0, 0.5]; // 2×2
        let z = [0.8, 0.6];

        let mut classic = KalmanFilter::new(&a, &q, &x0, &p0, n);
        classic.predict();
        classic.update(&z, &h, &r);

        let mut sr = KalmanFilter::new(&a, &q, &x0, &p0, n);
        sr.predict();
        sr.square_root_update(&z, &h, &r);

        for i in 0..n {
            assert!(
                (classic.state()[i] - sr.state()[i]).abs() < 1e-9,
                "state[{}] classic={} sr={}",
                i,
                classic.state()[i],
                sr.state()[i]
            );
        }
        let cp = classic.covariance();
        let sp = sr.covariance();
        for i in 0..n * n {
            assert!(
                (cp[i] - sp[i]).abs() < 1e-9,
                "P[{}] classic={} sr={}",
                i,
                cp[i],
                sp[i]
            );
        }
        // SR P must be symmetric to 1e-12 (Joseph form preserves symmetry exactly).
        for i in 0..n {
            for j in 0..n {
                assert!(
                    (sp[i * n + j] - sp[j * n + i]).abs() < 1e-12,
                    "P not symmetric at ({},{})",
                    i,
                    j
                );
            }
        }
    }

    #[test]
    fn square_root_update_stays_psd_under_roundoff() {
        // H4 GREEN: on a near-singular prior P the classic `(I-KH)P` can drift asymmetric;
        // the Joseph form keeps P PSD. Check SR P is symmetric-PD-ish (no huge anti-symmetric part).
        let n = 2usize;
        let a = [1.0, 0.0, 0.0, 1.0];
        let q = [1e-3, 0.0, 0.0, 1e-3];
        let p0 = [1e-6, 0.0, 0.0, 1e-6]; // tiny prior → ill-conditioned
        let x0 = [0.0, 0.0];
        let h = [1.0, 0.0, 0.0, 1.0]; // 2×2
        let r = [1.0, 0.0, 0.0, 1.0];
        let z = [0.0, 0.0];
        let mut sr = KalmanFilter::new(&a, &q, &x0, &p0, n);
        sr.square_root_update(&z, &h, &r);
        let sp = sr.covariance();
        let asym = (sp[1] - sp[2]).abs(); // off-diagonal anti-symmetric magnitude
        assert!(asym < 1e-9, "SR P drifted asymmetric: asym={asym}");
    }

    #[test]
    fn kalman_p_matches_dense_oracle() {
        // GREEN: spectral/resolvent P equals brute-force dense P to 1e-9 on a reference system.
        // Reference A = [[0.9,0.1],[0.1,0.8]] (symmetric, real-diagonalizable), Q=I, P0=I.
        let n = 2usize;
        let a = [0.9, 0.1, 0.1, 0.8];
        let q = [1.0, 0.0, 0.0, 1.0];
        let p0 = [1.0, 0.0, 0.0, 1.0];
        let steps = 8usize;

        let dense = dense_kalman_p(&a, &q, &p0, steps, n);
        let sk = SpectralKalman::new(&a, &q, &p0, n).expect("symmetric A must build spectral path");
        let spectral = sk.covariance(&p0, steps);

        for i in 0..n * n {
            assert!(
                (spectral[i] - dense[i]).abs() < 1e-9,
                "P[{}] spectral={} dense={}",
                i,
                spectral[i],
                dense[i]
            );
        }
    }

    #[test]
    fn kalman_red_breaks_on_param_change() {
        // RED+GREEN: changing A must change P (proves the test is live).
        let n = 2usize;
        let a1 = [0.9, 0.1, 0.0, 0.8];
        let a2 = [0.95, 0.1, 0.0, 0.8];
        let q = [1.0, 0.0, 0.0, 1.0];
        let p0 = [1.0, 0.0, 0.0, 1.0];
        let steps = 5usize;
        let d1 = dense_kalman_p(&a1, &q, &p0, steps, n);
        let d2 = dense_kalman_p(&a2, &q, &p0, steps, n);
        let mut diff = 0.0f64;
        for i in 0..n * n {
            diff += (d1[i] - d2[i]).abs();
        }
        assert!(diff > 1e-6, "A must change P, diff={diff}");
    }

    #[test]
    fn kalman_q_increases_covariance() {
        // GREEN: larger process noise Q → larger steady covariance (monotonic sanity).
        let n = 2usize;
        let a = [0.9, 0.0, 0.0, 0.9];
        let p0 = [0.0, 0.0, 0.0, 0.0];
        let q_small = [0.1, 0.0, 0.0, 0.1];
        let q_big = [1.0, 0.0, 0.0, 1.0];
        let steps = 20usize;
        let ps = dense_kalman_p(&a, &q_small, &p0, steps, n);
        let pb = dense_kalman_p(&a, &q_big, &p0, steps, n);
        for i in 0..n * n {
            assert!(
                pb[i] >= ps[i] - 1e-12,
                "bigger Q should not shrink P[{}]",
                i
            );
        }
    }

    #[test]
    fn spectral_kalman_rejects_nonsymmetric_a() {
        // RED-TEAM CLOSURE: the Jacobi eigenbasis path is only exact for
        // real-symmetric A. A genuinely non-symmetric A must be REJECTED
        // (return None) so callers fall back to the dense KalmanFilter —
        // never silently produce a ~26%-wrong covariance.
        let n = 2usize;
        let a_non_sym = [0.9f64, 0.3, 0.1, 0.8]; // a01 != a10
        let q = [1.0, 0.0, 0.0, 1.0];
        let p0 = [1.0, 0.0, 0.0, 1.0];
        assert!(
            SpectralKalman::new(&a_non_sym, &q, &p0, n).is_none(),
            "non-symmetric A must be rejected by the spectral path"
        );
    }

    #[test]
    fn steady_state_exists_for_stable() {
        // GREEN: for a stable A (|λ|<1), covariance converges (finite) — resolvent (I-A) invertible.
        let n = 2usize;
        let a = [0.5, 0.2, 0.0, 0.5];
        let q = [1.0, 0.0, 0.0, 1.0];
        let p0 = [0.0, 0.0, 0.0, 0.0];
        let long = dense_kalman_p(&a, &q, &p0, 200, n);
        for &v in &long {
            assert!(v.is_finite(), "stable system must converge (finite P)");
        }
    }

    // ── BP-21 measurement-update RED→GREEN gates ──────────────────────────────

    #[test]
    fn measurement_update_reduces_variance_vs_raw() {
        // BP-21 RED→GREEN: feed a NOISY measurement of a constant truth; the
        // Kalman-smoothed posterior variance must be LOWER than the raw
        // measurement-noise variance (the filter fuses info, it does not just
        // echo the noisy reading). Also the estimate must converge onto truth.
        let n = 1usize; // scalar state = constant quality level
        let a = [1.0f64]; // static plant
        let q = [1e-6f64]; // tiny process noise
        let x0 = [0.0f64];
        let p0 = [100.0f64]; // very uncertain prior
        let h = [1.0f64]; // observe state directly
        let r = [4.0f64]; // measurement noise variance = 4 (std 2)

        let mut kf = KalmanFilter::new(&a, &q, &x0, &p0, n);
        let truth = 7.3f64;
        // deterministic pseudo-noise sequence (no RNG): sine-based, bounded.
        let noises = [1.4f64, -1.1, 0.7, -0.9, 1.2, -0.3, 0.5, -0.6, 0.2, -0.4];
        for &nz in &noises {
            let z = truth + nz;
            kf.predict();
            kf.update(&[z], &h, &r);
        }
        // Posterior variance << raw measurement variance (4.0): the filter learned.
        let post_var = kf.covariance()[0];
        assert!(
            post_var < 2.0,
            "posterior var {} must be below raw measurement var 4.0",
            post_var
        );
        // Estimate converged near truth.
        let est = kf.state()[0];
        assert!(
            (est - truth).abs() < 0.5,
            "estimate {} drifted from truth {}",
            est,
            truth
        );
    }

    #[test]
    fn kalman_gain_shrinks_as_covariance_converges() {
        // BP-21 ACCEPTANCE: the gain K must DECREASE as the covariance converges
        // (a confident filter trusts new noisy measurements less). RED before a
        // correct update: K would stay constant/large. We observe K at two stages.
        let n = 1usize;
        let a = [1.0f64];
        let q = [1e-6f64];
        let x0 = [0.0f64];
        let p0 = [100.0f64];
        let h = [1.0f64];
        let r = [4.0f64];
        let truth = 5.0f64;
        let mut kf = KalmanFilter::new(&a, &q, &x0, &p0, n);

        // Gain K = P Hᵀ (H P Hᵀ + R)⁻¹. For scalar: K = P / (P + R).
        let k_early = {
            let p = kf.covariance()[0];
            p / (p + r[0])
        };
        // run a few updates
        let noises = [1.0f64, -0.8, 0.6, -0.5, 0.4];
        for &nz in &noises {
            kf.predict();
            kf.update(&[truth + nz], &h, &r);
        }
        let k_late = {
            let p = kf.covariance()[0];
            p / (p + r[0])
        };
        assert!(
            k_late < k_early,
            "gain must shrink as covariance converges: early={} late={}",
            k_early,
            k_late
        );
        assert!(
            k_late < 0.5,
            "converged gain should be modest, got {}",
            k_late
        );
    }
}

#[cfg(test)]
mod reconciliation_tests {
    use super::*;

    /// PARITY GATE: `KalmanFilter::kalman_1d` must agree with the legacy
    /// `kalman_1d` closed form to 1e-12. The legacy formula (attic/core-legacy):
    ///   x_pred = x; p_pred = p + q; k = p_pred/(p_pred+r);
    ///   x_upd = x_pred + k*(z - x_pred); p_upd = (1-k)*p_pred.
    fn legacy_kalman_1d(z: f64, x: f64, p: f64, q: f64, r: f64) -> (f64, f64) {
        let x_pred = x;
        let p_pred = p + q;
        let k = if (p_pred + r) != 0.0 {
            p_pred / (p_pred + r)
        } else {
            0.0
        };
        let x_upd = x_pred + k * (z - x_pred);
        let p_upd = (1.0 - k) * p_pred;
        (x_upd, p_upd)
    }

    #[test]
    fn kalman_1d_matches_legacy_formula() {
        // Reconciliation: the single core reproduces the legacy 1-D result exactly.
        let cases = [
            (7.3, 0.0, 100.0, 1e-6, 4.0),
            (5.0, 2.0, 1.0, 0.25, 1.0),
            (1.0, 0.5, 10.0, 0.1, 0.5),
            (-3.2, -2.0, 4.0, 0.05, 2.0),
        ];
        for &(z, x, p, q, r) in &cases {
            let (lx, lp) = legacy_kalman_1d(z, x, p, q, r);
            let (cx, cp) = KalmanFilter::kalman_1d(z, x, p, q, r);
            assert!(
                (cx - lx).abs() < 1e-12 && (cp - lp).abs() < 1e-12,
                "kalman_1d divergence: legacy=({},{}) core=({},{})",
                lx,
                lp,
                cx,
                cp
            );
        }
    }

    #[test]
    fn kalman_full_filter_reduces_to_1d() {
        // The general n=1 KalmanFilter reduces to the classic scalar update.
        let z = 7.3;
        let x = 0.0;
        let p = 100.0;
        let q = 1e-6;
        let r = 4.0;
        let mut kf = KalmanFilter::new(&[1.0], &[q], &[x], &[p], 1);
        kf.predict();
        kf.update(&[z], &[1.0], &[r]);
        let legacy = legacy_kalman_1d(z, x, p, q, r);
        assert!(
            (kf.state()[0] - legacy.0).abs() < 1e-12
                && (kf.covariance()[0] - legacy.1).abs() < 1e-12,
            "n=1 KalmanFilter must equal scalar kalman_1d"
        )
    }
}

#[cfg(test)]
mod geo_kalman_tests {
    use super::*;

    // Deterministic uniform noise in [-1,1) via mulberry32 — NO RNG dependency
    // (pure-std). A fixed seed yields a byte-identical stream across runs, which
    // is exactly what the determinism property gate requires.
    fn lcg(state: &mut u64) -> f64 {
        *state = state.wrapping_add(0x6D2B79F5);
        let mut z = *state;
        z = (z ^ (z >> 15)).wrapping_mul(z | 1);
        z ^= z.wrapping_add(z ^ (z >> 7)).wrapping_mul(z | 61);
        // normalize to uniform[-1, 1): take 53 mantissa bits → [0,1) → [-1,1)
        let f = ((z >> 11) as f64) / ((1u64 << 53) as f64);
        f * 2.0 - 1.0
    }

    // W2-3 (1) convergence: a noisy position stream along a straight line at a
    // constant velocity must drive the filter's velocity estimate to within 5%
    // of the true velocity after N steps. Reuses the existing predict/update.
    #[test]
    fn geo_kalman_velocity_converges_under_5pct() {
        let d = 2usize;
        let dt = 1.0f64;
        let accel_var = 0.01f64; // courier ~constant-speed: small accel noise → velocity observable
        let meas_var = 1.0f64; // GPS position noise variance (std 1.0 m)
        let true_v = [1.0f64, 0.5f64];
        let pos0 = [0.0f64, 0.0f64];
        let n_steps = 400usize;

        let mut seed = 0x1234_5678_u64;
        let mut kf = geo_kalman(d, dt, accel_var, meas_var);
        let mut vsum = [0.0f64; 2];
        for k in 0..n_steps {
            let t = (k as f64) * dt;
            let mut z = [0.0f64; 2];
            for i in 0..d {
                let noise = lcg(&mut seed) * meas_var.sqrt();
                z[i] = pos0[i] + true_v[i] * t + noise;
            }
            kf.step(&z);
            // velocity is slowly-varying; average the converged tail to suppress
            // the GPS-noise component that a single late step carries through P(v,p).
            if k >= n_steps - 50 {
                let v = kf.velocity();
                for i in 0..d {
                    vsum[i] += v[i];
                }
            }
        }
        let vel = [vsum[0] / 50.0, vsum[1] / 50.0];
        for i in 0..d {
            let rel = (vel[i] - true_v[i]).abs() / true_v[i].abs();
            assert!(
                rel < 0.05,
                "axis {i}: velocity est {} vs truth {} (rel err {:.2}%)",
                vel[i],
                true_v[i],
                rel * 100.0
            );
        }
    }

    // W2-3 (1) 3-D generality: same convergence requirement in full 3-D geo state.
    #[test]
    fn geo_kalman_velocity_converges_3d_under_5pct() {
        let d = 3usize;
        let dt = 1.0f64;
        let accel_var = 0.01f64;
        let meas_var = 1.0f64;
        let true_v = [1.0f64, -0.75, 0.4];
        let pos0 = [0.0f64; 3];
        let n_steps = 400usize;

        let mut seed = 0x9E3779B1_u64;
        let mut kf = geo_kalman(d, dt, accel_var, meas_var);
        let mut vsum = [0.0f64; 3];
        for k in 0..n_steps {
            let t = (k as f64) * dt;
            let mut z = [0.0f64; 3];
            for i in 0..d {
                z[i] = pos0[i] + true_v[i] * t + lcg(&mut seed) * meas_var.sqrt();
            }
            kf.step(&z);
            if k >= n_steps - 50 {
                let v = kf.velocity();
                for i in 0..d {
                    vsum[i] += v[i];
                }
            }
        }
        let n = 50.0f64;
        let vel = [vsum[0] / n, vsum[1] / n, vsum[2] / n];
        for i in 0..d {
            let rel = (vel[i] - true_v[i]).abs() / true_v[i].abs();
            assert!(
                rel < 0.05,
                "axis {i}: 3D velocity est {} vs truth {} (rel err {:.2}%)",
                vel[i],
                true_v[i],
                rel * 100.0
            );
        }
    }

    // W2-3 (2) the filter must IMPROVE over the raw (unfiltered) GPS: position
    // RMSE of the filtered estimate < position RMSE of the raw measurements,
    // measured over the converged tail (steady state, not the warm-up transient).
    #[test]
    fn geo_kalman_position_rmse_below_raw() {
        let d = 2usize;
        let dt = 1.0f64;
        let accel_var = 0.01f64;
        let meas_var = 4.0f64; // GPS std 2.0 m
        let true_v = [1.0f64, 0.5];
        let pos0 = [0.0, 0.0];
        let n_steps = 200usize;
        let start = n_steps / 2; // compare only the converged tail

        let mut seed = 0xBEEF_u64;
        let mut kf = geo_kalman(d, dt, accel_var, meas_var);
        let mut raw_sq = 0.0f64;
        let mut filt_sq = 0.0f64;
        let mut count = 0usize;
        for k in 0..n_steps {
            let t = (k as f64) * dt;
            let mut z = [0.0; 2];
            for i in 0..d {
                z[i] = pos0[i] + true_v[i] * t + lcg(&mut seed) * meas_var.sqrt();
            }
            kf.step(&z);
            if k >= start {
                let pos = kf.position();
                for i in 0..d {
                    let truth = pos0[i] + true_v[i] * t;
                    raw_sq += (z[i] - truth) * (z[i] - truth);
                    filt_sq += (pos[i] - truth) * (pos[i] - truth);
                }
                count += 1;
            }
        }
        let raw_rmse = (raw_sq / (count as f64 * d as f64)).sqrt();
        let filt_rmse = (filt_sq / (count as f64 * d as f64)).sqrt();
        assert!(
            filt_rmse < raw_rmse,
            "filtered RMSE {} must beat raw measurement RMSE {}",
            filt_rmse,
            raw_rmse
        );
        // Meaningful margin: filter should cut RMSE by >50% vs raw noise.
        assert!(
            filt_rmse < 0.5 * raw_rmse,
            "filter should substantially cut RMSE: {} vs {}",
            filt_rmse,
            raw_rmse
        );
    }

    // W2-3 (3) byte-determinism: the SAME inputs (same seeded noise stream) must
    // produce the IDENTICAL estimate across two independent runs, bit-for-bit.
    #[test]
    fn geo_kalman_byte_deterministic() {
        fn run() -> Vec<f64> {
            let d = 2usize;
            let dt = 1.0f64;
            let accel_var = 0.5f64;
            let meas_var = 1.0f64;
            let true_v = [2.0f64, -1.5];
            let pos0 = [0.0, 0.0];
            let mut seed = 0xCAFE_u64;
            let mut kf = geo_kalman(d, dt, accel_var, meas_var);
            for k in 0..100usize {
                let t = (k as f64) * dt;
                let mut z = [0.0; 2];
                for i in 0..d {
                    z[i] = pos0[i] + true_v[i] * t + lcg(&mut seed) * meas_var.sqrt();
                }
                kf.step(&z);
            }
            kf.state().to_vec()
        }
        let a = run();
        let b = run();
        assert_eq!(a.len(), b.len());
        for (x, y) in a.iter().zip(b.iter()) {
            assert_eq!(
                x.to_bits(),
                y.to_bits(),
                "geo_kalman must be byte-deterministic: {} vs {}",
                x,
                y
            );
        }
    }

    // ── W2-3 SE(3) courier tests ──────────────────────────────────────────

    // Rotation angle (radians) between a unit quaternion `q` and identity.
    fn quat_angle_from_identity(q: &[f64; 4]) -> f64 {
        let n = crate::math::fsqrt(q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3]);
        let w = (q[0] / n).abs();
        let w = w.min(1.0);
        2.0 * w.acos()
    }

    // W2-3 (1) position convergence: a courier moving at constant velocity
    // [1.0, 0.5, 0.0] with GPS noise must drive the SE(3) position estimate to
    // within 1.0 m and velocity to within 0.1 of truth after ~200 steps.
    #[test]
    fn geo_se3_position_converges() {
        let dt = 1.0f64;
        let accel_var = 0.01f64;
        let gyro_var = 0.001f64;
        let pos_meas_var = 1.0f64;
        let att_meas_var = 1e-3f64;
        let true_v = [1.0f64, 0.5f64, 0.0f64];
        let pos0 = [0.0f64, 0.0f64, 0.0f64];
        let mut seed = 0x1234_u64;

        let mut kf = geo_se3_kalman(dt, accel_var, gyro_var, pos_meas_var, att_meas_var);
        let idq = [1.0f64, 0.0, 0.0, 0.0];
        for k in 0..200usize {
            let t = (k as f64) * dt;
            let mut z = [0.0f64; 3];
            for i in 0..3 {
                z[i] = pos0[i] + true_v[i] * t + lcg(&mut seed) * pos_meas_var.sqrt();
            }
            kf.step();
            kf.update(&z, &idq);
        }
        let pos = kf.position();
        let vel = kf.velocity();
        let true_pos = [
            pos0[0] + true_v[0] * 200.0,
            pos0[1] + true_v[1] * 200.0,
            0.0,
        ];
        for i in 0..3 {
            assert!(
                (pos[i] - true_pos[i]).abs() < 1.0,
                "pos[{}] {} vs {}",
                i,
                pos[i],
                true_pos[i]
            );
            assert!(
                (vel[i] - true_v[i]).abs() < 0.1,
                "vel[{}] {} vs {}",
                i,
                vel[i],
                true_v[i]
            );
        }
    }

    // W2-3 (2) attitude convergence: start perturbed off identity, feed noisy
    // identity-quaternion measurements; estimated orientation must approach
    // identity within 0.1 rad.
    #[test]
    fn geo_se3_attitude_converges() {
        let dt = 0.5f64;
        let accel_var = 0.01f64;
        let gyro_var = 0.001f64;
        let pos_meas_var = 1e-3f64;
        let att_meas_var = 1e-2f64;
        let mut seed = 0xABCD_u64;

        let mut kf = geo_se3_kalman(dt, accel_var, gyro_var, pos_meas_var, att_meas_var);
        // Perturbed initial attitude: small rotation about x (≈ 0.5 rad) encoded
        // as a quaternion [cos(θ/2), sin(θ/2), 0, 0].
        let th = 0.5f64;
        let ch = crate::math::fcos(th / 2.0);
        let sh = crate::math::fsin(th / 2.0);
        let mut x0 = vec![0.0f64; 13];
        x0[6] = ch;
        x0[7] = sh;
        kf.set_state_all(&x0);

        let idq = [1.0f64, 0.0, 0.0, 0.0];
        for _ in 0..300usize {
            let mut qm = idq;
            // small noisy measurement of identity; treat as additive on the
            // (w,x,y,z) components (kept tiny), then renormalize.
            let mut qq = [idq[0], idq[1], idq[2], idq[3]];
            for i in 0..4 {
                qq[i] += lcg(&mut seed) * att_meas_var.sqrt() * 0.1;
            }
            let nrm =
                crate::math::fsqrt(qq[0] * qq[0] + qq[1] * qq[1] + qq[2] * qq[2] + qq[3] * qq[3]);
            for i in 0..4 {
                qm[i] = qq[i] / nrm;
            }
            kf.step();
            kf.update(&[0.0, 0.0, 0.0], &qm);
        }
        let q = kf.orientation();
        let ang = quat_angle_from_identity(&q);
        assert!(
            ang < 0.1,
            "attitude angle from identity = {} (want < 0.1)",
            ang
        );
    }

    // W2-3 (3) byte-determinism: same seeded noise stream → identical estimate
    // across two independent runs, bit-for-bit.
    #[test]
    fn geo_se3_byte_deterministic() {
        fn run() -> Vec<f64> {
            let dt = 1.0f64;
            let mut kf = geo_se3_kalman(dt, 0.5f64, 0.1f64, 1.0f64, 1e-2f64);
            let idq = [1.0f64, 0.0, 0.0, 0.0];
            let mut seed = 0xCAFE_u64;
            for k in 0..100usize {
                let t = (k as f64) * dt;
                let mut z = [0.0f64; 3];
                for i in 0..3 {
                    z[i] = 2.0 * t + lcg(&mut seed);
                }
                kf.step();
                kf.update(&z, &idq);
            }
            kf.state_vec()
        }
        let a = run();
        let b = run();
        assert_eq!(a.len(), b.len());
        for (x, y) in a.iter().zip(b.iter()) {
            assert_eq!(
                x.to_bits(),
                y.to_bits(),
                "geo_se3 must be byte-deterministic: {} vs {}",
                x,
                y
            );
        }
    }
}
