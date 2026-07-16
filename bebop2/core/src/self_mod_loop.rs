//! W9 — self-mod-effector LIVE driver.
//!
//! The [`SelfModEffector`] (W5) was built + fail-closed but NEVER driven in a
//! live loop — only unit-tested. The operator authorized it ("allow activated
//! self-mod-effector"); this module makes it ACTUALLY EXECUTE, routing every
//! proposed revision through the adversarial `deliberate()` mirror dialogue
//! (operator mandate: research/reason/plan/self-mod MUST pass author↔mirror
//! before applying) and still hard-refusing human-gated red-lines.
//!
//! # Decision flow (per `observe`)
//! 1. propose the SAME localized q-scaler the effector's floor gate uses;
//! 2. build a proposal string, run it through `deliberate()` with `ParamMirror`;
//! 3. ONLY if the mirror converged (`Outcome::Agreed`, or least-friction with
//!    zero objections) call `effector.tick` — else fail-closed (no non-consensual
//!    self-mod). Human-gated ops are delegated untouched to `human_gated_op`.

use crate::deliberate::{deliberate, Mirror, Outcome, Role, Utterance};
use crate::kalman::KalmanFilter;
use crate::self_mod::{EffectorReject, SelfModCapability, SelfModEffector};

/// Max relative change a mirror will accept in one step (|Δθ|/θ).
const MAX_DELTA_FRACTION: f64 = 0.5;
/// A candidate below this fraction of the current scaler chokes process noise
/// (over-damping) — the mirror objects.
const DAMP_FLOOR: f64 = 0.5;

/// The result of one `observe` cycle (auditable upstream via `effector.log()`).
#[derive(Debug, Clone)]
pub struct LoopOutcome {
    /// Whether the revision was actually applied to the filter.
    pub applied: bool,
    /// Current q-scaler after the cycle (unchanged if the mirror refused).
    pub q_scaler: f64,
    /// How the mirror dialogue terminated.
    pub dialogue: Outcome,
    /// Final open objections the mirror still held (empty ⇒ agreement).
    pub objections: Vec<String>,
}

/// The live self-mod driver: owns the effector and gates every revision
/// through the mirror dialogue before applying.
pub struct SelfModLoop {
    effector: SelfModEffector,
}

impl SelfModLoop {
    /// Build a driver over `filter`, with noether tolerance `noether_tol`.
    pub fn new(filter: KalmanFilter, noether_tol: f64) -> Self {
        Self {
            effector: SelfModEffector::new(filter, noether_tol),
        }
    }

    /// Observe one window: propose → mirror-dialogue → apply iff consensual.
    ///
    /// Fail-closed: if the mirror does not converge to agreement, NOTHING is
    /// applied (q-scaler + filter untouched) and `applied` is `false`.
    pub fn observe(&mut self, surprise: f64, rho: f64) -> LoopOutcome {
        // (1) candidate via the SAME math the effector floor-gates with.
        let candidate = self.effector.propose(surprise, rho);
        let proposal = format!("set_q_scaler {candidate:.4}");

        // (2) route through the adversarial mirror dialogue (≤2 laps).
        let reference = self.effector.current_q_scaler();
        let mirror = ParamMirror::new(reference);
        let conclusion = deliberate(&proposal, &mirror, |p, _| p.to_string());

        // (3) apply ONLY if the mirror converged to agreement.
        let objections = final_objections(&conclusion);
        let may_apply = match conclusion.outcome {
            Outcome::Agreed => true,
            // least-friction with zero open objections == effectively agreed.
            Outcome::LeastFrictionAdopted => objections.is_empty(),
        };
        let applied = if may_apply {
            self.effector
                .tick(&SelfModCapability::authorized(), surprise, rho)
                .is_ok()
        } else {
            false
        };

        LoopOutcome {
            applied,
            q_scaler: self.effector.current_q_scaler(),
            dialogue: conclusion.outcome,
            objections,
        }
    }

    /// HARD-REFUSED human-gated red-lines (delegated, untouched).
    pub fn human_gated(&mut self, op: &'static str) -> Result<(), EffectorReject> {
        self.effector.human_gated_op(op)
    }

    pub fn q_scaler(&self) -> f64 {
        self.effector.current_q_scaler()
    }
    pub fn log_len(&self) -> usize {
        self.effector.log().len()
    }
    pub fn human_gate_firings(&self) -> u64 {
        self.effector.human_gate_firings()
    }
}

/// Extract the mirror's final open objections from a dialogue transcript.
fn final_objections(c: &crate::deliberate::Conclusion) -> Vec<String> {
    c.transcript
        .iter()
        .rev()
        .find(|u: &&Utterance| u.role == Role::Mirror)
        .map(|u| u.open_objections.clone())
        .unwrap_or_default()
}

/// The operator's adversarial mirror: a second vantage on the noether concern.
/// It objects if the proposed q-scaler delta is too large (could drift/over-fit)
/// or would over-damp (choke process noise). Empty vec ⇒ agree.
struct ParamMirror {
    reference: f64,
}

impl ParamMirror {
    fn new(reference: f64) -> Self {
        Self { reference }
    }
}

impl Mirror for ParamMirror {
    fn critique(&self, proposal: &str) -> Vec<String> {
        let candidate = parse_scaler(proposal);
        let mut objs = Vec::new();
        let rel = (candidate - self.reference).abs() / self.reference.max(1e-9);
        if rel > MAX_DELTA_FRACTION {
            objs.push(format!(
                "q-scaler delta {rel:.4} too large (> {MAX_DELTA_FRACTION:.2} of {:.4})",
                self.reference
            ));
        }
        if candidate < self.reference * DAMP_FLOOR {
            objs.push(format!(
                "would over-damp: candidate {candidate:.4} < floor {:.4}",
                self.reference * DAMP_FLOOR
            ));
        }
        objs
    }
}

/// Parse the candidate scaler out of `set_q_scaler {:.4}`; fall back to 1.0.
fn parse_scaler(p: &str) -> f64 {
    p.trim()
        .strip_prefix("set_q_scaler ")
        .and_then(|s| s.trim().parse::<f64>().ok())
        .unwrap_or(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn driver() -> SelfModLoop {
        // 2-D constant-velocity filter; identity A, tiny Q, zero state.
        let a = [1.0, 0.0, 0.0, 1.0];
        let q = [1e-6, 0.0, 0.0, 1e-6];
        let x0 = [0.0, 0.0, 0.0, 0.0];
        let p0 = [1.0, 0.0, 0.0, 1.0];
        let kf = KalmanFilter::new(&a, &q, &x0, &p0, 2);
        SelfModLoop::new(kf, 1e-3)
    }

    // (1) small surprise/rho → mirror agrees → effect applied, q-scaler changes.
    #[test]
    fn loop_applies_mirror_agreed_revision() {
        let mut l = driver();
        let before = l.q_scaler();
        let out = l.observe(0.05, 0.02);
        assert!(out.applied, "consensual revision must apply");
        assert_eq!(out.dialogue, Outcome::Agreed);
        assert!(out.q_scaler > before, "q-scaler must move upward");
        assert!(l.log_len() > 0, "audit log must grow on apply");
        assert!(out.objections.is_empty(), "no open objections at agreement");
    }

    // (2) huge surprise → mirror objects → NOT applied, q-scaler unchanged.
    #[test]
    fn loop_refuses_mirror_objected_revision() {
        let mut l = driver();
        let before = l.q_scaler();
        let out = l.observe(1e6, 1e6);
        assert!(!out.applied, "non-consensual self-mod must be refused");
        assert_eq!(out.q_scaler, before, "q-scaler must be untouched");
        assert!(!out.objections.is_empty(), "mirror must hold objections");
        assert_eq!(out.dialogue, Outcome::LeastFrictionAdopted);
    }

    // (3) red-lines still hard-refused through the loop.
    #[test]
    fn loop_still_hard_refuses_red_lines() {
        let mut l = driver();
        assert!(matches!(
            l.human_gated("push-to-main"),
            Err(EffectorReject::HumanGated(_))
        ));
        assert!(l.human_gate_firings() >= 1, "gate must record the firing");
    }
}
