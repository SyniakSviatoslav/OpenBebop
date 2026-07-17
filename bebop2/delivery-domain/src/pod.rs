//! P13 · §4 Proof-of-Delivery port (AC-3).
//!
//! A **DeliveryClaim** is a signed attestation that a courier physically
//! delivered order `order_id` at `location` at time `timestamp`. It settles the
//! delivery leg (the payout saga, built separately under P07, then consumes the
//! settled claim) only when **k-of-n** distinct hubs have each signed the SAME
//! canonical digest.
//!
//! # Design (explicit DECART — NOT FROST)
//! The blueprint §4 mandates an explicit k-of-n threshold over **distinct
//! hybrid (Ed25519 + ML-DSA-65) signatures on one shared digest** — deliberately
//! NOT FROST (no distributed-key ceremony, no extra dependency). Each signer
//! produces an ordinary two-leg hybrid signature over
//! `sha3_256(canonical_claim_bytes)`. Settlement = at least `k` signatures from
//! `k` *distinct* pubkeys, all verifying, all over the same digest.
//!
//! # Canonical + tamper/misattribution/wrong-location refusal
//! - Signing commits to `order_id || location || timestamp` ONLY via a fixed
//!   field encoding (no serde). Re-encoding is byte-stable, so two peers verify
//!   identically.
//! - A signature over a *different* claim (tampered order/location/time) does
//!   NOT verify against this claim's digest → rejected.
//! - A signature by a key that is NOT one of the `n` enrolled hub keys
//!   (misattribution — a non-hub forging a delivery) is rejected even if it
//!   verifies cryptographically, because the signer must be in the enrolled set.
//! - A claim at the wrong `location` is just a different claim (different
//!   digest) → its signatures won't match, so it cannot reach threshold.
//!
//! Zero new dependencies: reuses `bebop2_core::sign` (Ed25519), `pq_dsa`
//! (ML-DSA-65), and `bebop2_core::hash::sha3_256`. No tonic/prost.

use bebop2_core::hash::sha3_256;
use bebop2_core::pq_dsa::{keygen, keygen_derivable, sign as pq_sign, verify as pq_verify};
use bebop2_core::sign::{keygen as ed_keygen, sign as ed_sign, verify as ed_verify};

/// Fixed-layout canonical bytes for a delivery claim. Field order is pinned; no
/// serde, no float, no map — re-encoding is byte-identical across nodes.
fn canonical_claim(order_id: u64, location: &[u8], timestamp: u64) -> Vec<u8> {
    let mut b = Vec::with_capacity(8 + 8 + location.len());
    b.extend_from_slice(&order_id.to_le_bytes());
    b.extend_from_slice(&(location.len() as u64).to_le_bytes());
    b.extend_from_slice(location);
    b.extend_from_slice(&timestamp.to_le_bytes());
    b
}

/// A single hybrid (classical + PQ) signature over the claim digest.
#[derive(Debug, Clone)]
pub struct HybridSig {
    pub classical: [u8; 64],
    pub pq: Vec<u8>,
}

/// A signer's enrolled identity: classical Ed25519 pubkey + PQ ML-DSA-65 pubkey.
#[derive(Debug, Clone)]
pub struct HubSigner {
    pub classical_pk: [u8; 32],
    pub pq_pk: Vec<u8>,
}

/// A Proof-of-Delivery claim before/after signatures.
#[derive(Debug, Clone)]
pub struct DeliveryClaim {
    pub order_id: u64,
    pub location: Vec<u8>,
    pub timestamp: u64,
    /// Enrolled hub signers (the `n` in k-of-n). A signature counts only if its
    /// pubkey is in this set (misattribution guard).
    pub signers: Vec<HubSigner>,
    /// Threshold `k` (must satisfy `1 <= k <= signers.len()`).
    pub threshold: usize,
    /// Collected hybrid signatures, keyed by signer index into `signers`.
    pub sigs: Vec<(usize, HybridSig)>,
}

impl DeliveryClaim {
    /// Build a fresh, unsigned claim. `signers` is the enrolled hub set;
    /// `threshold` is `k`.
    pub fn new(
        order_id: u64,
        location: Vec<u8>,
        timestamp: u64,
        signers: Vec<HubSigner>,
        threshold: usize,
    ) -> Self {
        assert!(threshold >= 1 && threshold <= signers.len(), "k in [1, n]");
        DeliveryClaim {
            order_id,
            location,
            timestamp,
            signers,
            threshold,
            sigs: Vec::new(),
        }
    }

    /// The canonical digest every signer commits to.
    pub fn digest(&self) -> [u8; 32] {
        sha3_256(&canonical_claim(self.order_id, &self.location, self.timestamp))
    }

    /// Add one hybrid signature from signer `idx` (0-based into `signers`).
    /// Returns false if the index is out of range.
    pub fn add_sig(&mut self, idx: usize, sig: HybridSig) -> bool {
        if idx >= self.signers.len() {
            return false;
        }
        // Replace any prior signature from this signer (dedup by index).
        self.sigs.retain(|(i, _)| *i != idx);
        self.sigs.push((idx, sig));
        true
    }

    /// Verify a single hybrid signature set against the claim digest and the
    /// signer's enrolled pubkeys.
    fn verify_one(&self, idx: usize, sig: &HybridSig) -> bool {
        let s = match self.signers.get(idx) {
            Some(s) => s,
            None => return false,
        };
        let d = self.digest();
        ed_verify(&s.classical_pk, &d, &sig.classical)
            && pq_verify(
                &pq_pk_from_bytes(&s.pq_pk),
                &d,
                &pq_sig_from_bytes(&sig.pq),
            )
    }

    /// Count of *distinct* signers whose hybrid signature verifies over this
    /// claim's digest and against the enrolled key set.
    pub fn valid_signers(&self) -> usize {
        self.sigs
            .iter()
            .filter(|(idx, sig)| self.verify_one(*idx, sig))
            .map(|(idx, _)| *idx)
            .collect::<std::collections::HashSet<_>>()
            .len()
    }

    /// Settled iff at least `k` DISTINCT enrolled signers have verified
    /// hybrid signatures over this exact claim.
    pub fn is_settled(&self) -> bool {
        self.valid_signers() >= self.threshold
    }

    /// The current signature count (raw, before verification).
    pub fn sig_count(&self) -> usize {
        self.sigs.len()
    }

    /// Tamper evidence: returns true if ANY collected signature fails to verify
    /// against the *current* claim digest — i.e. the claim or a signature was
    /// mutated after signing.
    pub fn any_tampered(&self) -> bool {
        self.sigs
            .iter()
            .any(|(idx, sig)| !self.verify_one(*idx, sig))
    }
}

/// Produce a hybrid signature over `digest` from a signer holding the given
/// seeds. `ed_seed` derives the Ed25519 keypair; `pq_seed` derives the
/// ML-DSA-65 keypair. Returns the signer's enrolled identity plus the signature.
pub fn sign_claim(
    ed_seed: &[u8; 32],
    pq_seed: &[u8; 32],
    digest: &[u8; 32],
) -> (HubSigner, HybridSig) {
    let (classical_pk, _) = ed_keygen(ed_seed);
    let (pq_pk, pq_sk) = keygen_derivable(pq_seed);
    let classical = ed_sign(ed_seed, digest);
    let pq_sig = pq_sign(&pq_sk, digest, &[0u8; 32]);
    (
        HubSigner {
            classical_pk,
            pq_pk: pq_pk.bytes.to_vec(),
        },
        HybridSig {
            classical,
            pq: pq_sig.bytes.to_vec(),
        },
    )
}

// ── helpers to convert between bebop2_core pq_dsa typed keys/sigs and bytes ──
fn pq_pk_from_bytes(b: &[u8]) -> bebop2_core::pq_dsa::MlDsa65Pk {
    let mut a = [0u8; 1952];
    a.copy_from_slice(&b[..1952]);
    bebop2_core::pq_dsa::MlDsa65Pk { bytes: a.to_vec() }
}
fn pq_sig_from_bytes(b: &[u8]) -> bebop2_core::pq_dsa::MlDsa65Sig {
    let mut a = [0u8; 3309];
    a.copy_from_slice(&b[..3309]);
    bebop2_core::pq_dsa::MlDsa65Sig { bytes: a.to_vec() }
}


#[cfg(test)]
mod tests {
    use super::*;

    /// Build `n` distinct enrolled hub signers (real Ed25519 + ML-DSA-65 keys).
    /// Returns the signers plus each signer's seeds (for signing later).
    fn make_hubs(n: u8) -> (Vec<HubSigner>, Vec<([u8; 32], [u8; 32])>) {
        let mut signers = Vec::new();
        let mut seeds = Vec::new();
        for i in 0..n {
            let ed_seed = [i; 32];
            let pq_seed = [i.wrapping_mul(7).wrapping_add(3); 32];
            let (classical_pk, _) = ed_keygen(&ed_seed);
            let (pq_pk, _) = keygen_derivable(&pq_seed);
            signers.push(HubSigner {
                classical_pk,
                pq_pk: pq_pk.bytes.clone(),
            });
            seeds.push((ed_seed, pq_seed));
        }
        (signers, seeds)
    }

    fn claim(n: u8, k: usize, loc: &[u8], ts: u64) -> (DeliveryClaim, Vec<([u8; 32], [u8; 32])>) {
        let (signers, seeds) = make_hubs(n);
        let c = DeliveryClaim::new(777, loc.to_vec(), ts, signers, k);
        (c, seeds)
    }

    // ── AC-3 GREEN: a valid k-of-n (here 2-of-3) claim SETTLES.
    #[test]
    fn ac3_k_of_n_valid_settles() {
        let (mut c, seeds) = claim(3, 2, b"locker-12", 1_700_000_000);
        let d = c.digest();
        for i in 0..2 {
            let (_, sig) = sign_claim(&seeds[i].0, &seeds[i].1, &d);
            assert!(c.add_sig(i, sig));
        }
        assert!(c.is_settled(), "2-of-3 should settle");
        assert_eq!(c.valid_signers(), 2);
    }

    // ── AC-3 RED: a claim with fewer than k distinct signers does NOT settle.
    #[test]
    fn ac3_below_threshold_does_not_settle() {
        let (mut c, seeds) = claim(3, 3, b"locker-12", 1_700_000_000); // need 3
        let d = c.digest();
        let (_, sig0) = sign_claim(&seeds[0].0, &seeds[0].1, &d);
        let (_, sig1) = sign_claim(&seeds[1].0, &seeds[1].1, &d);
        c.add_sig(0, sig0);
        c.add_sig(1, sig1);
        assert!(!c.is_settled(), "2-of-3 must NOT settle");
    }

    // ── AC-3 RED: a DUPLICATE signer (same hub signs twice) counts once, so
    // k distinct is enforced — a single hub cannot fake a quorum.
    #[test]
    fn ac3_duplicate_signer_counts_once() {
        let (mut c, seeds) = claim(3, 2, b"locker-12", 1_700_000_000);
        let d = c.digest();
        // Hub 0 signs twice (re-add replaces).
        let (_, s0a) = sign_claim(&seeds[0].0, &seeds[0].1, &d);
        let (_, s0b) = sign_claim(&seeds[0].0, &seeds[0].1, &d);
        c.add_sig(0, s0a);
        c.add_sig(0, s0b);
        let (_, s1) = sign_claim(&seeds[1].0, &seeds[1].1, &d);
        c.add_sig(1, s1);
        assert_eq!(c.valid_signers(), 2, "two distinct hubs");
        assert!(c.is_settled());

        // Now ONLY hub 0, twice — must NOT reach threshold of 2 distinct.
        let (mut c2, seeds2) = claim(3, 2, b"locker-12", 1_700_000_000);
        let d2 = c2.digest();
        let (_, s) = sign_claim(&seeds2[0].0, &seeds2[0].1, &d2);
        c2.add_sig(0, s.clone());
        c2.add_sig(0, s.clone()); // re-add, still one distinct signer
        assert_eq!(c2.valid_signers(), 1);
        assert!(!c2.is_settled(), "one distinct hub cannot fake 2-of-n");
    }

    // ── AC-3 RED: TAMPERED claim (location changed after signing) does NOT
    // verify — the signatures no longer match the (new) digest.
    #[test]
    fn ac3_tampered_location_rejected() {
        let (mut c, seeds) = claim(3, 2, b"locker-12", 1_700_000_000);
        let d = c.digest();
        for i in 0..2 {
            let (_, sig) = sign_claim(&seeds[i].0, &seeds[i].1, &d);
            c.add_sig(i, sig);
        }
        assert!(c.is_settled());

        // Attacker rewrites the location on the (already-signed) claim.
        c.location = b"wrong-locker".to_vec();
        // The digest changed, so every previously-valid sig now fails.
        assert!(c.any_tampered(), "signatures must not verify after tamper");
        assert_eq!(c.valid_signers(), 0);
        assert!(!c.is_settled());
    }

    // ── AC-3 RED: TAMPERED claim (timestamp changed) is similarly rejected.
    #[test]
    fn ac3_tampered_timestamp_rejected() {
        let (mut c, seeds) = claim(3, 2, b"locker-12", 1_700_000_000);
        let d = c.digest();
        for i in 0..2 {
            let (_, sig) = sign_claim(&seeds[i].0, &seeds[i].1, &d);
            c.add_sig(i, sig);
        }
        assert!(c.is_settled());
        c.timestamp = 1_700_000_999;
        assert!(c.any_tampered());
        assert!(!c.is_settled());
    }

    // ── AC-3 RED: MISATTRIBUTION — a signature by a NON-enrolled key (a stranger
    // masquerading as a hub) is rejected even though it verifies cryptographically,
    // because the signer is not in the enrolled set.
    #[test]
    fn ac3_misattributed_signer_rejected() {
        let (mut c, _seeds) = claim(3, 2, b"locker-12", 1_700_000_000);
        let d = c.digest();
        // Stranger signs over the real digest with their own (valid) key.
        let stranger_ed = [0xABu8; 32];
        let stranger_pq = [0xCDu8; 32];
        let (_, sig) = sign_claim(&stranger_ed, &stranger_pq, &d);
        // idx 0 is a legitimate enrolled hub, but the signature is by the
        // stranger's key, so verify_one (which checks against enrolled pubkey)
        // returns false.
        c.add_sig(0, sig);
        assert_eq!(c.valid_signers(), 0, "misattributed sig must not count");
        assert!(!c.is_settled());
    }

    // ── AC-3 GREEN: signature is bound to the digest — signing a DIFFERENT
    // claim's digest and attaching it here fails (cross-claim reuse rejected).
    #[test]
    fn ac3_wrong_digest_rejected() {
        let (mut c, seeds) = claim(3, 2, b"locker-12", 1_700_000_000);
        // Signer signs a different digest (e.g. a different order/claim).
        let wrong_digest = sha3_256(b"some other claim");
        let (_, sig) = sign_claim(&seeds[0].0, &seeds[0].1, &wrong_digest);
        c.add_sig(0, sig);
        assert_eq!(c.valid_signers(), 0, "sig over wrong digest rejected");
        assert!(!c.is_settled());
    }

    // ── AC-3 GREEN: canonical bytes are byte-stable (re-encoding identical), so
    // the digest a signer computes equals the digest a verifier computes.
    #[test]
    fn ac3_canonical_bytes_stable() {
        let a = canonical_claim(777, b"locker-12", 1_700_000_000);
        let b = canonical_claim(777, b"locker-12", 1_700_000_000);
        assert_eq!(a, b);
        assert_eq!(sha3_256(&a), sha3_256(&b));
        // Different location -> different digest.
        let c = canonical_claim(777, b"locker-13", 1_700_000_000);
        assert_ne!(sha3_256(&a), sha3_256(&c));
    }
}
