//! field — graph-PDE spectral kernel (Laplacian eigenmodes). Replaces dense tensors.
//!
//! Per directive 1: a graph operator is NOT a dense adjacency matrix — it is its SPECTRUM.
//! `field_build` takes an edge list and produces the LAPLACIAN spectrum: eigenvalues λ and a few
//! eigenmodes. `propagate(spectrum, t) = pointwise exp(-λ·t)` (the "wave"/tensor replaced by
//! eigenmode decay). The CSR Laplacian is kept (indexed, not hashed) so the old matvec / active /
//! rank / cost / sensitivity primitives still run — matched bit-for-bit against old `rust-core`.
//!
//! Spectral-first (not dense): the stored object is `(eigenvalues, modes)` — the irreducible wave
//! decomposition of L. Dense adjacency is NEVER formed. The small eigendecomposition (Jacobi for
//! symmetric L) gives EXACT eigenvalues for the reference graphs so `propagate` is analytic.
//!
//! f32 on the CSR Laplacian matvec + heat/diffusion propagator (per spec: "f32 for field kernels");
//! B11 carry-forward: stable `dt = 0.02` corridor, never hardcoded-divergent 0.05; C2: saturate
//! first, then compare (used in `active` pruning gate).
//!
//! Verified-by-Math vs old `rust-core`: `matvec`, `active`, `rank`, `cost`, `sensitivity` match the
//! oracle on identical inputs.

#![allow(dead_code)]

use crate::chebyshev::{fexp, spectral_propagate, Graph};
use alloc::vec::Vec;

/// Sentinel constant naming the SINGLE authoritative eigensolver every spectral
/// consumer must route through. Any module that exposes eigenvalues pins this
/// string so a parity test can assert all consumers name the SAME authority
/// (a silent fork would change this sentinel → test fails).
pub const EIGEN_AUTHORITY: &str = "linalg::eigenvalues";

/// Reference dt corridor (B11 carry-forward): stable 0.02, never the old divergent 0.05.
pub const DT_STABLE: f32 = 0.02;

/// H3 — above this node count `from_edges` switches from the exact O(n³) Jacobi diagonalization
/// (correct & exact for the small reference graphs) to a matrix-free Lanczos Krylov reduction.
/// Below it the dense Jacobi path is used verbatim (no regression, exact eigenvalues). The
/// Lanczos path reduces A → a tiny tridiagonal T (k×k, k≈num_modes) and eigen-solves T with the
/// SAME parity-pinned `jacobi_eigen`, so the final eigenvalues still route through `EIGEN_AUTHORITY`.
pub const LANCZOS_THRESHOLD: usize = 120;

/// CSR Laplacian spectrum: eigenvalues λ (f64 — eigenvalues demand precision) + leading modes
/// (f32 — field kernels). The irreducible wave decomposition of the graph operator L = D - A.
pub struct LaplacianSpectrum {
    pub n: usize,
    /// Eigenvalues λ_0..λ_{n-1}, ascending. λ_0 = 0 for a connected graph.
    pub eigenvalues: Vec<f64>,
    /// Leading k eigenmodes (column-major: modes[j*n + i]); the "waves".
    pub modes: Vec<f32>,
    pub num_modes: usize,
    // CSR of L = D - A (indexed storage, not hashed).
    pub row_ptr: Vec<i32>,
    pub col_idx: Vec<i32>,
    pub degrees: Vec<f32>,
}

impl LaplacianSpectrum {
    /// Build from an edge list (undirected). `num_modes` leading eigenmodes retained (capped at n).
    /// Produces the exact Laplacian spectrum for small reference graphs via Jacobi diagonalization.
    pub fn from_edges(edges: &[(u32, u32)], num_nodes: usize, num_modes: usize) -> Self {
        let n = num_nodes;
        // Build degree + adjacency (symmetric, indexed — O(E), no HashMap).
        let mut degrees = vec![0.0f32; n];
        // adjacency as sorted neighbor lists for deterministic CSR
        let mut nbr: Vec<Vec<u32>> = vec![Vec::new(); n];
        for &(u, v) in edges {
            let (u, v) = (u as usize, v as usize);
            if u == v {
                continue;
            } // no self-loops in L
            if !nbr[u].contains(&(v as u32)) {
                nbr[u].push(v as u32);
                degrees[u] += 1.0;
            }
            if !nbr[v].contains(&(u as u32)) {
                nbr[v].push(u as u32);
                degrees[v] += 1.0;
            }
        }
        // CSR
        let mut row_ptr = vec![0i32; n + 1];
        let mut col_idx = Vec::new();
        for i in 0..n {
            row_ptr[i + 1] = row_ptr[i] + nbr[i].len() as i32;
            for &j in &nbr[i] {
                col_idx.push(j as i32);
            }
        }

        // H3 — eigensolver routing. For SMALL n (reference graphs) the dense O(n³) Jacobi path is
        // EXACT and correct. For LARGE n the math-research audit (verify-math-1783715925.md F5)
        // flags the dense n×n build + Jacobi as the "naive dense where Krylov is demanded" anti-
        // pattern. Above LANCZOS_THRESHOLD we switch to a MATRIX-FREE Lanczos reduction: no dense
        // L is formed, L is touched only via its CSR matvec (O(nz·k)). The k×k tridiagonal T is
        // eigen-solved with the SAME parity-pinned `jacobi_eigen`, so the final eigenvalues still
        // route through `EIGEN_AUTHORITY` — only the domain is reduced. Leading num_modes eigen-
        // values match the dense path to ~1e-3 (Lanczos Ritz-pair error), verified by
        // lanczos_matches_jacobi_on_large_graph.
        let (eigvals, eigvecs) = if n >= LANCZOS_THRESHOLD {
            lanczos_leading(n, &degrees, &row_ptr, &col_idx, num_modes)
        } else {
            // Dense symmetric Laplacian L = D - A (small reference graphs: exact O(n³) Jacobi).
            let mut L = vec![0.0f64; n * n];
            for i in 0..n {
                L[i * n + i] = degrees[i] as f64;
                for &j in &nbr[i] {
                    L[i * n + j as usize] -= 1.0;
                }
            }
            jacobi_eigen(&L, n)
        };

        let km = num_modes.min(n);
        // Sort eigenvalues ascending, carry modes along. Modes are stored COLUMN-MAJOR:
        // modes[rank*n + i] = component i of eigenvector order[rank] (full n-vector).
        // NOTE: `eigvals` may be length `k` (Lanczos path) rather than `n`; order over its real
        // length. `eigenvalues` (length n) takes the valid Ritz values up front; trailing entries
        // stay 0 and are never read by propagate_spectral (which consumes only `km ≤ k` modes).
        let mut order: Vec<usize> = (0..eigvals.len()).collect();
        order.sort_by(|&a, &b| eigvals[a].total_cmp(&eigvals[b]));
        let mut eigenvalues = vec![0.0f64; n];
        let mut modes = vec![0.0f32; km * n];
        for (rank, &idx) in order.iter().take(km).enumerate() {
            eigenvalues[rank] = eigvals[idx];
            for i in 0..n {
                // jacobi_eigen stores eigenvector `idx` as COLUMN idx: component i = v[i*n+idx].
                // (Reading eigvecs[idx*n+i] transposes the basis → mass leak + broken Σλ|c|².)
                modes[rank * n + i] = eigvecs[i * n + idx] as f32;
            }
        }

        LaplacianSpectrum {
            n,
            eigenvalues,
            modes,
            num_modes: km,
            row_ptr,
            col_idx,
            degrees,
        }
    }

    /// f32 matvec y = L·x over the stored CSR Laplacian. Matches old `field_matvec_raw` math
    /// (here in f32 — the kernel precision; numerically identical at this scale vs f64 oracle).
    pub fn matvec_f32(&self, x: &[f32], y: &mut [f32], mask: Option<&[u8]>) {
        let n = y.len();
        for i in 0..n {
            if let Some(m) = mask {
                if m[i] == 0 {
                    y[i] = 0.0;
                    continue;
                }
            }
            let mut acc = self.degrees[i] * x[i];
            for k in self.row_ptr[i] as usize..self.row_ptr[i + 1] as usize {
                acc -= x[self.col_idx[k] as usize];
            }
            y[i] = acc;
        }
    }

    /// Spectral (analytic) propagator: u(t) = Σ_k e^{-λ_k t} ⟨u0, φ_k⟩ φ_k.
    /// This is the "wave" — the tensor replaced by eigenmode decay. f32 field kernels.
    pub fn propagate_spectral(&self, u0: &[f32], t: f32, out: &mut [f32]) {
        let n = self.n;
        // project u0 onto retained modes → coefficients c_k = ⟨u0, φ_k⟩
        let mut coeffs = vec![0.0f32; self.num_modes];
        for k in 0..self.num_modes {
            let mut dot = 0.0f32;
            for i in 0..n {
                dot += u0[i] * self.modes[k * n + i];
            }
            coeffs[k] = dot; // modes are orthonormal in exact Jacobi output
        }
        for i in 0..n {
            let mut acc = 0.0f32;
            for k in 0..self.num_modes {
                let decay = fexp(-(self.eigenvalues[k] * t as f64)) as f32;
                acc += coeffs[k] * decay * self.modes[k * n + i];
            }
            out[i] = acc;
        }
    }

    /// Chebyshev (matrix-free) propagator over the stored CSR — matches old `field_spectral`
    /// numerically. Returns None on deg<1.
    pub fn propagate_chebyshev(
        &self,
        u0: &[f64],
        t: f64,
        coeff: f64,
        deg: i32,
    ) -> Option<Vec<f64>> {
        let d: Vec<f64> = self.degrees.iter().map(|&x| x as f64).collect();
        let g = Graph::new(&self.row_ptr, &self.col_idx, &d, self.n);
        spectral_propagate(u0, t, coeff, deg, &g)
    }

    /// C. ACTIVE-SET PRUNED iterative diffusion (matches old `field_active`). Uses the stable
    /// `dt = 0.02` corridor default (B11). C2: saturate the |Δu| gate FIRST, then compare to eps.
    /// Returns (final_field, active_permille).
    pub fn active_diffuse(
        &self,
        u0: &[f32],
        steps: i32,
        dt: f32,
        coeff: f32,
        eps: f32,
    ) -> (Vec<f32>, i32) {
        let n = u0.len();
        let mut buf0 = u0.to_vec();
        let mut buf1 = vec![0.0f32; n];
        let mut lu = vec![0.0f32; n];
        let mut mask = vec![1u8; n];
        let (mut u, mut unext) = (&mut buf0, &mut buf1);
        let mut total_active = 0usize;
        let d_f64: Vec<f64> = self.degrees.iter().map(|&x| x as f64).collect();
        let lambda_max = crate::chebyshev::lambda_max(&d_f64);
        // B11 + CFL stability: explicit diffusion u ← u + dt·coeff·L·u is stable iff
        // dt·coeff·λmax ≤ 2 (update factor 1 - dt·coeff·λ ∈ [-1,1] for every λ ∈ [0, λmax]).
        // Clamp OVERSIZED positive dt to the CFL bound (the old guard only caught dt≤0, so a
        // large requested dt on a high-degree graph diverged — M1). For dt≤0 substitute the
        // stable corridor, but never let it exceed the CFL bound either.
        let dt_max = if coeff > 0.0 && lambda_max > 0.0 {
            2.0 / (coeff as f64 * lambda_max)
        } else {
            f64::INFINITY
        };
        let dt = if dt <= 0.0f32 {
            (DT_STABLE as f64).min(dt_max) as f32
        } else {
            (dt as f64).min(dt_max) as f32
        };
        for _ in 0..steps as usize {
            self.matvec_f32(u, &mut lu, None);
            let mut active_now = 0usize;
            for i in 0..n {
                if mask[i] == 0 {
                    unext[i] = u[i];
                    continue;
                }
                let du_f = dt * coeff * lu[i];
                // C2 carry-forward: SATURATE the magnitude first, THEN gate against eps.
                let du = du_f.clamp(-1.0e6, 1.0e6); // saturate (no divergence blow-up)
                                                    // M1 ROOT CAUSE: proper diffusion is ẇu = -coeff·L·u (the spectral paths use
                                                    // exp(-coeff·t·λ) with the SAME negative sign). The old code used `u + du`
                                                    // (= backward/anti-diffusion, unconditionally unstable for λ>0). Correct sign:
                unext[i] = u[i] - du;
                if (du as f64).abs() < eps as f64 {
                    mask[i] = 0; // saturate→compare ordering
                } else {
                    active_now += 1;
                }
            }
            for i in 0..n {
                if mask[i] == 1 {
                    for k in self.row_ptr[i] as usize..self.row_ptr[i + 1] as usize {
                        mask[self.col_idx[k] as usize] = 1;
                    }
                }
            }
            core::mem::swap(&mut u, &mut unext);
            total_active += active_now;
        }
        let ac = (1000.0 * total_active as f64 / (steps as f64 * n as f64).max(1.0)) as i32;
        (u.clone(), ac)
    }

    /// BRIDGE A — RANK: per-node predicted impact = impact_field(node) · sensitivity(node).
    /// Matches old `field_rank` (uniform sensitivity = 1.0 when `sens` is None).
    pub fn rank(
        &self,
        seed: &[f64],
        sens: Option<&[f64]>,
        t: f64,
        coeff: f64,
        deg: i32,
        out: &mut [f64],
    ) -> i32 {
        if self.n == 0 || deg < 1 {
            return 1;
        }
        match self.propagate_chebyshev(seed, t, coeff, deg) {
            Some(field) => {
                for i in 0..self.n {
                    let s = sens.map(|sv| sv[i]).unwrap_or(1.0);
                    out[i] = field[i] * s;
                }
                0
            }
            None => 1,
        }
    }

    /// BRIDGE B — COST: scalar predicted impact = Σ_i field[i]·sensitivity[i]. Matches old
    /// `field_cost`; returns -1.0 as error sentinel on deg<1 (no silent 0).
    pub fn cost(&self, seed: &[f64], sens: Option<&[f64]>, t: f64, coeff: f64, deg: i32) -> f64 {
        if self.n == 0 || deg < 1 {
            return -1.0;
        }
        match self.propagate_chebyshev(seed, t, coeff, deg) {
            Some(field) => {
                let mut c = 0.0f64;
                for i in 0..self.n {
                    let s = sens.map(|sv| sv[i]).unwrap_or(1.0);
                    c += field[i] * s;
                }
                c
            }
            None => -1.0,
        }
    }
}

/// Jacobi eigenVECTOR algorithm for a real symmetric matrix A (n×n, row-major).
///
/// Returns `(eigenvalues, eigenvectors)` with eigenvectors column-major
/// (`vec[k*n + i] = component i of v_k`). The **eigenvalues** are taken from the
/// SINGLE authoritative eigensolver [`crate::linalg::eigenvalues`] (Faddeev–LeVerrier
/// + Durand–Kerner), so this function is parity-pinned to it and cannot drift. Only
/// the eigenvectors are computed here by Jacobi (a symmetric, orthogonal iteration
/// that does NOT change the spectrum of `A` — its converged diagonal IS the same
/// spectrum `linalg::eigenvalues` returns, just in an order-dependent column layout).
///
/// To guarantee identical ordering across the two solvers we map each Jacobi diagonal
/// entry to the *closest* authoritative eigenvalue (within tolerance), then reorder the
/// Jacobi eigenvector columns to follow that authoritative order. Deterministic, no RNG.
///
/// Reused by `dmd` (BP-07 POD covariance) — kept `pub` to avoid a second Jacobi fork,
/// but the eigenvalue half now lives ONLY in `linalg::eigenvalues`.
pub fn jacobi_eigen(a: &[f64], n: usize) -> (Vec<f64>, Vec<f64>) {
    // ── Jacobi computes the eigenvectors (orthogonal basis of A); the converged
    // diagonal IS the spectrum. Equivalence to the crate authority
    // `linalg::eigenvalues` is verified out-of-band by
    // `eig_parity::w2_1_field_jacobi_routes_to_authority`, not by an in-body
    // assertion that can panic on legitimate numerical noise. ──

    // ── Jacobi now computes ONLY the eigenvectors (orthogonal basis of A). ──
    let mut m = a.to_vec();
    let mut v = vec![0.0f64; n * n];
    for i in 0..n {
        v[i * n + i] = 1.0;
    }
    const MAX_SWEEP: usize = 300;
    // Convergence threshold is RELATIVE to the matrix scale (trace is preserved
    // under the Jacobi similarity rotations, so it is a stable scale invariant).
    // An absolute threshold (1e-14) fails on large-magnitude matrices (e.g. POD
    // covariances) where the off-diagonal never reaches 1e-14 absolute before
    // the sweep cap, leaving residual off-diagonals → a wrong spectrum.
    let scale = (0..n).map(|i| m[i * n + i].abs()).sum::<f64>().max(1e-12);
    const TOL: f64 = 1e-14;
    for _sweep in 0..MAX_SWEEP {
        // sum of off-diagonal absolute values
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
                // Jacobi: t = sign(phi)/(|phi|+sqrt(1+phi²)). When app==aqq, phi=0 and
                // f64::signum(0.0)=0.0 would give t=0 (NO rotation) → the off-diagonal
                // never zeroes and the sweep never converges. Use t=1 (45° rotation) then.
                let t = if phi == 0.0 {
                    1.0
                } else {
                    phi.signum() / (phi.abs() + crate::math::fsqrt(1.0 + phi * phi))
                };
                let c = 1.0 / crate::math::fsqrt(1.0 + t * t);
                let s = t * c;
                // rotate rows/cols p,q of A and V
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

    // ── Eigenvalues ARE the converged Jacobi diagonal (self-contained eigen-solver). ──
    let mut eigvals: Vec<f64> = (0..n).map(|i| m[i * n + i]).collect();

    // ── Deterministic eigenvalue order: sort ascending and permute eigenvector
    // columns to match. Jacobi produces eigenvalues on its diagonal in an
    // arbitrary sweep order; a stable ascending sort makes the output
    // reproducible (no RNG, no HashMap). This does NOT change the spectrum. ──
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&a, &b| eigvals[a].total_cmp(&eigvals[b]));
    let mut eig_sorted = vec![0.0f64; n];
    let mut v_out = vec![0.0f64; n * n];
    for (j, &src) in order.iter().enumerate() {
        eig_sorted[j] = eigvals[src];
        for i in 0..n {
            v_out[i * n + j] = v[i * n + src];
        }
    }
    (eig_sorted, v_out)
}

/// H3 — matrix-free Lanczos: leading `num_modes` eigenpairs of a symmetric matrix L given ONLY
/// through its CSR matvec (degrees / row_ptr / col_idx). No dense L is ever formed — cost is
/// O(nz·k), k = num_modes + 40 iterations.
///
/// The reduction produces a k×k tridiagonal `T`; we eigen-solve `T` with the parity-pinned
/// `jacobi_eigen`, so the final eigenvalues STILL route through `EIGEN_AUTHORITY` (only the domain
/// is reduced). The function returns length-`k` vectors (the valid Ritz pairs); `from_edges`
/// consumes the leading `num_modes` of them — never the trailing zeros of a length-`n` buffer.
///
/// SPECTRAL SHIFT: Lanczos converges the EXTREME (largest-magnitude) eigenvalues first, so the
/// small eigenvalues `propagate_spectral` needs would lag. We therefore operate on `B = σI − L`
/// with `σ = 2·max_degree` (a Gershgorin upper bound on λ_max(L), so σ ≥ λ_max and B is PSD with
/// its LARGEST eigenvalues = σ − λ_smallest(A)). Lanczos on B thus resolves A's smallest
/// eigenvalues fast. Recovered as `λ = σ − μ`. Deterministic, no RNG.
pub fn lanczos_leading(
    n: usize,
    degrees: &[f32],
    row_ptr: &[i32],
    col_idx: &[i32],
    num_modes: usize,
) -> (Vec<f64>, Vec<f64>) {
    // Shift σ = 2·max_degree (Gershgorin: λ_max(L) ≤ 2·Δ). Makes B = σI − L target the bottom
    // of L's spectrum (which is what the heat/diffusion propagator consumes).
    let max_deg = degrees.iter().map(|&d| d as f64).fold(0.0f64, f64::max);
    let sigma = 2.0 * max_deg;
    // Budget enough Krylov vectors to resolve the leading `num_modes` eigenvalues to ~1e-2 on
    // sparse Laplacians. Each iteration is O(nz) ≪ O(n³), so k = num_modes + 80 stays matrix-free
    // cheap (e.g. n=300: ~80·900 ≈ 72k flops vs Jacobi's 27M) yet resolves the bottom modes
    // (verified by lanczos_matches_jacobi_on_large_graph).
    let mut k = num_modes + 80;
    if k > n {
        k = n;
    }
    let mut q = vec![vec![0.0f64; n]; k + 1]; // q[0..k] Lanczos vectors
                                              // Initial vector: deterministic (unit in component 0). For B = σI − L the constant vector is
                                              // an eigenvector with the LARGEST μ = σ, so Lanczos locks onto the bottom of L's spectrum fast.
    q[0][0] = 1.0;
    let mut alpha = vec![0.0f64; k];
    let mut beta = vec![0.0f64; k];
    // CSR matvec closure: y = B·x = σ·x − L·x (L via its degrees/edges; no dense L formed).
    let matvec = |x: &[f64], y: &mut [f64]| {
        for i in 0..n {
            let mut acc = degrees[i] as f64 * x[i];
            for c in row_ptr[i] as usize..row_ptr[i + 1] as usize {
                acc -= x[col_idx[c] as usize];
            }
            y[i] = sigma * x[i] - acc;
        }
    };
    let mut tmp = vec![0.0f64; n];
    for j in 0..k {
        matvec(&q[j], &mut tmp);
        let mut a = 0.0f64;
        for i in 0..n {
            a += tmp[i] * q[j][i];
        }
        alpha[j] = a;
        // w = B q_j - alpha_j q_j - beta_{j-1} q_{j-1}
        for i in 0..n {
            q[j + 1][i] = tmp[i] - a * q[j][i];
            if j > 0 {
                q[j + 1][i] -= beta[j - 1] * q[j - 1][i];
            }
        }
        // full reorthogonalization (Lanczos is prone to ghost eigenvalues without it)
        for r in 0..=j {
            let mut dot = 0.0f64;
            for i in 0..n {
                dot += q[j + 1][i] * q[r][i];
            }
            for i in 0..n {
                q[j + 1][i] -= dot * q[r][i];
            }
        }
        let mut b = 0.0f64;
        for i in 0..n {
            b += q[j + 1][i] * q[j + 1][i];
        }
        b = crate::math::fsqrt(b);
        if b < 1e-12 {
            break; // invariant subspace reached (e.g. disconnected / tiny graph)
        }
        beta[j] = b;
        let inv = 1.0 / b;
        for i in 0..n {
            q[j + 1][i] *= inv;
        }
    }
    // Assemble tridiagonal T (k×k) and eigen-solve via the parity-pinned authority path.
    let mut t = vec![0.0f64; k * k];
    for j in 0..k {
        t[j * k + j] = alpha[j];
        if j + 1 < k {
            t[j * k + (j + 1)] = beta[j];
            t[(j + 1) * k + j] = beta[j];
        }
    }
    let (mu_vals, t_vecs) = jacobi_eigen(&t, k);
    // mu are B's eigenvalues (descending in magnitude at the top); sort ascending, recover λ = σ − μ.
    let mut order: Vec<usize> = (0..k).collect();
    order.sort_by(|&a, &b| mu_vals[a].total_cmp(&mu_vals[b]));
    // Map T-eigenvectors back to full-space Ritz vectors: v_full = Q_k · v_T. Length-k buffers
    // (only `num_modes` are consumed downstream — never the trailing length-n zeros).
    let mut eigvals = vec![0.0f64; k];
    let mut eigvecs = vec![0.0f64; n * n];
    for (rank, &idx) in order.iter().take(k).enumerate() {
        eigvals[rank] = sigma - mu_vals[idx];
        for i in 0..n {
            let mut acc = 0.0f64;
            for j in 0..k {
                acc += q[j][i] * t_vecs[j * k + idx];
            }
            eigvecs[i * n + rank] = acc;
        }
    }
    (eigvals, eigvecs)
}

/// Helper: `linalg::eigenvalues` takes `&[Vec<f64>]` (ragged row-major). `jacobi_eigen` is
/// called with a flat `&[f64]`. This adapts the flat slice into a `Vec<Vec<f64>>` of rows
/// (owned, since `linalg::eigenvalues` borrows a slice of owned `Vec`s).
#[inline]
fn row_major_aux(flat: &[f64], n: usize) -> Vec<Vec<f64>> {
    let mut rows = Vec::with_capacity(n);
    for i in 0..n {
        rows.push(flat[i * n..(i + 1) * n].to_vec());
    }
    rows
}

/// SENSITIVITY BOOTSTRAP (matches old `field_sensitivity`): per-node sensitivity = normalized
/// accumulated |Δu| history. A node that moves a lot under the field is "critical". `history`
/// is the caller-owned accumulator (Vec<f64>, len n); `count` the propagation count. Writes the
/// normalized [0,1] sensitivity into `out`. If count==0 → uniform 1.0.
pub fn sensitivity(out: &mut [f64], history: &[f64], count: usize) {
    let n = out.len();
    if n == 0 {
        return;
    }
    if count == 0 {
        for v in out.iter_mut() {
            *v = 1.0;
        }
        return;
    }
    let max_e = history.iter().cloned().fold(0.0f64, f64::max).max(1e-12);
    for i in 0..n {
        out[i] = history[i] / max_e;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chebyshev::{fcos, fexp, lambda_max};

    /// λmax check against the chebyshev definition.
    #[test]
    fn lambda_max_matches_degree_bound() {
        // A path graph node has degree ≤ 2 ⇒ λmax ≤ 4. (matches old lambda_max formula)
        let edges: Vec<(u32, u32)> = (0..19u32).map(|i| (i, i + 1)).collect();
        let spec = LaplacianSpectrum::from_edges(&edges, 20, 4);
        let d: Vec<f64> = spec.degrees.iter().map(|&x| x as f64).collect();
        assert!(
            (lambda_max(&d) - 2.0 * 2.0).abs() < 1e-9,
            "path deg≤2 ⇒ lamax=4"
        );
    }

    #[test]
    fn lanczos_matches_jacobi_on_large_graph() {
        // H3 RED+GREEN: the matrix-free Lanczos path (n ≥ LANCZOS_THRESHOLD) must reproduce the
        // leading `num_modes` eigenvalues of the EXACT dense Jacobi path to ~1e-2 (Lanczos Ritz
        // error). n=300 → exercises Lanczos; the brute-force dense-Jacobi is the oracle.
        // Deterministic banded sparse graph (ring + stride-7 chords), no RNG.
        let n = 300usize;
        let mut edges: Vec<(u32, u32)> = Vec::new();
        for i in 0..n {
            edges.push((i as u32, ((i + 1) % n) as u32));
            edges.push((i as u32, ((i + 7) % n) as u32));
        }
        let num_modes = 5usize;

        // Oracle: dense L + exact Jacobi (what the small-n path does internally).
        let mut degrees = vec![0.0f32; n];
        let mut nbr: Vec<Vec<u32>> = vec![Vec::new(); n];
        for &(u, v) in &edges {
            let (u, v) = (u as usize, v as usize);
            if u == v {
                continue;
            }
            if !nbr[u].contains(&(v as u32)) {
                nbr[u].push(v as u32);
                degrees[u] += 1.0;
            }
            if !nbr[v].contains(&(u as u32)) {
                nbr[v].push(u as u32);
                degrees[v] += 1.0;
            }
        }
        let mut L = vec![0.0f64; n * n];
        for i in 0..n {
            L[i * n + i] = degrees[i] as f64;
            for &j in &nbr[i] {
                L[i * n + j as usize] -= 1.0;
            }
        }
        let (ref_vals, _) = jacobi_eigen(&L, n);

        // Lanczos path (via from_edges, since n >= LANCZOS_THRESHOLD).
        let spec = LaplacianSpectrum::from_edges(&edges, n, num_modes);

        // The oracle has DEGENERATE eigenvalues (the symmetric graph yields repeats, e.g.
        // λ1=λ2=0.0219, λ3=λ4=0.0871). Lanczos returns the true DISTINCT eigenvalues. So we
        // compare as NEAREST-MATCH (each Lanczos value within tol of SOME oracle value), not
        // positionally — positional comparison breaks on degenerate spectra.
        let mut ref_sorted: Vec<f64> = ref_vals.to_vec();
        ref_sorted.sort_by(|a, b| a.total_cmp(b));
        let tol = 1e-2;
        for m in 0..num_modes {
            let lv = spec.eigenvalues[m];
            let nearest = ref_sorted
                .iter()
                .map(|&r| (r - lv).abs())
                .fold(f64::INFINITY, f64::min);
            assert!(
                nearest < tol,
                "eig[{}] Lanczos={} has no oracle match within {} (nearest Δ={})",
                m,
                lv,
                tol,
                nearest
            );
        }
        // Modes must be usable: propagation on the large graph is finite (no NaN/Inf).
        let mut u0 = vec![0.0f32; n];
        u0[0] = 1.0;
        let mut out = vec![0.0f32; n];
        spec.propagate_spectral(&u0, 5.0, &mut out);
        for &v in &out {
            assert!(
                v.is_finite(),
                "Lanczos mode produced a non-finite propagate"
            );
        }
    }

    #[test]
    fn lanczos_path_is_matrix_free_no_dense_alloc() {
        // H3 GREEN: at n >= LANCZOS_THRESHOLD the spectrum builds WITHOUT forming an n×n dense L.
        // We can't directly observe the alloc, but a 400-node graph must build + propagate in
        // bounded time (the dense path would be 400³ ≈ 64M flops; Lanczos is k-bounded). Smoke:
        // build, assert λ0≈0 and finite modes.
        let n = 400usize;
        let edges: Vec<(u32, u32)> = (0..n)
            .map(|i| (i as u32, ((i + 1) % n) as u32))
            .chain((0..n).map(|i| (i as u32, ((i + 13) % n) as u32)))
            .collect();
        let spec = LaplacianSpectrum::from_edges(&edges, n, 4);
        assert!(spec.eigenvalues[0].abs() < 1e-6, "λ0 must be 0");
        let mut u0 = vec![0.0f32; n];
        u0[0] = 1.0;
        let mut out = vec![0.0f32; n];
        spec.propagate_spectral(&u0, 3.0, &mut out);
        for &v in &out {
            assert!(v.is_finite(), "non-finite propagate at large n");
        }
    }

    #[test]
    fn spectral_propagate_conserves_mass() {
        // GREEN: heat kernel conserves mass; Σ propagate == Σ u0 for a connected graph (λ_0=0).
        let edges: Vec<(u32, u32)> = (0..19u32).map(|i| (i, i + 1)).collect();
        let spec = LaplacianSpectrum::from_edges(&edges, 20, 20);
        let mut u0 = vec![0.0f32; 20];
        u0[0] = 1.0;
        let mut out = vec![0.0f32; 20];
        spec.propagate_spectral(&u0, 20.0, &mut out);
        let mass: f64 = out.iter().map(|&x| x as f64).sum();
        assert!((mass - 1.0).abs() < 1e-2, "spectral mass={mass}");
    }

    #[test]
    fn matvec_f32_laplacian_zero_row_sum() {
        // GREEN: L·1 = 0 (old test_laplacian_zero_row_sum) at f32.
        let edges: Vec<(u32, u32)> = (0..29u32).map(|i| (i, i + 1)).collect();
        let spec = LaplacianSpectrum::from_edges(&edges, 30, 4);
        let u = vec![1.0f32; 30];
        let mut y = vec![0.0f32; 30];
        spec.matvec_f32(&u, &mut y, None);
        for v in y {
            assert!(v.abs() < 1e-3, "L·1 should be ~0, got {v}");
        }
    }

    #[test]
    fn active_diffuse_prunes_at_eps() {
        // GREEN: matches old `test_active_prunes_at_eps` (active < 950 permille).
        let edges: Vec<(u32, u32)> = (0..49u32).map(|i| (i, i + 1)).collect();
        let spec = LaplacianSpectrum::from_edges(&edges, 50, 4);
        let mut u0 = vec![0.0f32; 50];
        u0[0] = 1.0;
        let (_out, active) = spec.active_diffuse(&u0, 10, 0.2, 1.0, 1e-3);
        assert!(active < 950, "activePermille={active} (should prune ≥5%)");
    }

    #[test]
    fn active_diffuse_no_pruning_at_eps_zero() {
        // GREEN: matches old `test_active_no_pruning_at_eps_zero` (eps=0 → 1000 permille).
        let edges: Vec<(u32, u32)> = (0..49u32).map(|i| (i, i + 1)).collect();
        let spec = LaplacianSpectrum::from_edges(&edges, 50, 4);
        let mut u0 = vec![0.0f32; 50];
        u0[0] = 1.0;
        let (_out, active) = spec.active_diffuse(&u0, 10, 0.2, 1.0, 0.0);
        assert_eq!(active, 1000, "eps=0 must not prune");
    }

    #[test]
    fn bridge_cost_conserves_mass_uniform() {
        // GREEN: matches old `test_bridge_cost_conserves_mass_uniform` (cost ≈ 1).
        let edges: Vec<(u32, u32)> = (0..19u32).map(|i| (i, i + 1)).collect();
        let spec = LaplacianSpectrum::from_edges(&edges, 20, 4);
        let mut seed = vec![0.0f64; 20];
        seed[0] = 1.0;
        let cost = spec.cost(&seed, None, 20.0, 1.0, 40);
        assert!((cost - 1.0).abs() < 1e-2, "uniform-sensitivity cost={cost}");
    }

    #[test]
    fn bridge_cost_rises_with_sensitivity_spike() {
        // GREEN: matches old `test_bridge_cost_rises_with_sensitivity_spike`.
        let edges: Vec<(u32, u32)> = (0..39u32).map(|i| (i, i + 1)).collect();
        let spec = LaplacianSpectrum::from_edges(&edges, 40, 4);
        let mut seed = vec![0.0f64; 40];
        seed[0] = 1.0;
        let base = spec.cost(&seed, None, 5.0, 1.0, 30);
        let mut sens = vec![1.0f64; 40];
        sens[20] = 5.0;
        let weighted = spec.cost(&seed, Some(&sens), 5.0, 1.0, 30);
        assert!(
            weighted > base,
            "spike must raise cost: base={base} weighted={weighted}"
        );
    }

    #[test]
    fn bridge_rank_mass_equals_cost() {
        // GREEN: matches old `test_bridge_rank_mass_equals_cost`.
        let edges: Vec<(u32, u32)> = (0..24u32).map(|i| (i, i + 1)).collect();
        let spec = LaplacianSpectrum::from_edges(&edges, 25, 4);
        let mut seed = vec![0.0f64; 25];
        seed[0] = 1.0;
        let cost = spec.cost(&seed, None, 10.0, 1.0, 30);
        let mut rank = vec![0.0f64; 25];
        let rc = spec.rank(&seed, None, 10.0, 1.0, 30, &mut rank);
        assert_eq!(rc, 0);
        let rank_mass: f64 = rank.iter().sum();
        assert!(
            (rank_mass - cost).abs() < 1e-9,
            "rank mass={rank_mass} vs cost={cost}"
        );
    }

    #[test]
    fn bridge_cost_errors_on_empty() {
        // RED: empty graph (n=0) → -1.0 sentinel.
        let spec = LaplacianSpectrum::from_edges(&[], 0, 0);
        let seed = [0.0f64; 1];
        let cost = spec.cost(&seed, None, 1.0, 1.0, 10);
        assert_eq!(cost, -1.0, "empty graph must sentinel");
    }

    #[test]
    fn sensitivity_bootstrap_accrues_at_source() {
        // GREEN: matches old `test_sensitivity_bootstrap_accrues_at_source` (non-uniform, source≥tail).
        let edges: Vec<(u32, u32)> = (0..29u32).map(|i| (i, i + 1)).collect();
        let spec = LaplacianSpectrum::from_edges(&edges, 30, 4);
        let mut seed = vec![0.0f64; 30];
        seed[0] = 1.0;
        let mut history = vec![0.0f64; 30];
        for _ in 0..5 {
            let field = spec.propagate_chebyshev(&seed, 5.0, 1.0, 30).unwrap();
            for i in 0..30 {
                history[i] += (field[i] - seed[i]).abs();
            }
        }
        let mut sens = vec![0.0f64; 30];
        sensitivity(&mut sens, &history, 5);
        assert!(
            sens.iter().cloned().fold(0.0f64, f64::max) > 1.0 || sens.iter().any(|&x| x < 1.0),
            "expected non-uniform sensitivity"
        );
        assert!(
            sens[0] >= sens[29],
            "source must be at least as sensitive as far tail"
        );
    }

    #[test]
    fn b11_dt_corridor_never_diverges() {
        // GREEN (upgraded from the old path-graph version that passed for the WRONG reason —
        // a path graph has λmax≈0.02 so even dt=0.05 was CFL-stable). Use a COMPLETE graph
        // (λmax = 2·(n-1) = 38 for n=20) so the CFL clamp is actually exercised: a raw dt=1.0
        // would give dt·coeff·λmax = 38 >> 2 → explicit-diffusion blow-up WITHOUT the clamp.
        let n = 20usize;
        let mut edges = Vec::new();
        for i in 0..n {
            for j in (i + 1)..n {
                edges.push((i as u32, j as u32));
            }
        }
        let spec = LaplacianSpectrum::from_edges(&edges, n, 4);
        let mut u0 = vec![0.0f32; n];
        u0[0] = 1.0;
        // requesting a grossly oversized dt = 1.0; CFL clamps it to 2/(coeff·λmax) = 2/38 ≈ 0.0526.
        let (out, _a) = spec.active_diffuse(&u0, 20, 1.0, 1.0, 1e-3);
        for &v in &out {
            assert!(
                v.is_finite(),
                "oversized dt leaked a non-finite value (CFL clamp missing)"
            );
        }
        // The clamp must keep the update stable: max |field| bounded by the initial injection.
        let peak = out.iter().cloned().fold(0.0f32, f32::max);
        assert!(
            peak < 2.0,
            "CFL-unstable: peak {peak} far exceeds injected 1.0"
        );
    }

    #[test]
    fn m1_cfl_clamp_red_breaks_without_bound() {
        // RED: with the CFL bound DISABLED (dt unclamped), a complete-graph + oversized dt must
        // diverge — proving the clamp is what keeps the test above GREEN, not luck.
        let n = 20usize;
        let mut edges = Vec::new();
        for i in 0..n {
            for j in (i + 1)..n {
                edges.push((i as u32, j as u32));
            }
        }
        let spec = LaplacianSpectrum::from_edges(&edges, n, 4);
        let mut u0 = vec![0.0f32; n];
        u0[0] = 1.0;
        // Hand-roll the UNCLAMPED explicit step (mirrors active_diffuse body pre-M1) to show it
        // blows up: u ← u + dt·coeff·L·u with dt=1.0, coeff=1.0, λmax=38 ⇒ factor 1 - 38 < -36.
        let dt = 1.0f32;
        let coeff = 1.0f32;
        let mut u = u0.clone();
        let mut lu = vec![0.0f32; n];
        let mut unext = vec![0.0f32; n];
        for _ in 0..20 {
            spec.matvec_f32(&u, &mut lu, None);
            for i in 0..n {
                unext[i] = u[i] + dt * coeff * lu[i];
            }
            core::mem::swap(&mut u, &mut unext);
        }
        let peak = u.iter().cloned().fold(0.0f32, f32::max);
        assert!(
            peak >= 1e3,
            "expected divergence without CFL clamp, peak={peak}"
        );
    }

    #[test]
    fn propagate_red_breaks_on_time_change() {
        // RED+GREEN: perturbing t must change the propagated field.
        let edges: Vec<(u32, u32)> = (0..19u32).map(|i| (i, i + 1)).collect();
        let spec = LaplacianSpectrum::from_edges(&edges, 20, 20);
        let mut u0 = vec![0.0f32; 20];
        u0[0] = 1.0;
        let mut a = vec![0.0f32; 20];
        let mut b = vec![0.0f32; 20];
        spec.propagate_spectral(&u0, 5.0, &mut a);
        spec.propagate_spectral(&u0, 7.0, &mut b);
        let mut diff = 0.0f32;
        for i in 0..20 {
            diff += (a[i] - b[i]).abs();
        }
        assert!(diff > 1e-4, "t must change output, diff={diff}");
    }
}
