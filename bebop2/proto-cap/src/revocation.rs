//! Revocation set — UCAN-style irreversible invalidation (MESH-11).
//!
//! Before this module, authorization was expiry-only: a capability could only
//! *expire* (its `nonce`/`expiry` window close). There was **no way to pull a
//! capability or a key out of circulation before its natural expiry** — the
//! single biggest authz hole in the line. If a key is compromised or a
//! capability is leaked, the attacker keeps full authority until the clock runs
//! out. Revocation closes that hole.
//!
//! A [`RevocationSet`] is an append-only, in-memory invalidate set: once a key
//! or a capability hash is added it can never be un-added (matches the UCAN
//! `revoke` semantic — revocation is monotonic and irreversible, never a
//! temporary suspension). A real mesh would gossip this set between peers so
//! every node converges on the same revoked set; this build provides the local
//! set plus a [`RevocationSet::merge`] method for anti-entropy (a peer pulls
//! another peer's revocations and folds them in).
//!
//! Hashes are computed with the in-tree `bebop2_core::hash::sha3_256` (FIPS 202,
//! KAT-green) — no new dependency, and the same zero-dep primitive the rest of
//! the line uses. [`revocation_hash`] hashes a capability's canonical TLV bytes
//! (`Capability::canonical_bytes_tlv`), so revoking a capability hits exactly
//! that capability's `(subject, scope, nonce, expiry)` tuple — a different
//! nonce yields a different hash, proving revocation is *surgical*, not blanket.
//!
//! CI GUARD: NO-COURIER-SCORING — revocation acts on public keys and capability
//! hashes (identities / statements), never on scores or reputation.

use std::collections::HashSet;

use bebop2_core::hash::sha3_256;

use crate::capability::Capability;
use crate::roster::AnchorRoster;

/// An append-only set of revoked identities.
///
/// Two namespaces are tracked independently:
/// - `revoked_keys` — 32-byte subject public keys (classical `subject_key`, or a
///   32-byte SHA3-256 *id* derived from the PQ `subject_key_pq`). Revoking a key
///   kills every capability ever minted to it, regardless of nonce/scope/expiry.
/// - `revoked_cap_hash` — 32-byte SHA3-256 hashes of a capability's canonical
///   TLV bytes (see [`revocation_hash`]). Revoking a single capability hash is
///   surgical: it only invalidates that exact `(subject, scope, nonce, expiry)`
///   statement, leaving sibling capabilities (same key, different nonce) valid.
///
/// Both sets are monotonic: `insert` only ever grows them. That is the UCAN
/// revoke model — there is deliberately no `unrevoke`.
#[derive(Debug, Clone, Default)]
pub struct RevocationSet {
    /// Revoked subject public keys (or PQ-key ids).
    revoked_keys: HashSet<[u8; 32]>,
    /// Revoked capability hashes (SHA3-256 over canonical TLV bytes).
    revoked_cap_hash: HashSet<[u8; 32]>,
}

impl RevocationSet {
    /// Empty revocation set. Populate with [`revoke_key`](Self::revoke_key) /
    /// [`revoke_capability`](Self::revoke_capability) (or fold in a peer's set
    /// with [`merge`](Self::merge)).
    pub fn new() -> Self {
        RevocationSet {
            revoked_keys: HashSet::new(),
            revoked_cap_hash: HashSet::new(),
        }
    }

    /// Irrevocably revoke a subject key (or PQ-key id). Every capability minted
    /// to this key is thereafter rejected by [`crate::hybrid_gate::HybridGate`].
    pub fn revoke_key(&mut self, key: [u8; 32]) {
        self.revoked_keys.insert(key);
    }

    /// Irrevocably revoke a single capability by its revocation hash (see
    /// [`revocation_hash`]). Surgical: only the exact capability statement whose
    /// canonical TLV hashes to `cap_hash` is invalidated.
    pub fn revoke_capability(&mut self, cap_hash: [u8; 32]) {
        self.revoked_cap_hash.insert(cap_hash);
    }

    /// Whether `key` has been revoked.
    pub fn is_revoked_key(&self, key: &[u8; 32]) -> bool {
        self.revoked_keys.contains(key)
    }

    /// Whether the capability whose revocation hash is `cap_hash` has been
    /// revoked.
    pub fn is_revoked_capability(&self, cap_hash: &[u8; 32]) -> bool {
        self.revoked_cap_hash.contains(cap_hash)
    }

    /// Anti-entropy: fold another peer's revocation set into this one. Union of
    /// both namespaces — monotonic, never removes entries. In a real mesh this
    /// is called after gossiping deltas so every node converges.
    pub fn merge(&mut self, other: &RevocationSet) {
        self.revoked_keys.extend(other.revoked_keys.iter().copied());
        self.revoked_cap_hash
            .extend(other.revoked_cap_hash.iter().copied());
    }

    /// Remove an anchor from the *enrolling roster* (drop-anchor, MESH-11).
    /// Revocation is **irreversible** for the revoked *set*, but an anchor that
    /// was merely enrolled (not yet revoked) can be dropped from the roster so
    /// it can no longer vouch. This is trivially local + fail-closed: dropping a
    /// key that was never enrolled is a no-op, not an error.
    pub fn drop_anchor(roster: &mut AnchorRoster, key: &[u8; 32]) {
        roster.remove(key);
    }

    /// Gossip this revocation set to a peer: serialize the two namespaces as
    /// sorted 32-byte id lists. The peer calls [`RevocationSet::merge`] to fold.
    /// innovate: consensus-grade propagation (Vouchsafe / Lingering-Authority,
    /// 2026 research-open) is a future upgrade; this is the anti-entropy
    /// primitive every node can run today.
    pub fn gossip_payload(&self) -> (Vec<[u8; 32]>, Vec<[u8; 32]>) {
        let mut keys: Vec<[u8; 32]> = self.revoked_keys.iter().copied().collect();
        let mut caps: Vec<[u8; 32]> = self.revoked_cap_hash.iter().copied().collect();
        keys.sort_unstable();
        caps.sort_unstable();
        (keys, caps)
    }
}

/// Compute the revocation hash of a capability: SHA3-256 over its canonical TLV
/// signing bytes ([`Capability::canonical_bytes_tlv`]). The hash is
/// deterministic and domain-separated from the signature domain — it identifies
/// the capability *statement* (subject, scope, nonce, expiry) for surgical
/// revocation. Two capabilities that differ only in nonce produce distinct
/// hashes, which is exactly what makes revocation selective rather than blanket.
pub fn revocation_hash(cap: &Capability) -> [u8; 32] {
    sha3_256(&cap.canonical_bytes_tlv())
}

/// Derive a stable 32-byte revocation id for a capability's post-quantum subject
/// key. The PQ key is a 1952-byte ML-DSA-65 public key (`Option<Vec<u8>>`), so we
/// cannot store it directly in the 32-byte `revoked_keys` set; we hash it down to
/// a 32-byte SHA3-256 id. Revoking the PQ leg therefore revokes by this id. (The
/// classical `subject_key` is already 32 bytes and is stored as-is.)
pub fn pq_key_id(subject_key_pq: &[u8]) -> [u8; 32] {
    sha3_256(subject_key_pq)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scope::{Action, Resource};

    #[test]
    fn revocation_hash_is_deterministic_and_nonce_sensitive() {
        let a = Capability::new([7u8; 32], Resource::Route, Action::Send, [1u8; 8], 9999);
        let b = Capability::new([7u8; 32], Resource::Route, Action::Send, [2u8; 8], 9999);
        assert_eq!(revocation_hash(&a), revocation_hash(&a), "stable");
        assert_ne!(
            revocation_hash(&a),
            revocation_hash(&b),
            "different nonce -> different revocation hash (surgical)"
        );
    }

    #[test]
    fn merge_union_both_namespaces() {
        let mut a = RevocationSet::new();
        a.revoke_key([1u8; 32]);
        let mut b = RevocationSet::new();
        b.revoke_key([2u8; 32]);
        b.revoke_capability([9u8; 32]);
        a.merge(&b);
        assert!(a.is_revoked_key(&[1u8; 32]));
        assert!(a.is_revoked_key(&[2u8; 32]));
        assert!(a.is_revoked_capability(&[9u8; 32]));
    }

    #[test]
    fn revoke_then_is_revoked_queries() {
        let mut rs = RevocationSet::new();
        let key = [0xABu8; 32];
        let cap_hash = [0xCDu8; 32];
        assert!(!rs.is_revoked_key(&key));
        assert!(!rs.is_revoked_capability(&cap_hash));
        rs.revoke_key(key);
        rs.revoke_capability(cap_hash);
        assert!(rs.is_revoked_key(&key));
        assert!(rs.is_revoked_capability(&cap_hash));
    }

    // RED — MESH-11 drop-anchor: removing an enrolled anchor from the roster
    // makes it unable to vouch (a delegation chain rooted at it is now rejected
    // as UnknownIssuer), while an un-enrolled key is a silent no-op.
    #[test]
    fn drop_anchor_removes_vouch_power() {
        let (a_seed, a_pk) = bebop2_core::sign::keygen(&[0x11u8; 32]);
        let _ = a_seed;
        let mut roster = AnchorRoster::new();
        roster.enroll(&a_pk);
        assert!(roster.contains(&a_pk), "anchor enrolled");
        RevocationSet::drop_anchor(&mut roster, &a_pk);
        assert!(
            !roster.contains(&a_pk),
            "dropped anchor can no longer vouch"
        );
        let (_, other) = bebop2_core::sign::keygen(&[0x22u8; 32]);
        RevocationSet::drop_anchor(&mut roster, &other);
        assert!(!roster.contains(&other));
    }

    // RED — MESH-11 gossip anti-entropy: two nodes' revocation sets converge
    // after a gossip_payload -> merge round (idempotent, no duplication).
    #[test]
    fn gossip_payload_merge_converges_idempotent() {
        let mut a = RevocationSet::new();
        a.revoke_key([1u8; 32]);
        a.revoke_capability([9u8; 32]);
        let mut b = RevocationSet::new();
        b.revoke_key([2u8; 32]);

        let (ak, ac) = a.gossip_payload();
        let mut wire = RevocationSet::new();
        for k in &ak {
            wire.revoke_key(*k);
        }
        for c in &ac {
            wire.revoke_capability(*c);
        }
        b.merge(&wire);

        assert!(b.is_revoked_key(&[1u8; 32]));
        assert!(b.is_revoked_key(&[2u8; 32]));
        assert!(b.is_revoked_capability(&[9u8; 32]));
        // Second gossip round is a no-op (idempotent union).
        let (ak2, ac2) = a.gossip_payload();
        let mut wire2 = RevocationSet::new();
        for k in &ak2 {
            wire2.revoke_key(*k);
        }
        for c in &ac2 {
            wire2.revoke_capability(*c);
        }
        b.merge(&wire2);
        assert_eq!(
            b.gossip_payload().0.len(),
            2,
            "no duplicate keys after re-merge"
        );
    }
}
