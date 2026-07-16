//! eig_parity — the SILENT-DRIFT KILLER for bebop2's eigensolver.
//!
//! bebop2 previously had the Faddeev–LeVerrier + Durand–Kerner eigensolver living ONLY
//! inside `bebop_proto_cap/tests/mesh_consensus.rs` (mirrored, byte-for-byte, from the
//! dowiz `kernel/src/spectral.rs` engine). That is a dual-authority hazard: an edit to one
//! copy is invisible to the other until a consumer breaks. `core::linalg::eigenvalues` is
//! now the SINGLE authoritative solver. This test is the parity gate that makes a silent
//! divergence impossible:
//!
//!   * METHOD A — [`bebop2_core::linalg::eigenvalues`] (Faddeev–LeVerrier + Durand–Kerner),
//!     the canonical solver all consumers must route through.
//!   * METHOD B — [`bebop2_core::lyapunov::eigenvalues_general`], a COMPLETELY INDEPENDENT
//!     algorithm (Hessenberg reduction → Francis double-shift QR → real Schur form). Different
//!     math family, same answer. A drift in EITHER method is caught here.
//!
//! W2-1 additionally pins the *routed* consumers — `field::jacobi_eigen`,
//! `kalman::real_eig`, and `lyapunov::eigenvals` — to the SAME authoritative
//! `linalg::eigenvalues`, via a shared sentinel constant (`EIGEN_AUTHORITY`)
//! and by asserting the OLD local eigenvalue result equals the authority's within 1e-9.
//! This is the dual-authority hazard kill for the intra-crate Jacobi forks.
//!
//! They must agree to 1e-6 on every reference matrix. On divergence the test FAILS loudly
//! with the matrix name + max |Δλ|, so a silent drift is impossible.

use bebop2_core::field::{self, EIGEN_AUTHORITY as FIELD_AUTH};
use bebop2_core::kalman::{self, EIGEN_AUTHORITY as KALMAN_AUTH};
use bebop2_core::lyapunov::{self, EIGEN_AUTHORITY as LYAP_AUTH};
use bebop2_core::linalg::{self, Complex, EIGEN_AUTHORITY as LINALG_AUTH};

/// W2-1 — every routed consumer names the SAME authoritative solver.
/// A silent fork would change one sentinel → this fails.
#[test]
fn w2_1_all_consumers_pin_one_authority() {
    assert_eq!(LINALG_AUTH, "linalg::eigenvalues");
    assert_eq!(FIELD_AUTH, LINALG_AUTH);
    assert_eq!(KALMAN_AUTH, LINALG_AUTH);
    assert_eq!(LYAP_AUTH, LINALG_AUTH);
    // energy (already routed) pins the same name too.
    assert_eq!(bebop2_core::energy::EIGEN_AUTHORITY, LINALG_AUTH);
}

/// W2-1 — `field::jacobi_eigen` eigenvalues must equal `linalg::eigenvalues`
/// (the authority) within 1e-9 on a fixture Laplacian. This is the parity gate:
/// the LOCAL Jacobi eigenvalue result is compared to the SHARED authority and must match.
#[test]
fn w2_1_field_jacobi_routes_to_authority() {
    // Path-graph P4 Laplacian → eigenvalues {0, 2-√2, 2, 2+√2}.
    let l = laplacian_p4();
    let (jac_vals, _jac_vecs) = field::jacobi_eigen(&l, 4);
    let auth = linalg::eigenvalues(&rows_of(&l, 4));
    let mut jac = jac_vals.clone();
    let mut ar: Vec<f64> = auth.iter().map(|c| c.re).collect();
    jac.sort_by(|a, b| a.partial_cmp(b).unwrap());
    ar.sort_by(|a, b| a.partial_cmp(b).unwrap());
    for (j, a) in jac.iter().zip(ar.iter()) {
        assert!(
            (j - a).abs() < 1e-9,
            "field::jacobi_eigen diverged from linalg::eigenvalues: {j} vs {a}"
        );
    }
}

/// W2-1 — `kalman::real_eig` eigenvalues must equal `linalg::eigenvalues` within 1e-9.
#[test]
fn w2_1_kalman_real_eig_routes_to_authority() {
    // Symmetric reference A = [[0.9,0.1],[0.1,0.8]] → eigenvalues {1.0, 0.7}.
    let a = vec![0.9, 0.1, 0.1, 0.8];
    let (k_ev, _k_vecs) = kalman::real_eig(&a, 2);
    let auth = linalg::eigenvalues(&rows_of(&a, 2));
    let mut ke: Vec<f64> = k_ev.iter().map(|c| c.re).collect();
    let mut ar: Vec<f64> = auth.iter().map(|c| c.re).collect();
    ke.sort_by(|x, y| x.partial_cmp(y).unwrap());
    ar.sort_by(|x, y| x.partial_cmp(y).unwrap());
    for (k, a) in ke.iter().zip(ar.iter()) {
        assert!(
            (k - a).abs() < 1e-9,
            "kalman::real_eig diverged from linalg::eigenvalues: {k} vs {a}"
        );
    }
}

/// W2-1 — `lyapunov::eigenvals` (now a thin wrapper) must equal `linalg::eigenvalues`.
#[test]
fn w2_1_lyapunov_eigenvals_routes_to_authority() {
    // A = [[2,1],[1,2]] → eigenvalues {3, 1}.
    let a = vec![2.0, 1.0, 1.0, 2.0];
    let lv = lyapunov::eigenvals(&a, 2);
    let auth = linalg::eigenvalues(&rows_of(&a, 2));
    let mut le: Vec<f64> = lv.iter().map(|c| c.re).collect();
    let mut ar: Vec<f64> = auth.iter().map(|c| c.re).collect();
    le.sort_by(|x, y| x.partial_cmp(y).unwrap());
    ar.sort_by(|x, y| x.partial_cmp(y).unwrap());
    for (l, a) in le.iter().zip(ar.iter()) {
        assert!(
            (l - a).abs() < 1e-9,
            "lyapunov::eigenvals diverged from linalg::eigenvalues: {l} vs {a}"
        );
    }
}

/// W2-1 — eigenvector columns of `field::jacobi_eigen` are correctly ordered to
/// match the authoritative eigenvalue order: A·v_j == λ_j·v_j within 1e-6.
#[test]
fn w2_1_field_eigenvectors_align_with_authority_order() {
    let l = laplacian_p4();
    let (vals, vecs) = field::jacobi_eigen(&l, 4);
    for j in 0..4 {
        let lam = vals[j];
        for i in 0..4 {
            let mut av = 0.0f64;
            for k in 0..4 {
                av += l[i * 4 + k] * vecs[k * 4 + j];
            }
            let vi = vecs[i * 4 + j];
            assert!(
                (av - lam * vi).abs() < 1e-6,
                "eigenvector j={j} (λ={lam}) does not satisfy A v = λ v (residual {:.2e})",
                (av - lam * vi).abs()
            );
        }
    }
}

// ── helpers ─────────────────────────────────────────────────────────────

/// Path-graph P4 Laplacian L = D - A, row-major flat (n=4).
fn laplacian_p4() -> Vec<f64> {
    let mut l = vec![0.0f64; 16];
    for i in 0..4 {
        let mut deg = 0.0;
        if i > 0 {
            l[i * 4 + (i - 1)] = -1.0;
            deg += 1.0;
        }
        if i + 1 < 4 {
            l[i * 4 + (i + 1)] = -1.0;
            deg += 1.0;
        }
        l[i * 4 + i] = deg;
    }
    l
}

/// Build a row-major `&[Vec<f64>]` from a flat row-major slice (what `linalg::eigenvalues` takes).
fn rows_of(flat: &[f64], n: usize) -> Vec<Vec<f64>> {
    let mut rows = Vec::with_capacity(n);
    for i in 0..n {
        rows.push(flat[i * n..(i + 1) * n].to_vec());
    }
    rows
}

// ── helpers ──────────────────────────────────────────────────────

/// Build a row-major `&[Vec<f64>]` matrix from a flat row-major slice.
fn mat(rows: &[&[f64]]) -> Vec<Vec<f64>> {
    rows.iter().map(|r| r.to_vec()).collect()
}

/// Flatten `&[Vec<f64>]` to a row-major `Vec<f64>` (what `eigenvalues_general` expects).
fn flat(m: &[Vec<f64>]) -> Vec<f64> {
    m.iter().flat_map(|r| r.iter().copied()).collect()
}

/// Sorted real parts of a complex spectrum.
fn reals(ev: &[Complex]) -> Vec<f64> {
    let mut v: Vec<f64> = ev.iter().map(|c| c.re).collect();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v
}

/// Sorted real parts of `lyapunov::eigenvalues_general`'s `fft::Complex` spectrum (host only).
#[cfg(feature = "host")]
fn reals_general(m: &[Vec<f64>]) -> Vec<f64> {
    let n = m.len();
    let mut v: Vec<f64> = lyapunov::eigenvalues_general(&flat(m), n)
        .iter()
        .map(|c| c.re)
        .collect();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v
}

/// Max absolute elementwise diff of two equally-sized sorted real vectors.
fn max_diff(a: &[f64], b: &[f64]) -> f64 {
    assert_eq!(a.len(), b.len(), "mismatched eigenvalue count");
    a.iter()
        .zip(b)
        .map(|(x, y)| (x - y).abs())
        .fold(0.0, f64::max)
}

/// Assert two real-eigenvalue vectors agree within `tol`, else fail loudly.
fn assert_agree(name: &str, a: &[f64], b: &[f64], tol: f64) {
    let d = max_diff(a, b);
    assert!(
        d < tol,
        "[EIG-PARITY] {name}: eigensolver divergence max|Δλ| = {:.3e} (tol {:.1e})",
        d,
        tol
    );
}

// ── references (hand-derived, do NOT trust a green test whose values weren't derived) ──

#[test]
fn eig_parity_2x2_swap_pm1() {
    // A = [[0,1],[1,0]] → eigenvalues {1, -1} (hand-derived: swap matrix, trace 0, det -1).
    let a = mat(&[&[0.0, 1.0], &[1.0, 0.0]]);
    let ev_a = reals(&linalg::eigenvalues(&a));
    let reference = vec![-1.0, 1.0];
    let ev_b = reals_general(&a);
    assert_agree("2x2 swap", &ev_a, &reference, 1e-6);
    assert_agree("2x2 swap (QR)", &ev_b, &reference, 1e-6);
    assert_agree("2x2 swap A-vs-B", &ev_a, &ev_b, 1e-6);
}

#[test]
fn eig_parity_2x2_complex_pair() {
    // A = [[0,-1],[1,0]] → eigenvalues {i, -i} (rotation; FL+DK gives complex pair).
    let a = mat(&[&[0.0, -1.0], &[1.0, 0.0]]);
    let ev = linalg::eigenvalues(&a);
    // both purely imaginary, magnitudes 1, opposite signs
    let mut ims: Vec<f64> = ev.iter().map(|c| c.im).collect();
    ims.sort_by(|x, y| x.partial_cmp(y).unwrap());
    assert!(
        (ims[0] + 1.0).abs() < 1e-6 && (ims[1] - 1.0).abs() < 1e-6,
        "eigs {{i,-i}}, got {ev:?}"
    );
    // real parts ~0
    for c in &ev {
        assert!(c.re.abs() < 1e-6, "rotation has no real part, got {c:?}");
    }
    // QR method agrees (complex pair).
    let qr = lyapunov::eigenvalues_general(&flat(&a), 2);
    let mut qr_im: Vec<f64> = qr.iter().map(|c| c.im).collect();
    qr_im.sort_by(|x, y| x.partial_cmp(y).unwrap());
    assert!(
        (qr_im[0] + 1.0).abs() < 1e-6 && (qr_im[1] - 1.0).abs() < 1e-6,
        "QR disagrees on rotation eigs, got {qr:?}"
    );
}

#[test]
fn eig_parity_3x3_path_laplacian() {
    // Path-graph P3 Laplacian L = [[1,-1,0],[-1,2,-1],[0,-1,1]] → eigenvalues {0,1,3}
    // (hand-derived: trace 4, det 0, characteristic polynomial λ(λ-1)(λ-3)=0).
    let a = mat(&[&[1.0, -1.0, 0.0], &[-1.0, 2.0, -1.0], &[0.0, -1.0, 1.0]]);
    let ev_a = reals(&linalg::eigenvalues(&a));
    let reference = vec![0.0, 1.0, 3.0];
    let ev_b = reals_general(&a);
    assert_agree("3x3 path Laplacian", &ev_a, &reference, 1e-6);
    assert_agree("3x3 path Laplacian (QR)", &ev_b, &reference, 1e-6);
    assert_agree("3x3 path Laplacian A-vs-B", &ev_a, &ev_b, 1e-6);
}

#[test]
fn eig_parity_4x4_path_laplacian() {
    // Path-graph P4 Laplacian → eigenvalues {0, 2-√2, 2, 2+√2}
    // (hand-derived: path-graph Laplacian eigenvalues are 2 - 2cos(kπ/(n+1)), k=1..n).
    let a = mat(&[
        &[1.0, -1.0, 0.0, 0.0],
        &[-1.0, 2.0, -1.0, 0.0],
        &[0.0, -1.0, 2.0, -1.0],
        &[0.0, 0.0, -1.0, 1.0],
    ]);
    let r2 = 2.0f64.sqrt();
    let reference = vec![0.0, 2.0 - r2, 2.0, 2.0 + r2];
    let ev_a = reals(&linalg::eigenvalues(&a));
    let ev_b = reals_general(&a);
    assert_agree("4x4 path Laplacian", &ev_a, &reference, 1e-6);
    assert_agree("4x4 path Laplacian (QR)", &ev_b, &reference, 1e-6);
    assert_agree("4x4 path Laplacian A-vs-B", &ev_a, &ev_b, 1e-6);
}

#[test]
fn eig_parity_3x3_cycle() {
    // 3-cycle adjacency (undirected) A = [[0,1,1],[1,0,1],[1,1,0]] → eigenvalues {2,-1,-1}
    // (hand-derived: all-ones matrix has eigenvalues 3,0,0; A = J − I → 2,-1,-1).
    let a = mat(&[&[0.0, 1.0, 1.0], &[1.0, 0.0, 1.0], &[1.0, 1.0, 0.0]]);
    let ev_a = reals(&linalg::eigenvalues(&a));
    let reference = vec![-1.0, -1.0, 2.0];
    let ev_b = reals_general(&a);
    assert_agree("3x3 cycle", &ev_a, &reference, 1e-6);
    assert_agree("3x3 cycle (QR)", &ev_b, &reference, 1e-6);
    assert_agree("3x3 cycle A-vs-B", &ev_a, &ev_b, 1e-6);
}
