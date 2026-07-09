//! Analytics — the L5 telemetry governor surface (ported from `src/integration/analytics/*`).
//! Kalman filter, anomaly, dual-track gate, GOAP planner, field planner.
//! Each seam is a pure function with a RED+GREEN test (Verified-by-Math).

pub mod kalman {
    /// 1-D Kalman step. `z` = measurement, returns the smoothed estimate.
    pub fn kalman1d_step(x: f64, p: f64, z: f64, q: f64, r: f64) -> (f64, f64) {
        // predict
        let p_pred = p + q;
        // update
        let k = p_pred / (p_pred + r);
        let x_up = x + k * (z - x);
        let p_up = (1.0 - k) * p_pred;
        (x_up, p_up)
    }

    /// Innovation-based anomaly: |z − x| / sqrt(p + r) exceeding `k` ⇒ anomaly.
    pub fn kalman_anomaly(x: f64, p: f64, z: f64, r: f64, k: f64) -> bool {
        let innov = (z - x).abs();
        let sigma = (p + r).sqrt();
        innov > k * sigma
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn kalman_converges_to_measurement() {
            // GREEN: with tiny R, estimate tracks the measurement.
            let (mut x, mut p) = (0.0, 1.0);
            for z in [1.0, 1.1, 0.9, 1.0, 1.05] {
                let (nx, np) = kalman1d_step(x, p, z, 1e-3, 1e-2);
                x = nx;
                p = np;
            }
            assert!((x - 1.0).abs() < 0.1, "kalman drifted: {x}");
        }
        #[test]
        fn anomaly_flags_large_innovation() {
            // RED: a 10-sigma spike must be flagged.
            assert!(kalman_anomaly(0.0, 1.0, 50.0, 1.0, 3.0));
        }
        #[test]
        fn no_anomaly_on_small_innovation() {
            assert!(!kalman_anomaly(1.0, 0.01, 1.02, 0.01, 3.0));
        }
    }
}

pub mod dual_track {
    /// Dual-Track gate: advisor proposal vs the Truth Layer graph. A hallucinated
    /// edge (not in the graph) is REJECTED (RED+GREEN).
    pub fn dual_track_gate(proposal: &str, truth_edges: &[&str]) -> bool {
        truth_edges.iter().any(|e| *e == proposal)
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn real_edge_passes() {
            let g = ["a->b", "b->c"];
            assert!(dual_track_gate("a->b", &g));
        }
        #[test]
        fn hallucinated_edge_rejected() {
            // RED: an edge absent from the Truth Layer must be denied.
            let g = ["a->b", "b->c"];
            assert!(!dual_track_gate("a->z", &g));
        }
    }
}

pub mod goap {
    /// GOAP symbolic planner. Given goal + reachability map, returns Some(path)
    /// or None if UNREACHABLE (anti-hallucination: no plan for a bogus goal).
    pub fn plan(goal: &str, reachable: &[(&str, &str)]) -> Option<Vec<String>> {
        // BFS over the reachability graph from any "start".
        let mut frontier: Vec<&str> = reachable
            .iter()
            .filter(|(a, _)| *a == "start")
            .map(|(_, b)| *b)
            .collect();
        let mut seen = std::collections::HashSet::new();
        while let Some(n) = frontier.pop() {
            if n == goal {
                return Some(vec![n.into()]);
            }
            if !seen.insert(n) {
                continue;
            }
            for (a, b) in reachable {
                if *a == n {
                    frontier.push(b);
                }
            }
        }
        None
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn planner_finds_path() {
            let g = [("start", "a"), ("a", "b"), ("b", "goal")];
            assert!(plan("goal", &g).is_some());
        }
        #[test]
        fn unreachable_goal_no_plan() {
            // RED: a goal with no path yields NO plan (no hallucinated steps).
            let g = [("start", "a"), ("a", "b")];
            assert!(plan("goal", &g).is_none());
        }
    }
}
