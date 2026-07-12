//! Hybrid gate — require BOTH a classical and a post-quantum signature.
//!
//! Per the bebop Tier-5 earn-it rule ("hybrid-only until audit"), a frame is
//! accepted only if it verifies under a classical scheme (Ed25519) AND under a
//! post-quantum scheme (ML-DSA-65). The classical leg is REAL (wired to
//! `bebop2-core::sign`). The PQ leg is a TODO pending the ML-DSA pack/unpack API
//! (see `signed_frame::{sign_pq,verify_pq}`); until then the gate reports
//! `HybridIncomplete` for the missing PQ proof rather than fabricating one.
//!
//! CI GUARD: NO-COURIER-SCORING — gating on signature validity, never on score.

use std::collections::HashSet;
use std::sync::Mutex;

use crate::error::{CapError, CapResult};
use crate::signed_frame::SignedFrame;

/// Policy for the hybrid gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HybridPolicy {
    /// Require the classical signature to verify AND the PQ signature to verify.
    /// (Current build: PQ is a TODO, so this always returns `HybridIncomplete`.)
    RequireBoth,
    /// Accept as soon as the classical signature verifies; record that PQ is
    /// still pending. Used during the pre-audit ramp (does NOT lower the bar on
    /// the classical leg — it still must be a real, valid Ed25519 signature).
    ClassicalUntilPqAudit,
}

/// The hybrid gate. Construct once with the policy; call [`HybridGate::check`] per
/// frame. Stateless re: trust/score — but it DOES track seen nonces to reject
/// replays (a single `Mutex<HashSet>`; in-process only, not a distributed
/// ledger — fine for the single-writer/pre-audit model).
#[derive(Debug)]
pub struct HybridGate {
    pub policy: HybridPolicy,
    /// Nonces already accepted this gate's lifetime. Dup = replay.
    seen: Mutex<HashSet<[u8; 8]>>,
}

impl HybridGate {
    /// Build a gate with the given policy.
    pub fn new(policy: HybridPolicy) -> Self {
        HybridGate {
            policy,
            seen: Mutex::new(HashSet::new()),
        }
    }

    /// Check a frame against the policy. `now` is the caller-supplied tick used
    /// for expiry (monotonic counter — no wall-clock dependency).
    /// The classical leg is always verified for real; the PQ leg status is
    /// reported honestly (todo = `HybridIncomplete`). Replays (dup nonce) and
    /// expired capabilities are rejected before the signature even matters.
    pub fn check(&self, frame: &SignedFrame, now: u64) -> CapResult<()> {
        // Replay + expiry first (cheap, fail-closed).
        if !frame.capability.is_fresh(now) {
            return Err(CapError::Expired);
        }
        let nonce = frame.capability.nonce;
        {
            let mut seen = self.seen.lock().expect("nonce set poisoned");
            if !seen.insert(nonce) {
                return Err(CapError::NonceRejected);
            }
        }

        // Classical leg must ALWAYS verify (real Ed25519). Never relaxed.
        frame.verify_classical()?;

        // PQ leg.
        match frame.pq_sig {
            Some(_) => {
                // A PQ signature is present (todo: real verify once pack/unpack lands).
                // Until then we mark it incomplete rather than claiming success.
                match self.policy {
                    HybridPolicy::RequireBoth | HybridPolicy::ClassicalUntilPqAudit => {
                        Err(CapError::HybridIncomplete)
                    }
                }
            }
            None => match self.policy {
                // RequireBoth: missing PQ proof -> incomplete (not faked, not silent).
                HybridPolicy::RequireBoth => Err(CapError::HybridIncomplete),
                // ClassicalUntilPqAudit: classical verified, PQ pending — frame accepted
                // at the pre-audit bar, explicitly (and only because classical is real).
                HybridPolicy::ClassicalUntilPqAudit => Ok(()),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability::Capability;
    use crate::scope::{Action, Resource};
    use crate::signed_frame::SignedFrame;

    fn signed_frame() -> SignedFrame {
        let seed = [5u8; 32];
        let (pk, _) = bebop2_core::sign::keygen(&seed);
        let cap = Capability::new(pk, Resource::Route, Action::Send, [4u8; 8], 777);
        let mut f = SignedFrame::new(cap, b"data".to_vec());
        f.sign_classical(&seed).unwrap();
        f
    }

    #[test]
    fn require_both_reports_pq_todo() {
        let gate = HybridGate::new(HybridPolicy::RequireBoth);
        assert!(matches!(
            gate.check(&signed_frame(), 0),
            Err(CapError::HybridIncomplete)
        ));
    }

    #[test]
    fn classical_until_pq_audit_accepts_real_classical() {
        let gate = HybridGate::new(HybridPolicy::ClassicalUntilPqAudit);
        assert!(gate.check(&signed_frame(), 0).is_ok());
    }

    #[test]
    fn gate_rejects_bad_classical() {
        let mut f = signed_frame();
        f.payload = b"evil".to_vec(); // tamper -> classical verify fails
        let gate = HybridGate::new(HybridPolicy::ClassicalUntilPqAudit);
        assert!(gate.check(&f, 0).is_err());
    }

    #[test]
    fn gate_rejects_replay_and_expiry() {
        let gate = HybridGate::new(HybridPolicy::ClassicalUntilPqAudit);
        // First sight of the nonce is accepted...
        assert!(gate.check(&signed_frame(), 0).is_ok());
        // ...a second frame with the SAME nonce is a replay.
        assert!(matches!(
            gate.check(&signed_frame(), 0),
            Err(CapError::NonceRejected)
        ));
        // Expired capability (now >= expiry) is rejected.
        let mut expired = signed_frame();
        expired.capability.expiry = 10;
        assert!(matches!(gate.check(&expired, 11), Err(CapError::Expired)));
    }
}
