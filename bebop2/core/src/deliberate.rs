//! Adversarial mirroring deliberation — operator protocol (2026-07-16).
//!
//! Research / reason / plan agents MUST explain their idea to a *critically
//! aligned mirror* agent, which challenges the explanation/reasoning. The two
//! reconcile in dialogue until BOTH agree. The dialogue is capped at **2 laps**
//! (propose → mirror-critique → reconcile → agree counts as one lap; a second
//! lap is the hard ceiling). If agreement is not reached within 2 laps, the
//! **least-friction version** (the proposal with the fewest open objections) is
//! adopted automatically — no third lap, no hang.
//!
//! This module enforces the *mechanics* (lap cap, agreement gate, least-friction
//! tiebreak). The substantive critique logic lives behind the `Mirror` trait so
//! any research/reason/plan agent can plug in its own adversarial check.
//!
//! This is NOT a self-mod effector: it never mutates kernel state. It is a
//! decision-protocol gate that returns the adopted `Conclusion`.

use alloc::string::String;
use alloc::vec::Vec;

/// One side of the dialogue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Role {
    /// The originating research/reason/plan agent proposing the idea.
    Author,
    /// The critically aligned mirror agent challenging it.
    Mirror,
}

/// A single exchange: who spoke, the text, and (for the mirror) the open
/// objections it still holds against the current proposal.
#[derive(Debug, Clone)]
pub struct Utterance {
    pub role: Role,
    pub text: String,
    /// Open objections the mirror still holds (empty ⇒ mirror agrees).
    pub open_objections: Vec<String>,
}

impl Utterance {
    pub fn author(text: impl Into<String>) -> Self {
        Utterance {
            role: Role::Author,
            text: text.into(),
            open_objections: vec![],
        }
    }
    pub fn mirror(text: impl Into<String>, open_objections: Vec<String>) -> Self {
        Utterance {
            role: Role::Mirror,
            text: text.into(),
            open_objections,
        }
    }
    /// A mirror utterance carrying NO open objections ⇒ agreement reached.
    pub fn is_agreement(&self) -> bool {
        self.role == Role::Mirror && self.open_objections.is_empty()
    }
}

/// The mirror's adversarial check. Implementors return the objections still
/// standing against `proposal`. Returning an empty `Vec` means "I agree".
pub trait Mirror {
    fn critique(&self, proposal: &str) -> Vec<String>;
}

/// Outcome of a deliberation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Both agents converged within the lap cap.
    Agreed,
    /// Lap cap hit without agreement → least-friction version adopted.
    LeastFrictionAdopted,
}

/// The result handed back to the calling agent.
#[derive(Debug, Clone)]
pub struct Conclusion {
    /// The adopted proposal text (reconciled, or the least-friction original).
    pub adopted: String,
    /// How the dialogue terminated.
    pub outcome: Outcome,
    /// Number of laps actually run (1 or 2).
    pub laps: usize,
    /// Full transcript (auditable; mirror it via event-log upstream).
    pub transcript: Vec<Utterance>,
}

/// Run the adversarial mirroring deliberation.
///
/// `proposal` is the author's initial idea. `mirror` critiques; the author's
/// `reconcile` closure turns (proposal, objections) into a revised proposal for
/// the next lap. The loop:
///   lap 1: author proposes → mirror critiques → if no objections, AGREED.
///          else author reconciles → lap 2.
///   lap 2: mirror critiques revised → if no objections, AGREED.
///          else LEAST-FRICTION adopted (revised proposal, fewer open objections
///          than the original if the reconciliation reduced them; otherwise the
///          original). No third lap.
///
/// `MAX_LAPS = 2` is enforced structurally — the loop cannot exceed it.
pub fn deliberate<A, M>(initial: &str, mirror: &M, mut reconcile: A) -> Conclusion
where
    A: FnMut(&str, &[String]) -> String,
    M: Mirror,
{
    const MAX_LAPS: usize = 2;
    let mut transcript = Vec::new();

    // ── lap 1 ──
    transcript.push(Utterance::author(String::from(initial)));
    let obj1 = mirror.critique(initial);
    transcript.push(Utterance::mirror(
        format!("lap1 critique: {} open", obj1.len()),
        obj1.clone(),
    ));
    if obj1.is_empty() {
        return Conclusion {
            adopted: String::from(initial),
            outcome: Outcome::Agreed,
            laps: 1,
            transcript,
        };
    }

    // ── reconcile → lap 2 ──
    let revised = reconcile(initial, &obj1);
    transcript.push(Utterance::author(format!("reconciled for lap2")));
    let obj2 = mirror.critique(&revised);
    transcript.push(Utterance::mirror(
        format!("lap2 critique: {} open", obj2.len()),
        obj2.clone(),
    ));

    if obj2.is_empty() {
        return Conclusion {
            adopted: revised,
            outcome: Outcome::Agreed,
            laps: 2,
            transcript,
        };
    }

    // ── lap cap hit: least-friction version wins ──
    // Lower objection count = less friction. Tie or worse → original stands
    // (never reward a reconciliation that failed to reduce objections).
    let (adopted, laps) = if obj2.len() < obj1.len() {
        (revised, 2)
    } else {
        (String::from(initial), 2)
    };
    Conclusion {
        adopted,
        outcome: Outcome::LeastFrictionAdopted,
        laps,
        transcript,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A mirror that agrees with anything (no objections).
    struct AgreeingMirror;
    impl Mirror for AgreeingMirror {
        fn critique(&self, _p: &str) -> Vec<String> {
            vec![]
        }
    }

    /// A mirror that always objects to the original but accepts the reconciled
    /// form (simulates a real critique that the author can satisfy).
    struct SatisfiableMirror;
    impl Mirror for SatisfiableMirror {
        fn critique(&self, p: &str) -> Vec<String> {
            if p.contains("mitigated") {
                vec![]
            } else {
                vec!["missing risk mitigation".into()]
            }
        }
    }

    /// A mirror that objects to everything (forces the least-friction tiebreak).
    struct StubbornMirror;
    impl Mirror for StubbornMirror {
        fn critique(&self, _p: &str) -> Vec<String> {
            vec!["still wrong".into(), "incomplete".into()]
        }
    }

    // (1) agreement on lap 1 — no dialogue needed.
    #[test]
    fn agreement_lap1_no_objections() {
        let c = deliberate("idea A", &AgreeingMirror, |p, _| String::from(p));
        assert_eq!(c.outcome, Outcome::Agreed);
        assert_eq!(c.laps, 1);
        assert_eq!(c.adopted, "idea A");
    }

    // (2) author reconciles, mirror agrees on lap 2.
    #[test]
    fn reconciliation_converges_lap2() {
        let c = deliberate("ship it", &SatisfiableMirror, |_, _| {
            String::from("ship it (mitigated)")
        });
        assert_eq!(c.outcome, Outcome::Agreed);
        assert_eq!(c.laps, 2);
        assert_eq!(c.adopted, "ship it (mitigated)");
    }

    // (3) lap cap without agreement → least-friction adopted. Since reconcile
    //     did NOT reduce objections, the ORIGINAL stands (not the failed revision).
    #[test]
    fn lap_cap_least_friction_original_wins() {
        let c = deliberate("original plan", &StubbornMirror, |_, _| {
            String::from("revised plan")
        });
        assert_eq!(c.outcome, Outcome::LeastFrictionAdopted);
        assert_eq!(c.laps, 2, "must run exactly 2 laps, never a 3rd");
        assert_eq!(c.adopted, "original plan", "failed reconcile must not win");
    }

    // (4) lap cap, but reconcile DID reduce objections → revised (less friction) adopted.
    #[test]
    fn lap_cap_least_friction_revision_wins_when_better() {
        // Stubborn on original, but a reconcile that halves objections.
        struct HalfMirror;
        impl Mirror for HalfMirror {
            fn critique(&self, p: &str) -> Vec<String> {
                if p.contains("v2") {
                    vec!["minor".into()]
                } else {
                    vec!["a".into(), "b".into()]
                }
            }
        }
        let c = deliberate("plan v0", &HalfMirror, |_, _| String::from("plan v2"));
        assert_eq!(c.outcome, Outcome::LeastFrictionAdopted);
        assert_eq!(
            c.adopted, "plan v2",
            "revision with fewer open objections wins"
        );
    }

    // (5) structural guarantee: transcript never exceeds the 2-lap exchange count.
    #[test]
    fn never_exceeds_two_laps() {
        let c = deliberate("x", &StubbornMirror, |p, _| String::from(p));
        // author + mirror (lap1) + author-reconcile + mirror (lap2) = 4 utterances.
        assert!(c.transcript.len() <= 4, "max 4 utterances across 2 laps");
    }
}
