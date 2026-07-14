//! SEAL-feedback — the closed-loop "self-editing" control law for the field.
//!
//! Analogy made deterministic: SEAL has a model rewrite its own weights from
//! observed performance. Here the "weights" are the field's per-node TOLERANCE
//! (the veto threshold). The existing `rust-core` already accrues
//! `field_energy[i]` = Σ|Δu| per node but NEVER reads it back. This module is
//! that read-back: sustained high field energy at a node → its local tolerance
//! TIGHTENS (damping ↑), shedding future conflict energy there. Calm field →
//! tolerance holds.
//!
//! NO LLM, NO rng, NO wall-clock. Pure function of (energy history, base tol).
//! RED+GREEN tests prove high sustained energy tightens tolerance and a calm
//! field leaves it unchanged.

/// Per-node SEAL update: given an energy history `energy[i]` (Σ|Δu| over
/// propagations) and a `base` tolerance, return a tightened tolerance
/// `base / (1 + k·energy_norm[i])` where `energy_norm` is the energy divided
/// by its max (so the hottest node is pulled hardest). `k` is the learning
/// rate (damping coefficient). Returns the per-node tolerance vector.
pub fn seal_tighten(energy: &[f64], base: f64, k: f64) -> Vec<f64> {
    let n = energy.len();
    if n == 0 {
        return vec![];
    }
    let max_e = energy.iter().cloned().fold(0.0f64, f64::max).max(1e-12);
    (0..n)
        .map(|i| {
            let norm = energy[i] / max_e; // [0,1]
            base / (1.0 + k * norm) // high energy → smaller tolerance (tighter veto)
        })
        .collect()
}

/// Whether the field has reached a stationary (calm) point: max energy change
/// between two consecutive snapshots is below `eps` (hysteresis band). This is
/// the "does the system stop oscillating?" check — the hysteresis guard.
pub fn is_stationary(prev: &[f64], cur: &[f64], eps: f64) -> bool {
    if prev.len() != cur.len() || prev.is_empty() {
        return false;
    }
    let mut max_d = 0.0f64;
    for i in 0..prev.len() {
        let d = (cur[i] - prev[i]).abs();
        if d > max_d {
            max_d = d;
        }
    }
    max_d < eps
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn high_energy_tightens_tolerance() {
        // RED+GREEN: a node with high accumulated energy gets a STRICTER
        // (smaller) tolerance than a calm node. Prove the gradient.
        let energy = [0.0f64, 5.0, 0.2]; // node1 hot, node2 calm
        let base = 0.10f64;
        let tol = seal_tighten(&energy, base, 2.0);
        assert!(tol[1] < base, "hot node must tighten below base");
        assert!(tol[1] < tol[0], "hot < calm");
        assert!((tol[0] - base).abs() < 1e-9, "calm node ~ base");
    }

    #[test]
    fn calm_field_holds_tolerance() {
        // GREEN: a field with ZERO accrued energy (norm=0) keeps the base tolerance
        // everywhere — no spurious tightening when nothing moved.
        let energy = [0.0f64, 0.0, 0.0];
        let base = 0.10f64;
        let tol = seal_tighten(&energy, base, 2.0);
        for t in &tol {
            assert!((t - base).abs() < 1e-9, "calm field keeps base tol");
        }
    }

    #[test]
    fn stationary_detects_settling() {
        // RED+GREEN: two near-identical snapshots ARE stationary; a big jump is NOT.
        let a = [0.5f64, 0.3, 0.1];
        let b = [0.51f64, 0.30, 0.10];
        assert!(is_stationary(&a, &b, 0.05), "small drift = stationary");
        let c = [0.9f64, 0.3, 0.1]; // big jump at node0
        assert!(!is_stationary(&a, &c, 0.05), "big jump = not stationary");
    }
}
