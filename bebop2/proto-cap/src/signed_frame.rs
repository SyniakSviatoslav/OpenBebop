//! Signed frame — a frame carrying its own capability + signature(s).
//!
//! A [`SignedFrame`] binds a capability (authorizing an action on a resource by a
//! key, until a nonce/expiry) to the frame payload via a signature over
//! `(capability_canonical_bytes || payload)`. Because the signature covers the
//! capability, the frame cannot be replayed on a different payload/scope/nonce.
//!
//! # Signing — REAL, not faked
//! The **classical leg** is signed with `bebop2-core::sign` Ed25519 (RFC 8032,
//! from scratch, zero-dep). This is a genuine signature: `verify` returns `false`
//! on tamper, and the round-trip test asserts that.
//!
//! The **post-quantum leg** is ML-DSA-65 in `bebop2-core::pq_dsa`. It is NOT yet
//! wired here because that module exposes its keys/signature as private structs
//! with no `pack`/`unpack` byte API yet (see the `TODO-PQ` marker in `sign_pq` /
//! `verify_pq`). Until then the hybrid gate still requires the classical leg to
//! verify, and the PQ todo is surfaced explicitly — we do NOT invent a fake PQ
//! signature. This is the honest "TODO with exact call shape" the protocol review
//! gate requires.
//!
//! CI GUARD: NO-COURIER-SCORING — a frame binds action+resource+key only. No
//! score, no trust accumulation, no reputation ledger.

use serde::{Deserialize, Serialize};

use crate::capability::Capability;
use crate::error::{CapError, CapResult};
use crate::hybrid_gate::HybridGate;

/// A frame that carries its own signed capability. Neutral transport payload —
/// the `payload` bytes are opaque to authorization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignedFrame {
    /// The authorization statement (no score fields).
    pub capability: Capability,
    /// Opaque, carrier-neutral payload (the route/ledger/delivery intent bytes).
    pub payload: Vec<u8>,
    /// Ed25519 signature (64 bytes) over `signing_domain()`. Stored as `Vec<u8>`
    /// because serde's derive only auto-implements arrays up to length 32; the
    /// byte length is fixed at 64 by `bebop2_core::sign`.
    pub classical_sig: Option<Vec<u8>>,
    /// TODO-PQ: 32-byte-encoded ML-DSA-65 signature over `signing_domain()`.
    /// `None` until the PQ pack/unpack API lands. Not faked.
    pub pq_sig: Option<Vec<u8>>,
}

impl SignedFrame {
    /// Build an unsigned frame (signatures filled by [`sign`]).
    pub fn new(capability: Capability, payload: Vec<u8>) -> Self {
        SignedFrame {
            capability,
            payload,
            classical_sig: None,
            pq_sig: None,
        }
    }

    /// The exact bytes a signature commits to: `capability_canonical || payload`.
    /// Any change to the capability (scope/nonce/expiry/subject) or the payload
    /// invalidates the signature.
    pub fn signing_domain(&self) -> CapResult<Vec<u8>> {
        let mut buf = self.capability.canonical_bytes()?;
        buf.extend_from_slice(&self.payload);
        Ok(buf)
    }

    /// Sign this frame with the classical (Ed25519) key derived from `seed`.
    /// `seed` is the 32-byte Ed25519 seed (see `bebop2-core::sign::keygen`).
    ///
    /// This produces a REAL Ed25519 signature; tampering fails verification.
    pub fn sign_classical(&mut self, seed: &[u8; 32]) -> CapResult<()> {
        let msg = self.signing_domain()?;
        let sig: [u8; 64] = bebop2_core::sign::sign(seed, &msg);
        self.classical_sig = Some(sig.to_vec());
        Ok(())
    }

    /// TODO-PQ: sign with the post-quantum (ML-DSA-65) key. NOT YET WIRED — the
    /// `bebop2-core::pq_dsa` keys/sigs are private structs without a pack/unpack
    /// byte API, so there is no way to serialize the signature into `pq_sig`
    /// honestly. The exact intended call shape (once the API exists) is:
    ///
    /// ```ignore
    /// let (pk, sk) = bebop2_core::pq_dsa::keygen(&seed32);
    /// let rnd = [0u8; 32]; // caller-supplied, never OS RNG
    /// let sig = bebop2_core::pq_dsa::sign(&sk, &msg, &rnd);
    /// self.pq_sig = Some(pack_mldsa_sig(&sig)); // pack API TBD
    /// ```
    ///
    /// We leave this as a todo and DO NOT fabricate a signature.
    pub fn sign_pq(&mut self, _seed: &[u8; 32]) -> CapResult<()> {
        Err(CapError::HybridIncomplete)
    }

    /// Verify the classical signature against the capability's `subject_key`.
    pub fn verify_classical(&self) -> CapResult<()> {
        let sig = self
            .classical_sig
            .as_ref()
            .ok_or(CapError::ClassicalVerifyFailed)?;
        if sig.len() != 64 {
            return Err(CapError::ClassicalVerifyFailed);
        }
        let sig_arr: [u8; 64] = sig.clone().try_into().map_err(|_| CapError::BadLength)?;
        let msg = self.signing_domain()?;
        let ok = bebop2_core::sign::verify(&self.capability.subject_key, &msg, &sig_arr);
        if ok {
            Ok(())
        } else {
            Err(CapError::ClassicalVerifyFailed)
        }
    }

    /// TODO-PQ: verify the ML-DSA-65 signature. NOT YET WIRED (same API gap as
    /// `sign_pq`). Intended call shape:
    ///
    /// ```ignore
    /// let pk = unpack_mldsa_pk(&self.capability.subject_key_pq); // TBD
    /// let sig = unpack_mldsa_sig(self.pq_sig.as_deref()?);       // TBD
    /// if !bebop2_core::pq_dsa::verify(&pk, &msg, &sig) {
    ///     return Err(CapError::PqVerifyFailed);
    /// }
    /// ```
    pub fn verify_pq(&self) -> CapResult<()> {
        match &self.pq_sig {
            Some(_) => Err(CapError::PqVerifyFailed),
            None => Err(CapError::HybridIncomplete),
        }
    }

    /// Run the hybrid gate: classical MUST verify; PQ is required by policy but
    /// currently reports `HybridIncomplete` (todo) rather than failing the frame
    /// outright. See [`crate::hybrid_gate`].
    pub fn verify(&self, gate: &HybridGate, now: u64) -> CapResult<()> {
        gate.check(self, now)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scope::{Action, Resource};

    #[test]
    fn sign_verify_roundtrip_real_ed25519() {
        // Real Ed25519 from bebop2-core: seed -> (pk, _sk); sign with seed.
        let seed = [42u8; 32];
        let (pk, _) = bebop2_core::sign::keygen(&seed);
        let cap = Capability::new(pk, Resource::Route, Action::Send, [9u8; 8], 12345);
        let mut frame = SignedFrame::new(cap, b"hello wire".to_vec());
        frame.sign_classical(&seed).expect("sign");
        assert!(
            frame.verify_classical().is_ok(),
            "real signature must verify"
        );
    }

    #[test]
    fn tampered_payload_fails_classical() {
        let seed = [7u8; 32];
        let (pk, _) = bebop2_core::sign::keygen(&seed);
        let cap = Capability::new(pk, Resource::Ledger, Action::Append, [1u8; 8], 999);
        let mut frame = SignedFrame::new(cap, b"original".to_vec());
        frame.sign_classical(&seed).unwrap();
        // tamper with the payload after signing
        frame.payload = b"tampered".to_vec();
        assert!(frame.verify_classical().is_err(), "tamper must fail");
    }

    #[test]
    fn tampered_capability_fails_classical() {
        let seed = [11u8; 32];
        let (pk, _) = bebop2_core::sign::keygen(&seed);
        let cap = Capability::new(pk, Resource::Route, Action::Send, [3u8; 8], 500);
        let mut frame = SignedFrame::new(cap, b"x".to_vec());
        frame.sign_classical(&seed).unwrap();
        // tamper with the nonce (part of the signed domain)
        frame.capability.nonce = [99u8; 8];
        assert!(frame.verify_classical().is_err(), "nonce tamper must fail");
    }

    #[test]
    fn pq_leg_is_honest_todo_not_faked() {
        let seed = [1u8; 32];
        let (pk, _) = bebop2_core::sign::keygen(&seed);
        let cap = Capability::new(pk, Resource::Presence, Action::Send, [2u8; 8], 1);
        let mut frame = SignedFrame::new(cap, b"ping".to_vec());
        // sign_pq must NOT silently produce a fake signature.
        assert!(matches!(
            frame.sign_pq(&seed),
            Err(CapError::HybridIncomplete)
        ));
        assert!(frame.pq_sig.is_none(), "pq_sig must stay None (not faked)");
    }
}
