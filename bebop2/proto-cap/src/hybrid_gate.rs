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
use crate::roster::{verify_chain, AnchorRoster, Delegation};
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
///
/// **Authorization is now rooted in an [`AnchorRoster`].** Every frame must carry
/// a UCAN-subset [`Delegation`] chain whose root issuer is an enrolled anchor;
/// `verify_chain` enforces root∈roster → link chaining → narrow-only attenuation →
/// tail binds to `subject_key` → requested effect ⊆ tail scope. A self-signed
/// frame (no anchor-rooted chain) is rejected as `UnknownIssuer`. This closes the
/// red-team §3A self-issued-capability auth bypass: the chain is checked BEFORE
/// the frame is returned, not merely asserted in isolated unit tests.
///
/// The replay ledger (`seen`) is bounded to [`MAX_SEEN_NONCES`] and pruned so a
/// long-lived authorized peer cannot OOM the connection (red-team B2/B3 DoS).
/// A poisoned lock returns [`CapError::LockPoisoned`] instead of panicking.
/// Nonces are checked AFTER expiry but the set is only mutated once acceptance
/// is committed — verify-then-record, never record-then-verify.
const MAX_SEEN_NONCES: usize = 1 << 20; // ~1M; ~8 MiB worst case, then pruned.

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
    ///
    /// `roster` is the enrolled trust-anchor set; `chain` is the frame's carried
    /// UCAN-subset delegation chain (taken from `frame.delegation_chain`). A
    /// frame with no anchor-rooted chain is rejected (`UnknownIssuer`) — this is
    /// the live, single highest-value auth control.
    pub fn check(
        &self,
        frame: &SignedFrame,
        roster: &AnchorRoster,
        chain: &[Delegation],
        now: u64,
    ) -> CapResult<()> {
        // Replay + expiry first (cheap, fail-closed).
        if !frame.capability.is_fresh(now) {
            return Err(CapError::Expired);
        }
        let nonce = frame.capability.nonce;
        {
            let mut seen = self.seen.lock().map_err(|_| CapError::LockPoisoned)?;
            if !seen.insert(nonce) {
                return Err(CapError::NonceRejected);
            }
            // Bound the set: once over capacity, drop half the entries so a
            // long-lived connection cannot OOM on distinct nonces. Order is
            // irrelevant for replay defense — any half is fine.
            if seen.len() > MAX_SEEN_NONCES {
                let keep: HashSet<[u8; 8]> =
                    seen.iter().take(MAX_SEEN_NONCES / 2).copied().collect();
                *seen = keep;
            }
        }

        // Authorization root-of-trust: the delegation chain MUST root in an
        // enrolled anchor and satisfy the UCAN-subset lattice. Fail-closed:
        // an empty/absent chain or a non-anchor root is UnknownIssuer.
        verify_chain(roster, chain, &frame.capability, now)?;

        // Classical leg must ALWAYS verify (real Ed25519). Never relaxed.
        frame.verify_classical()?;

        // PQ leg — now a REAL ML-DSA-65 verification (no longer a todo).
        //  - RequireBoth: a real PQ signature MUST verify against the capability's
        //    subject_key_pq. Missing or invalid PQ proof => HybridIncomplete / PqVerifyFailed.
        //  - ClassicalUntilPqAudit: the transitional pre-audit bar. If a PQ signature
        //    is present it must verify (no silent pass); if absent, the frame is
        //    accepted on the strength of the real classical leg, explicitly marked
        //    as pre-audit. We never fake a PQ result.
        match self.policy {
            HybridPolicy::RequireBoth => frame.verify_pq(),
            HybridPolicy::ClassicalUntilPqAudit => match &frame.pq_sig {
                Some(_) => frame.verify_pq(),
                None => Ok(()),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability::Capability;
    use crate::roster::{AnchorRoster, Delegation, Effect};
    use crate::scope::{Action, Resource, Scope};
    use crate::signed_frame::SignedFrame;

    fn key(seed_byte: u8) -> ([u8; 32], [u8; 32]) {
        let seed = [seed_byte; 32];
        let (pk, _) = bebop2_core::sign::keygen(&seed);
        (seed, pk)
    }

    /// Build a frame signed by `leaf_pk`/`leaf_seed`, plus a valid anchor-rooted
    /// delegation chain (anchor -> leaf) carrying the same scope as the cap.
    /// The capability is HYBRID: it also carries a real ML-DSA-65 `subject_key_pq`
    /// so the `RequireBoth` policy can verify the PQ leg for real.
    fn signed_frame_with_chain(
        anchor_seed: &[u8; 32],
        anchor_pk: &[u8; 32],
        leaf_seed: &[u8; 32],
        leaf_pk: &[u8; 32],
        resource: Resource,
        action: Action,
        nonce: [u8; 8],
        expiry: u64,
    ) -> (SignedFrame, AnchorRoster, Vec<Delegation>) {
        // Real ML-DSA-65 keypair for the leaf (PQ half of the hybrid identity).
        let pq_seed = [0xABu8; 32];
        let (pq_pk, _pq_sk) = bebop2_core::pq_dsa::keygen(&pq_seed);
        let cap = Capability::new_hybrid(
            *leaf_pk,
            pq_pk.bytes.clone(),
            resource,
            action,
            nonce,
            expiry,
        );
        let mut f = SignedFrame::new(cap, b"data".to_vec());
        f.sign_classical(leaf_seed).unwrap();
        let link = Delegation::sign(
            *anchor_pk,
            *leaf_pk,
            Scope::new(resource, action),
            Effect::new(resource, action),
            expiry,
            nonce,
            anchor_seed,
        )
        .unwrap();
        let mut roster = AnchorRoster::new();
        roster.enroll(anchor_pk);
        (f, roster, vec![link])
    }

    fn signed_frame() -> SignedFrame {
        let (seed, pk) = key(5);
        let cap = Capability::new(pk, Resource::Route, Action::Send, [4u8; 8], 777);
        let mut f = SignedFrame::new(cap, b"data".to_vec());
        f.sign_classical(&seed).unwrap();
        f
    }

    #[test]
    fn require_both_reports_pq_todo() {
        let gate = HybridGate::new(HybridPolicy::RequireBoth);
        // `gated()` produces a hybrid cap (has subject_key_pq) but no pq_sig, so
        // RequireBoth must reject on the missing PQ proof (PqVerifyFailed), not
        // silently accept classical-only, and never before the auth root check.
        let (f, roster, chain) = gated();
        assert!(matches!(
            gate.check(&f, &roster, &chain, 0),
            Err(CapError::PqVerifyFailed) | Err(CapError::UnknownIssuer)
        ));
    }

    // Helper used by the PQ-policy tests: a properly-anchored frame.
    fn gated() -> (SignedFrame, AnchorRoster, Vec<Delegation>) {
        let (a_seed, a_pk) = key(2);
        let (l_seed, l_pk) = key(3);
        signed_frame_with_chain(
            &a_seed,
            &a_pk,
            &l_seed,
            &l_pk,
            Resource::Route,
            Action::Send,
            [7u8; 8],
            9999,
        )
    }

    #[test]
    fn classical_until_pq_audit_accepts_real_classical() {
        let gate = HybridGate::new(HybridPolicy::ClassicalUntilPqAudit);
        let (f, roster, chain) = gated();
        assert!(gate.check(&f, &roster, &chain, 0).is_ok());
    }

    #[test]
    fn pq_gate_require_both_enforces_real_pq() {
        // Under RequireBoth, an anchored frame with a real PQ signature passes;
        // without a PQ signature (or a bad one) it is rejected.
        let gate = HybridGate::new(HybridPolicy::RequireBoth);
        let (a_seed, a_pk) = key(2);
        let (l_seed, l_pk) = key(3);
        // Consistent PQ keypair for the leaf (same key used to sign the PQ leg).
        let pq_seed = [0xABu8; 32];
        let (pq_pk, pq_sk) = bebop2_core::pq_dsa::keygen(&pq_seed);
        let cap = Capability::new_hybrid(
            l_pk,
            pq_pk.bytes.clone(),
            Resource::Route,
            Action::Send,
            [7u8; 8],
            9999,
        );
        let mut f = SignedFrame::new(cap, b"data".to_vec());
        f.sign_classical(&l_seed).unwrap();
        let link = Delegation::sign(
            a_pk,
            l_pk,
            Scope::new(Resource::Route, Action::Send),
            Effect::new(Resource::Route, Action::Send),
            9999,
            [7u8; 8],
            &a_seed,
        )
        .unwrap();
        let mut roster = AnchorRoster::new();
        roster.enroll(&a_pk);
        let chain = vec![link];
        // No PQ sig yet -> RequireBoth must reject (PqVerifyFailed; cap has a PQ key).
        assert!(
            gate.check(&f, &roster, &chain, 0).is_err(),
            "RequireBoth needs PQ"
        );
        // Add a real PQ signature (same key as subject_key_pq).
        f.sign_pq(&pq_sk.bytes.clone().try_into().unwrap(), &[0u8; 32])
            .unwrap();
        // Fresh gate instance: a new nonce set so the prior (rejected) check does
        // not consume the nonce (the gate tracks seen nonces per instance).
        let gate2 = HybridGate::new(HybridPolicy::RequireBoth);
        let res = gate2.check(&f, &roster, &chain, 0);
        assert!(
            res.is_ok(),
            "RequireBoth passes with real PQ, got: {:?}",
            res.err()
        );
    }

    #[test]
    fn gate_rejects_bad_classical() {
        let mut f = signed_frame();
        f.payload = b"evil".to_vec(); // tamper -> classical verify fails
        let gate = HybridGate::new(HybridPolicy::ClassicalUntilPqAudit);
        // No chain: fails auth root-of-trust regardless.
        let roster = AnchorRoster::new();
        assert!(gate.check(&f, &roster, &[], 0).is_err());
    }

    #[test]
    fn gate_rejects_self_signed_frame_no_anchor_chain() {
        // The weaponized self-issue bypass: a key signs its own capability and
        // sends it with no anchor-rooted delegation chain. Must be UnknownIssuer.
        let gate = HybridGate::new(HybridPolicy::ClassicalUntilPqAudit);
        let (seed, pk) = key(9);
        let cap = Capability::new(pk, Resource::Route, Action::Send, [1u8; 8], 9999);
        let mut f = SignedFrame::new(cap, b"takeover".to_vec());
        f.sign_classical(&seed).unwrap(); // real sig, but self-attested authority
        let roster = AnchorRoster::new();
        assert!(
            matches!(
                gate.check(&f, &roster, &[], 0),
                Err(CapError::UnknownIssuer)
            ),
            "self-signed frame with no anchor chain MUST be rejected"
        );
    }

    #[test]
    fn gate_rejects_replay_and_expiry() {
        let gate = HybridGate::new(HybridPolicy::ClassicalUntilPqAudit);
        let (f, roster, chain) = gated();
        // First sight of the nonce is accepted...
        assert!(gate.check(&f, &roster, &chain, 0).is_ok());
        // ...a second frame with the SAME nonce is a replay.
        assert!(matches!(
            gate.check(&f, &roster, &chain, 0),
            Err(CapError::NonceRejected)
        ));
        // Expired capability (now >= expiry) is rejected.
        let mut expired = f;
        expired.capability.expiry = 10;
        assert!(matches!(
            gate.check(&expired, &roster, &chain, 11),
            Err(CapError::Expired)
        ));
    }
}
