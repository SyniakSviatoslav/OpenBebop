//! Capability — the signed per-frame authorization statement.
//!
//! Replaces JWT bearer tokens. A capability is a single-use, signed statement
//! `{subject_key, scope, nonce, expiry}` — verifiable by any peer without a
//! central issuer. It authorises exactly one ACTION on one RESOURCE for one KEY,
//! bounded by a nonce/expiry. NOT a bearer token, NOT a score.
//!
//! CI GUARD: NO-COURIER-SCORING — capability never references a score/trust.

use serde::{Deserialize, Serialize};

use crate::scope::{Action, Resource, Scope};

/// A single-use, signed authorization statement.
///
/// The signing domain is the *canonical* serialization of the public fields only
/// (not the signatures). Tampering with any field invalidates the signature, so a
/// capability cannot be replayed on a different scope/nonce/expiry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Capability {
    /// Ed25519 public key (32 bytes) of the subject the capability is issued to.
    /// Verbatim bytes, never interpreted as a reputation/score.
    pub subject_key: [u8; 32],
    /// What the capability authorizes (resource + action). No rating fields.
    pub scope: Scope,
    /// Single-use nonce (8 bytes). Replay-protected by the verifier's nonce set.
    pub nonce: [u8; 8],
    /// Expiry as a unix-ish monotonically-increasing counter (no clock dependency
    /// required by this struct; the caller supplies a comparable tick).
    pub expiry: u64,
}

impl Capability {
    /// Build a capability. `subject_key` is the Ed25519 public key of the mover;
    /// it is an identity, not a trust rating.
    pub fn new(
        subject_key: [u8; 32],
        resource: Resource,
        action: Action,
        nonce: [u8; 8],
        expiry: u64,
    ) -> Self {
        Capability {
            subject_key,
            scope: Scope::new(resource, action),
            nonce,
            expiry,
        }
    }

    /// Canonical bytes that get signed. `serde_json` (deterministic field order)
    /// gives a stable encoding across peers.
    pub fn canonical_bytes(&self) -> CapResult<Vec<u8>> {
        serde_json::to_vec(self).map_err(|_| CapError::Encode)
    }

    /// Whether `expiry` is still acceptable against `now`. Pure comparison — no
    /// clock, no drift score.
    pub fn is_fresh(&self, now: u64) -> bool {
        self.expiry > now
    }
}

use crate::error::{CapError, CapResult};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_canonical_is_stable() {
        let cap = Capability::new(
            [7u8; 32],
            Resource::Route,
            Action::Send,
            [1, 2, 3, 4, 5, 6, 7, 8],
            9999,
        );
        let a = cap.canonical_bytes().unwrap();
        let b = cap.canonical_bytes().unwrap();
        assert_eq!(a, b, "canonical encoding must be deterministic");
        assert!(cap.is_fresh(9998));
        assert!(!cap.is_fresh(9999));
    }
}
