//! P10 (M9/F28) — the operator **kill-switch**: a unilateral, anchor-signed
//! `KillOrder` that HALTS this hub. This is the DELIBERATE OPPOSITE of the
//! legacy `crates/bebop/src/guard.rs::KillSwitch`, which is a ≥2/3 *consensus*
//! vote registry. M9 authority is UNILATERAL: a single genesis kill-anchor
//! signature is sufficient and NO quorum is consulted on this path.
//!
//! # Trust model (reuses existing crypto — no new primitive)
//! A `KillOrder` is signed with the SAME `bebop2-core::sign` Ed25519 primitive
//! the roster/hybrid-gate already use. Verification requires the signing key to
//! be an enrolled **kill anchor** (loaded from genesis, §4). The order carries a
//! monotonic `nonce`; a replay ledger rejects any nonce already seen (F28
//! replay defense). No quorum, no vote count — one valid kill-anchor signature
//! HALTS the hub.
//!
//! # COLD-backup-THEN-halt sequencing (§3.3, HARD invariant)
//! The handler MUST NOT halt before a COLD snapshot is confirmed durable. It
//! blocks on a [`SnapshotConfirmer`] (a trait, because the Phase-12 archiver may
//! not exist yet — we block on a real receipt, we never fake one). Order of
//! operations, enforced by [`KillSequence`]:
//!   1. verify signature + kill-anchor membership + nonce (fail-closed),
//!   2. request a COLD snapshot and BLOCK for the confirmed receipt,
//!   3. only then transition to `Halted`.
//! If the snapshot is not confirmed, the hub STAYS RUNNING (never a silent halt
//! with lost state).
//!
//! CI GUARD: NO-COURIER-SCORING — a kill order names an authority, not a score.

use std::collections::HashSet;

use bebop2_core::hash::sha3_256;
use bebop2_core::sign;

/// The set of genesis-enrolled **kill anchors** (Ed25519 public keys). A
/// `KillOrder` is only honored if signed by a key in this set. Loaded from
/// genesis at boot (§4); deny-by-default when empty (no anchor => no kill).
#[derive(Debug, Clone, Default)]
pub struct KillAnchors {
    keys: HashSet<[u8; 32]>,
}

impl KillAnchors {
    /// Empty set — no kill authority (deny-by-default).
    pub fn new() -> Self {
        KillAnchors {
            keys: HashSet::new(),
        }
    }

    /// Enroll a kill-anchor public key (from genesis).
    pub fn enroll(&mut self, pubkey: [u8; 32]) {
        self.keys.insert(pubkey);
    }

    /// Whether `key` is an enrolled kill anchor.
    pub fn contains(&self, key: &[u8; 32]) -> bool {
        self.keys.contains(key)
    }

    /// True when no anchor is enrolled (kill path is deny-by-default).
    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }
}

/// A unilateral operator kill order. Rides inside a `proto-wire`
/// `FrameKind::OperatorKill` frame. Signed by a genesis kill anchor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KillOrder {
    /// The signing kill-anchor public key (must be enrolled).
    pub anchor: [u8; 32],
    /// Monotonic replay nonce. Any nonce already in the replay ledger is
    /// rejected (F28). 16 bytes to match the capability nonce convention.
    pub nonce: [u8; 16],
    /// Reason string (audit; part of the signed domain so it cannot be swapped).
    pub reason: String,
    /// Ed25519 signature over [`KillOrder::signing_domain`].
    pub sig: [u8; 64],
}

impl KillOrder {
    /// Canonical, domain-separated signing input. Fixed-layout (NOT serde): a
    /// tag, the anchor key, the nonce, then a length-prefixed reason. Matches
    /// the "no serde on the signed path" rule (ARCHITECTURE.md:75).
    pub fn signing_domain(anchor: &[u8; 32], nonce: &[u8; 16], reason: &str) -> Vec<u8> {
        let mut d = Vec::new();
        d.extend_from_slice(b"BEBOP2/OPERATOR-KILL/v1");
        d.extend_from_slice(anchor);
        d.extend_from_slice(nonce);
        let rb = reason.as_bytes();
        d.extend_from_slice(&(rb.len() as u32).to_le_bytes());
        d.extend_from_slice(rb);
        d
    }

    /// Sign a kill order with the anchor's Ed25519 seed. `anchor` MUST be the
    /// public key derived from `seed` (the verifier re-checks membership only —
    /// it trusts the signature to bind the key).
    pub fn sign(anchor: [u8; 32], nonce: [u8; 16], reason: &str, seed: &[u8; 32]) -> KillOrder {
        let domain = Self::signing_domain(&anchor, &nonce, reason);
        let sig = sign::sign(seed, &domain);
        KillOrder {
            anchor,
            nonce,
            reason: reason.to_string(),
            sig,
        }
    }

    /// A stable id for the order (for the replay ledger): SHA3-256 of the
    /// signing domain (binds anchor+nonce+reason). Two distinct orders never
    /// collide; a replay of the exact same order collides (and is rejected).
    pub fn replay_id(&self) -> [u8; 32] {
        sha3_256(&Self::signing_domain(&self.anchor, &self.nonce, &self.reason))
    }
}

/// A monotonic replay ledger of kill-order ids already accepted. Fail-closed: a
/// repeated id is rejected. In-memory here; a durable backing is a later phase.
#[derive(Debug, Clone, Default)]
pub struct ReplayLedger {
    seen: HashSet<[u8; 32]>,
}

impl ReplayLedger {
    /// New empty ledger.
    pub fn new() -> Self {
        ReplayLedger {
            seen: HashSet::new(),
        }
    }

    /// Record an id. Returns `true` if newly recorded, `false` if it was a
    /// replay (already present).
    pub fn record(&mut self, id: [u8; 32]) -> bool {
        self.seen.insert(id)
    }

    /// Whether `id` was already seen.
    pub fn contains(&self, id: &[u8; 32]) -> bool {
        self.seen.contains(id)
    }
}

/// Why a kill order was refused (fail-closed reasons).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KillReject {
    /// Signing key is not an enrolled genesis kill anchor.
    NotAnchor,
    /// Ed25519 signature does not verify over the signing domain.
    BadSignature,
    /// Nonce/id already seen (replay).
    Replay,
    /// No kill anchor enrolled at all (deny-by-default).
    NoAnchors,
    /// The COLD snapshot was NOT confirmed durable — hub stays running (§3.3).
    SnapshotNotConfirmed,
}

/// A confirmed COLD snapshot receipt (the archiver's proof the state is
/// durable). We block on a REAL receipt; we never synthesize one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotReceipt {
    /// Content hash of the confirmed snapshot (audit).
    pub snapshot_sha3: [u8; 32],
}

/// The COLD-snapshot boundary the kill handler blocks on before halting.
///
/// The Phase-12 archiver may not exist yet; this trait is the seam it implements
/// later. Until then the boot wiring supplies an implementor (a real local file
/// snapshot, or an explicit `RefuseSnapshot` that keeps the hub RUNNING — never
/// a fake "confirmed" receipt).
pub trait SnapshotConfirmer {
    /// Take a COLD snapshot and BLOCK until it is confirmed durable. `Ok`
    /// carries the receipt; `Err` means NOT durable => the caller MUST NOT halt.
    fn snapshot_and_confirm(&self) -> Result<SnapshotReceipt, String>;
}

/// The kill-handler state machine (§3.3). Enforces order: verify → snapshot →
/// halt. The `Halted` state is only reachable AFTER a confirmed receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KillState {
    /// Normal operation.
    Running,
    /// Verified + snapshot confirmed; hub is halted with the receipt recorded.
    Halted { receipt: SnapshotReceipt, reason: String },
}

/// The kill sequencer: owns the anchors, the replay ledger, and the current
/// state. `handle` runs the full fail-closed sequence.
#[derive(Debug)]
pub struct KillSequence {
    anchors: KillAnchors,
    ledger: ReplayLedger,
    state: KillState,
}

impl KillSequence {
    /// New sequencer over a genesis anchor set (Running state).
    pub fn new(anchors: KillAnchors) -> Self {
        KillSequence {
            anchors,
            ledger: ReplayLedger::new(),
            state: KillState::Running,
        }
    }

    /// Current state.
    pub fn state(&self) -> &KillState {
        &self.state
    }

    /// Whether the hub is halted.
    pub fn is_halted(&self) -> bool {
        matches!(self.state, KillState::Halted { .. })
    }

    /// Verify a kill order WITHOUT running the halt sequence. Fail-closed:
    /// checks anchor enrollment, signature, and replay. Does NOT mutate the
    /// ledger (call [`KillSequence::handle`] to actually accept + halt).
    pub fn verify(&self, order: &KillOrder) -> Result<(), KillReject> {
        if self.anchors.is_empty() {
            return Err(KillReject::NoAnchors);
        }
        if !self.anchors.contains(&order.anchor) {
            return Err(KillReject::NotAnchor);
        }
        let domain = KillOrder::signing_domain(&order.anchor, &order.nonce, &order.reason);
        if !sign::verify(&order.anchor, &domain, &order.sig) {
            return Err(KillReject::BadSignature);
        }
        if self.ledger.contains(&order.replay_id()) {
            return Err(KillReject::Replay);
        }
        Ok(())
    }

    /// Handle a kill order end-to-end (§3.3): verify → record nonce → COLD
    /// snapshot (BLOCK for confirmation) → halt. The order of operations is the
    /// HARD invariant: the state only becomes `Halted` AFTER a confirmed
    /// receipt. If the snapshot is not confirmed, the ledger still records the
    /// nonce (the order was authentic) but the state STAYS `Running`.
    pub fn handle<C: SnapshotConfirmer>(
        &mut self,
        order: &KillOrder,
        confirmer: &C,
    ) -> Result<SnapshotReceipt, KillReject> {
        // 1. Verify (fail-closed) BEFORE any side effect.
        self.verify(order)?;
        // Record the nonce so an authentic order cannot be replayed even if the
        // snapshot step fails and we retry with a fresh order.
        self.ledger.record(order.replay_id());
        // 2. COLD snapshot — BLOCK for a confirmed receipt. NEVER halt first.
        let receipt = confirmer
            .snapshot_and_confirm()
            .map_err(|_| KillReject::SnapshotNotConfirmed)?;
        // 3. Only now: halt.
        self.state = KillState::Halted {
            receipt: receipt.clone(),
            reason: order.reason.clone(),
        };
        Ok(receipt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn anchor_key(b: u8) -> ([u8; 32], [u8; 32]) {
        let seed = [b; 32];
        let (pk, _) = sign::keygen(&seed);
        (seed, pk)
    }

    /// A confirmer that always confirms (a real in-memory durable stand-in for
    /// tests — it returns a REAL content hash, not a faked receipt).
    struct OkConfirmer;
    impl SnapshotConfirmer for OkConfirmer {
        fn snapshot_and_confirm(&self) -> Result<SnapshotReceipt, String> {
            Ok(SnapshotReceipt {
                snapshot_sha3: sha3_256(b"cold-snapshot-state"),
            })
        }
    }

    /// A confirmer that refuses (snapshot not durable) — proves we never halt
    /// without a confirmed COLD backup.
    struct RefuseConfirmer;
    impl SnapshotConfirmer for RefuseConfirmer {
        fn snapshot_and_confirm(&self) -> Result<SnapshotReceipt, String> {
            Err("archiver unavailable".into())
        }
    }

    // ── §6.4 GREEN: a valid anchor-signed kill order halts the hub ──
    #[test]
    fn valid_kill_order_halts_after_snapshot() {
        let (seed, pk) = anchor_key(0x40);
        let mut anchors = KillAnchors::new();
        anchors.enroll(pk);
        let mut seq = KillSequence::new(anchors);
        assert_eq!(seq.state(), &KillState::Running);

        let order = KillOrder::sign(pk, [1u8; 16], "operator halt", &seed);
        let receipt = seq.handle(&order, &OkConfirmer).unwrap();
        assert!(seq.is_halted());
        match seq.state() {
            KillState::Halted { receipt: r, reason } => {
                assert_eq!(r, &receipt);
                assert_eq!(reason, "operator halt");
            }
            _ => panic!("expected Halted"),
        }
    }

    // ── §6.5 RED: a NON-anchor signature is refused (no quorum, no fallback) ──
    #[test]
    fn non_anchor_kill_order_is_refused() {
        let (_seed, pk) = anchor_key(0x40);
        let (evil_seed, evil_pk) = anchor_key(0x99);
        let mut anchors = KillAnchors::new();
        anchors.enroll(pk); // only pk is a kill anchor
        let mut seq = KillSequence::new(anchors);

        // Evil key signs its own order — not enrolled => NotAnchor.
        let order = KillOrder::sign(evil_pk, [2u8; 16], "rogue halt", &evil_seed);
        assert_eq!(seq.verify(&order), Err(KillReject::NotAnchor));
        assert_eq!(seq.handle(&order, &OkConfirmer), Err(KillReject::NotAnchor));
        assert!(!seq.is_halted());
    }

    // ── §6.5 RED: a tampered signature is refused ──
    #[test]
    fn tampered_kill_order_is_refused() {
        let (seed, pk) = anchor_key(0x41);
        let mut anchors = KillAnchors::new();
        anchors.enroll(pk);
        let mut seq = KillSequence::new(anchors);

        let mut order = KillOrder::sign(pk, [3u8; 16], "halt", &seed);
        order.reason = "different reason".to_string(); // breaks the signed domain
        assert_eq!(seq.verify(&order), Err(KillReject::BadSignature));
        assert!(!seq.is_halted());
    }

    // ── §6.6 RED: a replayed kill order is refused (nonce ledger) ──
    #[test]
    fn replayed_kill_order_is_refused() {
        let (seed, pk) = anchor_key(0x42);
        let mut anchors = KillAnchors::new();
        anchors.enroll(pk);
        let mut seq = KillSequence::new(anchors);

        let order = KillOrder::sign(pk, [4u8; 16], "halt", &seed);
        seq.handle(&order, &OkConfirmer).unwrap();
        // Same order again => replay.
        assert_eq!(seq.verify(&order), Err(KillReject::Replay));
    }

    // ── §6.7 HARD invariant: never halt before COLD snapshot confirmed ──
    #[test]
    fn does_not_halt_when_snapshot_not_confirmed() {
        let (seed, pk) = anchor_key(0x43);
        let mut anchors = KillAnchors::new();
        anchors.enroll(pk);
        let mut seq = KillSequence::new(anchors);

        let order = KillOrder::sign(pk, [5u8; 16], "halt", &seed);
        let res = seq.handle(&order, &RefuseConfirmer);
        assert_eq!(res, Err(KillReject::SnapshotNotConfirmed));
        // HARD: the hub STAYS RUNNING — no silent halt with lost state.
        assert!(!seq.is_halted());
        assert_eq!(seq.state(), &KillState::Running);
    }

    // ── deny-by-default: no anchors enrolled => no kill possible ──
    #[test]
    fn no_anchors_denies_all_kills() {
        let (seed, pk) = anchor_key(0x44);
        let seq = KillSequence::new(KillAnchors::new()); // empty
        let order = KillOrder::sign(pk, [6u8; 16], "halt", &seed);
        assert_eq!(seq.verify(&order), Err(KillReject::NoAnchors));
    }
}
