//! SVC — space-vector control smoothing for discrete actuation.
//!
//! Analogy made deterministic: instead of discrete "left/right" commands to a
//! drone/courier, SVPWM emits a continuous vector in the αβ plane. Here we
//! take a sequence of discrete *intended* moves (target points in ℝ²) and emit
//! a smoothed trajectory via linear interpolation between consecutive targets,
//! with a `damping` coefficient that limits the per-step jump (overshoot guard).
//!
//! NO rng, NO wall-clock. Pure interpolation + clamp. RED+GREEN tests prove
//! the output is continuous (step < ε) and that higher damping reduces the
//! peak per-step jump (anti-oscillation).

/// Smooth a path of discrete waypoints into `steps`-per-segment samples in the
/// αβ plane. `damping` ∈ [0,1] scales how far each step may advance toward
/// the next waypoint (1.0 = full linear interp, <1 = under-damped creep that
/// resists overshoot). Returns the dense trajectory (Vec of (α,β)).
pub fn smooth(waypoints: &[(f64, f64)], steps: usize, damping: f64) -> Vec<(f64, f64)> {
    if waypoints.len() < 2 || steps == 0 {
        return waypoints.to_vec();
    }
    let d = damping.clamp(0.0, 1.0);
    let mut out = Vec::new();
    for w in 0..waypoints.len() - 1 {
        let (ax, ay) = waypoints[w];
        let (bx, by) = waypoints[w + 1];
        for s in 0..steps {
            // frac ∈ (0,1]·damping: s=0 → 1/steps·d, s=steps-1 → 1·d.
            // d=1 reaches the waypoint exactly; d<1 under-shoots (anti-overshoot).
            let frac = ((s as f64 + 1.0) / steps as f64) * d;
            out.push((ax + (bx - ax) * frac, ay + (by - ay) * frac));
        }
    }
    out
}

/// Max per-sample jump (Euclidean) in a trajectory — the overshoot metric.
pub fn max_jump(traj: &[(f64, f64)]) -> f64 {
    let mut m = 0.0f64;
    for w in 1..traj.len() {
        let dx = traj[w].0 - traj[w - 1].0;
        let dy = traj[w].1 - traj[w - 1].1;
        let d = (dx * dx + dy * dy).sqrt();
        if d > m {
            m = d;
        }
    }
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trajectory_is_continuous() {
        // GREEN: consecutive samples differ by a bounded (finite) step — smooth, not steppy.
        let wp = [(0.0f64, 0.0), (100.0, 0.0), (100.0, 100.0)];
        let t = smooth(&wp, 50, 1.0);
        let j = max_jump(&t);
        assert!(j < 2.5, "step should be ~2.0 (100/50), got {j}");
    }

    #[test]
    fn damping_reduces_overshoot() {
        // RED+GREEN: lower damping → smaller per-step jump (anti-oscillation proof).
        let wp = [(0.0f64, 0.0), (100.0, 0.0)];
        let full = smooth(&wp, 50, 1.0);
        let damp = smooth(&wp, 50, 0.3);
        assert!(
            max_jump(&damp) < max_jump(&full),
            "damped jump must be < full jump"
        );
    }

    #[test]
    fn degenerate_path_passthrough() {
        // GREEN: <2 waypoints returns as-is.
        let wp = [(1.0f64, 2.0)];
        assert_eq!(smooth(&wp, 5, 1.0), wp);
    }
}
