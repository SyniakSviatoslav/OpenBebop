//! zkVM boundary — deterministic, verifiable state-transition commitment.
//!
//! Replaces the Research-slot "zkVM boundary" as real, tested Rust. This is an
//! HONEST prototype: a deterministic boundary that, given (prev_state, input),
//! produces (next_state, receipt) where `receipt = H(prev_state || input ||
//! next_state || meta)`. `verify(receipt, prev, input, next)` recomputes the
//! hash and checks equality — a falsifiable integrity claim over a boundary
//! crossing (e.g. "this state change was authorized and is tamper-evident").
//!
//! It is NOT a full zero-knowledge proof system (no RISC Zero / no circuit). It
//! is the *shape* of the boundary: commit → cross → verify, with a RED case that
//! fails on tampered output. The seam where a real zkVM proof would slot in is
//! `verify()` — swap the hash check for a proof verification. NO rng, NO clock.

use sha2::{Digest, Sha256};

/// A state is an opaque byte blob (e.g. a serialized ledger snapshot).
pub type State = Vec<u8>;

/// A receipt commits to a boundary crossing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Receipt {
    pub prev: Vec<u8>,
    pub input: Vec<u8>,
    pub next: Vec<u8>,
    pub meta: Vec<u8>,
    pub seal: String, // = H(prev || input || next || meta)
    /// Structured proof slot. In the honest prototype this carries a
    /// deterministic mock receipt (a STARK-shaped journal string) so the
    /// verify seam is exercised end-to-end without the risc0 prover. A real
    /// deployment swaps `verify()` to check an actual STARK proof bytes here.
    pub proof: Proof,
}

/// The proof payload. `Mock` is the in-repo falsifiable stand-in; `Stark`
/// is the shape a real RISC Zero receipt would take (opaque bytes, checked
/// by an injected verifier — never fabricated in this crate).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Proof {
    /// Deterministic mock: the sealed journal string (same material as `seal`,
    /// so verify can recompute it and confirm the boundary is self-consistent).
    Mock(String),
    /// Real STARK receipt bytes. Verification is delegated to a caller-supplied
    /// closure (the prover/verifier lives outside the deterministic core).
    Stark(Vec<u8>),
}

fn seal(prev: &[u8], input: &[u8], next: &[u8], meta: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(prev);
    h.update(input);
    h.update(next);
    h.update(meta);
    let d = h.finalize();
    d.iter().map(|b| format!("{b:02x}")).collect()
}

/// Apply a pure transition `f` at the boundary, returning the next state and a
/// receipt that commits to (prev, input, next, meta). The receipt also carries a
/// deterministic mock proof so the verify seam is exercisable without risc0.
pub fn cross<F>(prev: &[u8], input: &[u8], meta: &[u8], f: F) -> (State, Receipt)
where
    F: Fn(&[u8], &[u8]) -> State,
{
    let next = f(prev, input);
    let seal = seal(prev, input, &next, meta);
    let proof = Proof::Mock(seal.clone());
    let r = Receipt {
        prev: prev.to_vec(),
        input: input.to_vec(),
        next: next.clone(),
        meta: meta.to_vec(),
        seal: seal.clone(),
        proof,
    };
    (next, r)
}

/// Verify a receipt: recompute the seal and check it binds prev/input/next/meta.
/// Returns false if any field was tampered (RED case).
///
/// For `Proof::Mock` the embedded journal is re-derived and must match `seal`
/// (the deterministic in-core check). For `Proof::Stark` the caller MUST supply
/// a verifier closure — without it the boundary fails closed (returns false),
/// never claiming a proof we cannot produce.
pub fn verify(r: &Receipt) -> bool {
    verify_with(r, None)
}

/// Verify with an optional STARK verifier. Honest seam: if the proof is `Stark`
/// and no verifier is provided, this returns false (fail-closed); we never
/// fabricate a successful verification. The verifier is HRTB-bound so any
/// `&Receipt` lifetime is accepted.
pub fn verify_with(
    r: &Receipt,
    stark_verify: Option<&dyn for<'a> Fn(&'a Receipt) -> bool>,
) -> bool {
    let recomputed = seal(&r.prev, &r.input, &r.next, &r.meta);
    if recomputed != r.seal {
        return false; // tampered core fields
    }
    match &r.proof {
        Proof::Mock(journal) => journal == &recomputed, // self-consistent mock
        Proof::Stark(_) => match stark_verify {
            Some(v) => v(r),
            None => false, // fail closed: cannot verify a real proof we don't have
        },
    }
}

/// Convenience: verify AND bind a specific expected `next` (caller knows the
/// post-condition they require of the boundary).
pub fn verify_expect(r: &Receipt, expect_next: &[u8]) -> bool {
    verify(r) && r.next == expect_next
}

#[cfg(test)]
mod tests {
    use super::*;

    // a trivial deterministic transition: append input to prev
    fn append(prev: &[u8], input: &[u8]) -> State {
        let mut v = prev.to_vec();
        v.extend_from_slice(input);
        v
    }

    #[test]
    fn cross_then_verify_green() {
        // GREEN: a legit crossing verifies.
        let (next, r) = cross(b"ledger-v1", b"+100", b"credit", append);
        assert_eq!(next, b"ledger-v1+100".to_vec());
        assert!(verify(&r), "valid receipt failed verification");
        assert!(
            verify_expect(&r, b"ledger-v1+100"),
            "expected next mismatch"
        );
        // the mock proof is self-consistent
        assert!(matches!(r.proof, Proof::Mock(_)));
    }

    #[test]
    fn tampered_next_fails() {
        // RED: if the recorded `next` is changed after the fact, verify fails.
        let (_next, mut r) = cross(b"ledger-v1", b"+100", b"credit", append);
        r.next = b"ledger-v1-999".to_vec(); // tamper
        assert!(!verify(&r), "tampered receipt verified (should fail)");
    }

    #[test]
    fn tampered_seal_fails() {
        // RED: a forged seal (without knowing the transition) fails.
        let (next, mut r) = cross(b"ledger-v1", b"+100", b"credit", append);
        // attacker tries to claim a different input but keeps old seal
        r.input = b"-999".to_vec();
        assert!(!verify(&r), "forged seal verified");
        // and the legit next (from the original crossing) is independent of the forged input
        assert_eq!(next, b"ledger-v1+100".to_vec());
    }

    #[test]
    fn determinism_same_in_same_out() {
        // GREEN: same inputs → same receipt seal (deterministic, replayable).
        let (_, r1) = cross(b"x", b"y", b"m", append);
        let (_, r2) = cross(b"x", b"y", b"m", append);
        assert_eq!(r1.seal, r2.seal);
    }

    #[test]
    fn stark_proof_fails_closed_without_verifier() {
        // RED (honest seam): a real STARK proof cannot be "verified" by the
        // in-core hash check — without an injected verifier we fail closed.
        let (_next, mut r) = cross(b"ledger-v1", b"+100", b"credit", append);
        r.proof = Proof::Stark(vec![0xDE, 0xAD, 0xBE, 0xEF]);
        fn always(_: &Receipt) -> bool {
            true
        }
        fn never(_: &Receipt) -> bool {
            false
        }
        assert!(
            !verify(&r),
            "stark proof verified without a verifier — false green!"
        );
        // but with a correct verifier it can pass
        assert!(
            verify_with(&r, Some(&always)),
            "verifier rejected valid stark"
        );
        // and a lying verifier is the caller's responsibility, not ours to fake
        assert!(!verify_with(&r, Some(&never)), "false verifier accepted");
    }
}
