//! energy — vectorless graph-energy + spectral-radius primitives for bebop2.
//!
//! A *vectorless* spectral signature of a graph: every quantity here is derived
//! purely from the eigenvalue spectrum of the (symmetric) adjacency matrix,
//! independent of any node embedding / coordinate assignment. This mirrors the
//! `graph_energy` / `spectral_radius` functions in the dowiz kernel
//! (`kernel/src/spectral.rs`) but routes through bebop2's SINGLE authoritative
//! eigensolver (`linalg::eigenvalues`) — no second, unsupervised eigen-copy is
//! allowed to drift (dual-authority hazard kill).
//!
//! Quantities:
//!   * `graph_energy`  E = Σ|λᵢ|  (Gutman–Adrić, 2001) — "how active is this
//!     graph": large when the spectrum spans many alternating-sign modes.
//!     Bounds for an n-vertex graph: 2(n−1) ≤ E ≤ 2n·√(n−1).
//!   * `spectral_radius` ρ = maxᵢ|λᵢ| — the stability / magnitude dial.
//!
//! MATH
//!   Eigenvalues come from `linalg::eigenvalues` (Faddeev–LeVerrier char-poly +
//!   Durand–Kerner root-find — zero-dep, deterministic, NO RNG). We only fold
//!   over the resulting spectrum. Pure, no I/O, no network.
//!
//! Float is used deliberately — this is graph/operator structure, never money
//! (the no-float rule is money-only). Verified-by-Math tests below.

use alloc::vec::Vec;

use crate::linalg::{eigenvalues, Complex};

/// Graph energy `E = Σ|λᵢ|` over ALL eigenvalues of the adjacency matrix `adj`.
///
/// `adj` is the row-major `n×n` (unweighted, undirected) adjacency matrix — entry
/// `adj[i][j]` is 1 iff vertices `i` and `j` share an edge (0 on the diagonal).
/// The sum of eigenvalue magnitudes is a topological invariant: it is large for
/// highly "active" graphs and small/zero for spectrally-flat ones.
///
/// # Panics
/// Debug builds assert `adj` is square (`n×n`).
pub fn graph_energy(adj: &[Vec<f64>]) -> f64 {
    let n = adj.len();
    debug_assert!(
        n == 0 || adj.iter().all(|r| r.len() == n),
        "graph_energy: expected a square n×n adjacency matrix"
    );
    eigenvalues(adj).iter().map(Complex::abs).sum()
}

/// Spectral radius `ρ(A) = maxᵢ|λᵢ|` of the adjacency matrix `adj`.
///
/// The largest eigenvalue modulus: the master stability/magnitude dial. `ρ < 1`
/// ⇒ a stochastic/contracting operator; `ρ > 1` ⇒ divergent; `ρ ≈ 1` ⇒ marginal
/// (e.g. a period-2 cycle is exactly ρ = 1).
///
/// # Panics
/// Debug builds assert `adj` is square (`n×n`).
pub fn spectral_radius(adj: &[Vec<f64>]) -> f64 {
    let n = adj.len();
    debug_assert!(
        n == 0 || adj.iter().all(|r| r.len() == n),
        "spectral_radius: expected a square n×n adjacency matrix"
    );
    eigenvalues(adj)
        .iter()
        .map(Complex::abs)
        .fold(0.0_f64, f64::max)
}

#[cfg(all(test, feature = "host"))]
mod tests {
    use super::*;

    fn approx(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() < tol
    }

    // ── GREEN: K₃ complete graph has adjacency eigs {2, −1, −1} ⇒ E = 4. ──
    #[test]
    fn green_k3_complete_graph_energy_is_four() {
        let k3 = vec![
            vec![0.0, 1.0, 1.0],
            vec![1.0, 0.0, 1.0],
            vec![1.0, 1.0, 0.0],
        ];
        let e = graph_energy(&k3);
        assert!(approx(e, 4.0, 1e-7), "K3 graph energy should be 4, got {e}");
        // spectral radius of K3 is the top eigenvalue magnitude = 2.
        assert!(
            approx(spectral_radius(&k3), 2.0, 1e-7),
            "ρ(K3) should be 2, got {}",
            spectral_radius(&k3)
        );
    }

    // ── GREEN: empty (edgeless) graph ⇒ every eigenvalue 0 ⇒ E = 0, ρ = 0. ──
    #[test]
    fn green_empty_graph_energy_is_zero() {
        let empty = vec![
            vec![0.0, 0.0, 0.0],
            vec![0.0, 0.0, 0.0],
            vec![0.0, 0.0, 0.0],
        ];
        let e = graph_energy(&empty);
        assert!(
            approx(e, 0.0, 1e-12),
            "empty graph energy should be 0, got {e}"
        );
        assert!(
            approx(spectral_radius(&empty), 0.0, 1e-12),
            "ρ(empty) should be 0, got {}",
            spectral_radius(&empty)
        );
    }

    // ── GREEN: a directed 2-cycle has eigenvalues ±1 ⇒ ρ = 1. ──
    #[test]
    fn green_two_cycle_spectral_radius_is_one() {
        let c = vec![vec![0.0, 1.0], vec![1.0, 0.0]];
        let rho = spectral_radius(&c);
        assert!(approx(rho, 1.0, 1e-9), "ρ=1 for a 2-cycle, got {rho}");
        // graph energy Σ|λ| = |+1| + |−1| = 2.
        assert!(
            approx(graph_energy(&c), 2.0, 1e-9),
            "E(2-cycle) should be 2, got {}",
            graph_energy(&c)
        );
    }
}
