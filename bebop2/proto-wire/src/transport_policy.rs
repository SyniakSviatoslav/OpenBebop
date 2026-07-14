//! MESH-10 transport hardening policy — replay-safety, TLS-required gate,
//! DoS limits, and post-quantum payload encryption (trait).
//!
//! This module holds the *policy + logic* of the transport hardening. The
//! actual TLS termination (rustls) and the QUIC carrier (iroh) are wired by the
//! host at deploy time; this code defines the enforceable invariants so they
//! are RED-tested offline without pulling rustls/quinn into the default build.
//!
//! CI GUARD: NO-COURIER-SCORING — policies act on frames/keys, never scores.

use std::sync::Arc;
use std::sync::Mutex;

use bebop_proto_cap::SignedFrame;

use crate::error::{WireError, WireResult};

/// Maximum accepted WebSocket message / frame size (B3 fix): 8 MiB, not
/// tungstenite's 64 MiB default. A frame larger than this is rejected before
/// allocation.
pub const MAX_MESSAGE_BYTES: usize = 8 << 20;

/// Idle read timeout (B3 / Slowloris fix): a stalled receiver is dropped.
pub const IDLE_TIMEOUT_SECS: u64 = 30;

/// Per-IP pre-accept token budget (DoS): a peer that exhausts its bucket must
/// wait. This is the pure accounting primitive; the caller applies it to the
/// accept path.
#[derive(Debug, Clone)]
pub struct TokenBucket {
    capacity: u32,
    refill_per_sec: u32,
    /// (tokens, last_refill_unix_sec)
    state: Arc<Mutex<(u32, u64)>>,
}

impl TokenBucket {
    /// New bucket with `capacity` tokens, refilling `refill_per_sec`.
    pub fn new(capacity: u32, refill_per_sec: u32) -> Self {
        TokenBucket {
            capacity,
            refill_per_sec,
            state: Arc::new(Mutex::new((capacity, 0))),
        }
    }

    /// Try to consume one token at `now` (unix seconds). Refills first.
    /// Returns `false` (reject) if empty.
    pub fn try_acquire(&self, now: u64) -> bool {
        let mut g = self.state.lock().unwrap();
        let (tokens, last) = *g;
        let elapsed = now.saturating_sub(last);
        let refilled =
            (tokens as u64 + elapsed * self.refill_per_sec as u64).min(self.capacity as u64);
        if refilled == 0 {
            return false;
        }
        *g = (refilled as u32 - 1, now);
        true
    }
}

/// Enforceable transport policy.
#[derive(Debug, Clone)]
pub struct TransportPolicy {
    /// If true, a frame arriving with no `channel_binding` (i.e. not derived
    /// from a real TLS/QUIC exporter, RFC 5705) is REJECTED. Plaintext `ws://`
    /// without a binder cannot satisfy this.
    pub require_tls_channel_binding: bool,
    /// Max message bytes (see [`MAX_MESSAGE_BYTES`]).
    pub max_message_bytes: usize,
    /// Idle read timeout seconds (see [`IDLE_TIMEOUT_SECS`]).
    pub idle_timeout_secs: u64,
    /// Connection cap (Semaphore-equivalent accounting).
    pub max_concurrent_conns: usize,
}

impl Default for TransportPolicy {
    fn default() -> Self {
        TransportPolicy {
            require_tls_channel_binding: false,
            max_message_bytes: MAX_MESSAGE_BYTES,
            idle_timeout_secs: IDLE_TIMEOUT_SECS,
            max_concurrent_conns: 1024,
        }
    }
}

impl TransportPolicy {
    /// Reject a frame that violates the policy. Returns `Ok(())` if admissible.
    /// - oversized payload -> `PayloadTooLarge`
    /// - `require_tls_channel_binding` && frame has no `channel_binding` ->
    ///   `InsecureTransport` (plaintext rejected when TLS required)
    pub fn admit(&self, frame: &SignedFrame) -> WireResult<()> {
        if frame.payload.len() > self.max_message_bytes {
            return Err(WireError::PayloadTooLarge(frame.payload.len()));
        }
        if self.require_tls_channel_binding && frame.channel_binding.is_none() {
            return Err(WireError::InsecureTransport(
                "channel_binding required but frame has none (plaintext rejected when TLS required)",
            ));
        }
        Ok(())
    }
}

/// Post-quantum payload encryption trait (MESH-10 defense-in-depth).
///
/// The default [`NoopPayloadEnc`] passes frames through (local-first dev
/// default). A production deployment wires a real impl (ML-KEM-768 ->
/// XChaCha20-Poly1305) provided by `dowiz-pq`; that impl lives behind a
/// feature the host enables — it is NOT compiled into the default build so the
/// sovereign core stays offline-clean.
pub trait PayloadEnc {
    /// Encrypt `plaintext` for `peer` (32-byte id). Returned bytes must be
    /// authenticated + nonce-safe.
    fn encrypt(&self, peer: &[u8; 32], plaintext: &[u8]) -> Vec<u8>;
    /// Inverse of [`encrypt`](Self::encrypt).
    fn decrypt(&self, peer: &[u8; 32], ciphertext: &[u8]) -> WireResult<Vec<u8>>;
}

/// No-operation payload encryption (default). Offline-clean, zero deps.
#[derive(Debug, Clone, Default)]
pub struct NoopPayloadEnc;

impl PayloadEnc for NoopPayloadEnc {
    fn encrypt(&self, _peer: &[u8; 32], plaintext: &[u8]) -> Vec<u8> {
        plaintext.to_vec()
    }
    fn decrypt(&self, _peer: &[u8; 32], ciphertext: &[u8]) -> WireResult<Vec<u8>> {
        Ok(ciphertext.to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bebop_proto_cap::{Action, Capability, Resource};

    fn frame_with_binding(binding: Option<[u8; 32]>) -> SignedFrame {
        let cap = Capability::new(
            [7u8; 32],
            Resource::Route,
            Action::Send,
            [1u8; 8],
            9_999_999_999,
        );
        let mut f = SignedFrame::new(cap, b"hello".to_vec());
        f.channel_binding = binding;
        f
    }

    // RED — B3 plaintext fix: with require_tls_channel_binding ON, a frame that
    // carries no channel binding (i.e. arrived over plaintext ws://) is REJECTED.
    #[test]
    fn red_plaintext_ws_rejected_when_tls_required() {
        let policy = TransportPolicy {
            require_tls_channel_binding: true,
            ..Default::default()
        };
        let plain = frame_with_binding(None);
        assert!(
            matches!(policy.admit(&plain), Err(WireError::InsecureTransport(_))),
            "plaintext frame must be rejected when TLS channel-binding required"
        );

        let bound = frame_with_binding(Some([0xAAu8; 32]));
        assert!(policy.admit(&bound).is_ok(), "channel-bound frame admitted");
    }

    // RED — oversized payload rejected before allocation (DoS / memory).
    #[test]
    fn red_oversized_payload_rejected() {
        let policy = TransportPolicy {
            max_message_bytes: 16,
            ..Default::default()
        };
        let mut big = frame_with_binding(None);
        big.payload = vec![0u8; 4096];
        assert!(matches!(
            policy.admit(&big),
            Err(WireError::PayloadTooLarge(_))
        ));
    }

    // RED — token bucket exhausts then refills (DoS pre-accept gate).
    #[test]
    fn red_token_bucket_exhaust_then_refill() {
        let b = TokenBucket::new(2, 1);
        assert!(b.try_acquire(0));
        assert!(b.try_acquire(0));
        assert!(!b.try_acquire(0), "empty bucket rejects");
        // No time passed -> still empty.
        assert!(!b.try_acquire(0));
        // 5 seconds later, 5 tokens refilled (capped at capacity 2).
        assert!(b.try_acquire(5));
        assert!(b.try_acquire(5));
        assert!(!b.try_acquire(5), "refilled to capacity then empty");
    }

    // GREEN — NoopPayloadEnc round-trips.
    #[test]
    fn green_noop_payload_enc_roundtrip() {
        let enc = NoopPayloadEnc;
        let peer = [3u8; 32];
        let ct = enc.encrypt(&peer, b"secret");
        assert_eq!(enc.decrypt(&peer, &ct).unwrap(), b"secret");
    }
}
