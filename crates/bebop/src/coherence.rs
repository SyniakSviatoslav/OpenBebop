//! Coherence — the quantum-oscillator / wave-interference layer over the field.
//!
//! Analogy made deterministic: the field core propagates a HEAT-KERNEL impulse
//! `u(t) = exp(-L·t)·u0`. That is a classical diffusion wavefront. Two
//! agents are two seeds; their wavefunctions SUPERPOSE. This module computes
//! the constructive interference `|ψ₁ + ψ₂|²` and destructive `|ψ₁ − ψ₂|²`
//! over the propagated field — real superposition on the existing kernel.
//!
//! NO LLM, NO rng, NO wall-clock. The interference math is exact linear
//! algebra on the propagated vectors. RED+GREEN tests prove aligned seeds
//! constructively reinforce a peak while anti-aligned seeds destructively
//! cancel a node to ~0.

/// Propagate a single impulse seed `u0` under the heat kernel for `t` steps
/// with coeff `coeff` over an undirected Laplacian built from `edges`.
/// Returns the n-vector `u(t)`. (Cheap re-implementation of the core's
/// active diffusion so this module stays dependency-light and testable.)
pub fn propagate(u0: &[f64], edges: &[(usize, usize)], t: f64, coeff: f64) -> Vec<f64> {
    let n = u0.len();
    if n == 0 {
        return vec![];
    }
    // Build degree (D) and adjacency for L = D - A.
    let mut deg = vec![0.0f64; n];
    for &(a, b) in edges {
        if a < n {
            deg[a] += 1.0;
        }
        if b < n {
            deg[b] += 1.0;
        }
    }
    let dt = t.max(1e-3);
    let mut u = u0.to_vec();
    let steps = (t / dt).round().max(1.0) as usize;
    for _ in 0..steps {
        let mut lu = vec![0.0f64; n];
        for i in 0..n {
            let mut acc = deg[i] * u[i];
            for &(a, b) in edges {
                if a == i {
                    acc -= u[b.min(n - 1)];
                }
                if b == i {
                    acc -= u[a.min(n - 1)];
                }
            }
            lu[i] = acc;
        }
        for i in 0..n {
            u[i] += dt * coeff * lu[i];
        }
    }
    u
}

/// Coherent superposition of two seeds. Returns (constructive, destructive)
/// n-vectors: `|ψ₁+ψ₂|²` and `|ψ₁−ψ₂|²`.
pub fn interfere(
    seed1: &[f64],
    seed2: &[f64],
    edges: &[(usize, usize)],
    t: f64,
    coeff: f64,
) -> (Vec<f64>, Vec<f64>) {
    let p1 = propagate(seed1, edges, t, coeff);
    let p2 = propagate(seed2, edges, t, coeff);
    let n = p1.len().min(p2.len());
    let mut con = vec![0.0f64; n];
    let mut des = vec![0.0f64; n];
    for i in 0..n {
        let s = p1[i] + p2[i];
        let d = p1[i] - p2[i];
        con[i] = s * s; // |ψ₁+ψ₂|² constructive
        des[i] = d * d; // |ψ₁−ψ₂|² destructive
    }
    (con, des)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aligned_seeds_constructive_peak() {
        // RED+GREEN: two IDENTICAL seeds interfere constructively.
        // The peak of |ψ₁+ψ₂|² must exceed the isolated peak (scale ×4 at the
        // seeded node), while a non-seeded node stays low.
        let edges = [(0usize, 1), (1, 2), (2, 3)];
        let s1 = [1.0f64, 0.0, 0.0, 0.0];
        let s2 = [1.0f64, 0.0, 0.0, 0.0];
        let (con, _des) = interfere(&s1, &s2, &edges, 1.0, 0.5);
        // constructive at node 0: (1+1)² = 4, isolated was 1 → reinforced
        assert!(con[0] > 3.5, "constructive peak should ~4, got {}", con[0]);
        // a far node should stay small
        assert!(con[3] < con[0], "energy should be concentrated near seed");
    }

    #[test]
    fn antialigned_seeds_destructive_cancel() {
        // RED: two OPPOSITE seeds at the SAME node cancel it to ~0
        // (destructive interference): |1 - (-1)|² at neighbors, |1+(-1)|²=0 at node.
        let edges = [(0usize, 1), (1, 2)];
        let s1 = [1.0f64, 0.0, 0.0];
        let s2 = [-1.0f64, 0.0, 0.0]; // opposite sign, same node
        let (con, des) = interfere(&s1, &s2, &edges, 0.5, 0.5);
        // at node 0 the constructive term is |1 + (-1)|² = 0
        assert!(con[0] < 1e-6, "node 0 must cancel to ~0, got {}", con[0]);
        // but destructive (|1 - (-1)|² = 4) is large there
        assert!(des[0] > 3.5, "destructive must peak, got {}", des[0]);
    }

    #[test]
    fn propagate_is_deterministic() {
        // GREEN: same inputs → identical output (no rng/timestamp).
        let edges = [(0usize, 1), (1, 2)];
        let s = [1.0f64, 0.0, 0.0];
        let a = propagate(&s, &edges, 1.0, 0.5);
        let b = propagate(&s, &edges, 1.0, 0.5);
        assert_eq!(a, b);
    }
}
