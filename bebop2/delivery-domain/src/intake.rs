//! P13 · §3 Order intake → signed envelope → DOD gate → state fold (AC-1, AC-2, AC-6).
//!
//! The delivery spine. An **owner hub** (chosen by `hub_ring`, §2) runs the
//! [`IntakeEdge`], which turns a local `OrderTransition` intent into a
//! **canonical, signed `SignedFrame`** carrying the transition as its payload.
//! Every receiving hub then runs the exact gate-chain the blueprint mandates:
//!
//! ```text
//! wire (SignedFrame, HRW-rooted delegation chain)
//!   └─ DodGate::admit   (mesh-node DOD: authored / scoped / fresh / non-replay)
//!   └─ KernelFacade::submit_intent
//!         ├─ WIRE  : HybridGate::check (replay/expiry/chain/Ed25519/ML-DSA-65)
//!         ├─ LAW   : dowiz_kernel assert_transition (forged skip rejected HERE)
//!         └─ MONEY : kernel apply_event i64 fold
//! ```
//!
//! # Why every receiver is authoritative (AC-2)
//! The receiver NEVER trusts the sender's claimed status. It validates the
//! transition against its OWN local order state through the kernel Law
//! (`assert_transition(order.status, next)`). A forged `Pending → Delivered`
//! frame carries a *valid* signature (the forger is a legit signer) but an
//! *illegal* state jump; the kernel Law returns `Err` and the fold is rejected
//! on **every** receiver — not just the honest owner. This is the extension of
//! `delivery-domain/src/lib.rs:179` (`mesh01_forge_pending_to_delivered`) into
//! the full wire→DOD→Law→money spine.
//!
//! # Zero new dependencies
//! Reuses `bebop_proto_cap::SignedFrame` + `HybridGate`, `bebop_mesh_node::DodGate`,
//! and the `dowiz_kernel` Law (re-exported under `kernel-rlib`). No tonic/prost,
//! no transport crate is *used* — only the gate types.

use bebop2_core::pq_dsa::{keygen, keygen_derivable};
use bebop2_core::sign::keygen as ed_keygen;
use bebop_proto_cap::capability::Capability;
use bebop_proto_cap::revocation::RevocationSet;
use bebop_proto_cap::roster::{AnchorRoster, Delegation, Effect};
use bebop_proto_cap::scope::{Action, Resource, Scope};
use bebop_proto_cap::signed_frame::SignedFrame;

use bebop_mesh_node::dod::{DodFault, DodGate};
use bebop_mesh_node::DodGate as _; // bring `admit` into scope

use dowiz_kernel::domain::{apply_event, place_order, Order, OrderItem};
use dowiz_kernel::money::Currency;
use dowiz_kernel::order_machine::{assert_transition, OrderStatus};
use dowiz_kernel::vendor::VendorId;

use crate::facade::{KernelFacade, Reject};
use crate::DeliveryStatus;
use crate::OrderTransition;

/// Map a wire `DeliveryStatus` to the kernel's canonical `OrderStatus`.
/// The two enums share the same lifecycle vocabulary (Pending → Delivered,
/// Rejected, Cancelled, PickedUp); `OrderStatus` additionally has the scaffold
/// `Scheduled` terminal, which delivery has no mapping for.
pub fn to_order_status(d: DeliveryStatus) -> OrderStatus {
    match d {
        DeliveryStatus::Pending => OrderStatus::Pending,
        DeliveryStatus::Confirmed => OrderStatus::Confirmed,
        DeliveryStatus::Preparing => OrderStatus::Preparing,
        DeliveryStatus::Ready => OrderStatus::Ready,
        DeliveryStatus::InDelivery => OrderStatus::InDelivery,
        DeliveryStatus::Delivered => OrderStatus::Delivered,
        DeliveryStatus::Rejected => OrderStatus::Rejected,
        DeliveryStatus::Cancelled => OrderStatus::Cancelled,
        DeliveryStatus::PickedUp => OrderStatus::PickedUp,
    }
}

/// Map the kernel `OrderStatus` back to a wire `DeliveryStatus`.
pub fn from_order_status(s: OrderStatus) -> Option<DeliveryStatus> {
    match s {
        OrderStatus::Pending => Some(DeliveryStatus::Pending),
        OrderStatus::Confirmed => Some(DeliveryStatus::Confirmed),
        OrderStatus::Preparing => Some(DeliveryStatus::Preparing),
        OrderStatus::Ready => Some(DeliveryStatus::Ready),
        OrderStatus::InDelivery => Some(DeliveryStatus::InDelivery),
        OrderStatus::Delivered => Some(DeliveryStatus::Delivered),
        OrderStatus::Rejected => Some(DeliveryStatus::Rejected),
        OrderStatus::Cancelled => Some(DeliveryStatus::Cancelled),
        OrderStatus::PickedUp => Some(DeliveryStatus::PickedUp),
        OrderStatus::Scheduled => None,
        // Kernel `OrderStatus` carries refund states the wire `DeliveryStatus` has
        // no counterpart for yet — leave unmapped (None). Required for match
        // exhaustiveness against the linked dowiz-kernel.
        OrderStatus::Refunding => None,
        OrderStatus::CompensatedRefund => None,
    }
}

/// Deterministic per-frame event id for the DOD replay set: the low 64 bits of
/// the SHA3-256 of the frame's canonical payload. Stable across nodes, so two
/// receivers agree on "same frame" for replay dedup.
fn event_id_of(frame: &SignedFrame) -> u64 {
    let h = bebop2_core::hash::sha3_256(&frame.payload);
    let mut a = [0u8; 8];
    a.copy_from_slice(&h[0..8]);
    u64::from_le_bytes(a)
}

/// Why a delivery fold was refused. Tagged by which gate stopped it so a caller
/// can distinguish a DOD fault (transport hygiene) from a Law fault (illegal
/// intent) from a wire fault (bad signature).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FoldError {
    /// mesh-node Definition-of-Done gate refused the carried event.
    Dod(DodFault),
    /// WIRE/LAW/MONEY gate-chain (KernelFacade) refused the intent.
    Gate(Reject),
    /// The frame's payload did not decode as a canonical `OrderTransition`.
    BadPayload,
}

/// The owner-hub intake edge. Turns a local `(from → to)` intent into a signed
/// `SignedFrame` carrying the canonical `OrderTransition` bytes as payload,
/// authorized by a real anchor-rooted hybrid delegation chain.
pub struct IntakeEdge {
    anchor_seed: [u8; 32],
    leaf_seed: [u8; 32],
    leaf_pk: [u8; 32],
    pq_pk: Vec<u8>,
    pq_sk: [u8; 4032],
    roster: AnchorRoster,
    chain: Vec<Delegation>,
    nonce: [u8; 8],
    expiry: u64,
}

impl IntakeEdge {
    /// Build a live intake edge. `seed_byte` derives the anchor + leaf +
    /// post-quantum keypairs and the HRW-rooted delegation chain that grants the
    /// leaf `(Order, OrderStatusChanged)`. All signatures are real Ed25519 +
    /// ML-DSA-65.
    pub fn new(seed_byte: u8) -> Self {
        let anchor_seed = [seed_byte; 32];
        let leaf_seed = [seed_byte ^ 0xFF; 32];
        let (anchor_pk, _) = ed_keygen(&anchor_seed);
        let (leaf_pk, _) = ed_keygen(&leaf_seed);

        // Real ML-DSA-65 PQ keypair (the PQ leg of the hybrid identity).
        let pq_seed = [seed_byte.wrapping_add(7); 32];
        let (pq_pk, pq_sk) = keygen_derivable(&pq_seed);

        let mut roster = AnchorRoster::new();
        roster.enroll(&anchor_pk);

        let scope = Scope::single(Resource::Order, Action::OrderStatusChanged);
        let effect = Effect::single(Resource::Order, Action::OrderStatusChanged);
        let nonce = [seed_byte as u8, 0, 0, 0, 0, 0, 0, 0];
        let expiry = 9_999_999_999u64;
        let delegation = Delegation::sign(
            anchor_pk, leaf_pk, scope, effect, expiry, nonce, &anchor_seed,
        )
        .expect("real anchor delegation signature");

        IntakeEdge {
            anchor_seed,
            leaf_seed,
            leaf_pk,
            pq_pk: pq_pk.bytes.clone(),
            pq_sk: pq_sk.bytes.clone().try_into().expect("ml-dsa sk is 4896 bytes"),
            roster,
            chain: vec![delegation],
            nonce,
            expiry,
        }
    }

    /// The enrolled anchor roster the gate verifies against.
    pub fn roster(&self) -> &AnchorRoster {
        &self.roster
    }

    /// The delegation chain authorizing this edge's frames.
    pub fn chain(&self) -> &[Delegation] {
        &self.chain
    }

    /// Emit a signed `SignedFrame` for the transition `from → to` on `order_id`.
    ///
    /// The payload is the canonical `OrderTransition::to_bytes()` (10 bytes),
    /// so any receiver can decode it and validate the jump against its OWN local
    /// order state (AC-2). The capability authorizes exactly
    /// `(Order, OrderStatusChanged)`; the frame is signed with BOTH the
    /// classical (Ed25519) and PQ (ML-DSA-65) legs.
    pub fn emit(&self, order_id: u64, from: DeliveryStatus, to: DeliveryStatus) -> SignedFrame {
        let t = OrderTransition { order_id, from, to };
        let payload = t.to_bytes();
        // Deterministic, per-transition nonce: distinct for every (order, from, to)
        // so the WIRE gate's replay ledger never rejects a legitimately-new frame.
        let mut nonce = [0u8; 8];
        nonce[0] = (order_id ^ ((from as u64) << 4) ^ ((to as u64) << 1)) as u8;
        nonce[1] = self.nonce[0];
        let cap = Capability::new_hybrid(
            self.leaf_pk,
            self.pq_pk.clone(),
            Resource::Order,
            Action::OrderStatusChanged,
            nonce,
            self.expiry,
        );
        let mut frame = SignedFrame::new(cap, payload.to_vec());
        frame.delegation_chain = self.chain.clone();
        frame
            .sign_classical(&self.leaf_seed)
            .expect("real classical signature");
        frame
            .sign_pq(&self.pq_sk, &[0u8; 32])
            .expect("real PQ signature");
        frame
    }

    /// Build the receiver-side fold pipeline (DOD gate + WIRE→LAW→MONEY facade)
    /// armed with this edge's roster + policy. `RequireBoth` is the production
    /// red-line policy (both classical + PQ legs must verify).
    pub fn receiver(&self) -> DeliveryReceiver {
        DeliveryReceiver {
            dod: DodGate::new(),
            facade: KernelFacade::new(
                bebop_proto_cap::hybrid_gate::HybridPolicy::RequireBoth,
                self.roster.clone(),
                RevocationSet::new(),
            ),
        }
    }
}

/// A receiving hub's fold pipeline: the mesh-node DOD gate feeding the
/// WIRE→LAW→MONEY `KernelFacade`. One instance per (hub, order).
pub struct DeliveryReceiver {
    pub dod: DodGate,
    pub facade: KernelFacade,
}

impl DeliveryReceiver {
    /// Admit a carried frame through the DOD gate, decode its transition, and
    /// fold it into `order` via the kernel Law.
    ///
    /// `now` is the monotonic tick; the DOD lifetime check uses the frame's
    /// capability `expiry` as `expires_at`.
    pub fn admit_and_fold(
        &mut self,
        frame: &SignedFrame,
        order: &Order,
        now: u64,
    ) -> Result<DeliveryStatus, FoldError> {
        // ── Gate 0 · DOD (mesh-node) ── fail-closed event hygiene.
        let event = bebop_proto_cap::Event {
            id: event_id_of(frame),
            payload: frame.payload.clone(),
        };
        self.dod
            .admit(&event, now, frame.capability.expiry)
            .map_err(FoldError::Dod)?;

        // ── Decode the transition the sender is asserting.
        let t = OrderTransition::from_bytes(frame.payload.as_slice())
            .ok_or(FoldError::BadPayload)?;
        let next = to_order_status(t.to);

        // ── Gate 1-3 · WIRE → LAW → MONEY (kernel facade).
        let applied = self
            .facade
            .submit_intent(frame, frame.delegation_chain.as_slice(), order, next, now)
            .map_err(FoldError::Gate)?;

        from_order_status(applied.status).ok_or(FoldError::BadPayload)
    }
}

/// Build a fresh kernel `Order` at `start` status with a single priced line, for
/// use as a receiver's local order state.
pub fn fresh_order(order_id: &str, start: DeliveryStatus) -> Order {
    let order = place_order(
        order_id.to_string(),
        None,
        vec![OrderItem {
            product_id: "sku-p13".to_string(),
            modifier_ids: vec![],
            quantity: 1,
            unit_price: 1000,
            currency: Currency::Usd,
            vendor_id: VendorId(13),
        }],
        0,
        None,
        None,
    )
    .expect("place_order");
    // `place_order` always starts at Pending; re-apply to reach `start` if needed.
    if start == DeliveryStatus::Pending {
        return order;
    }
    // Drive the order to `start` with a minimal legal prefix (for tests that
    // begin mid-lifecycle). Folds through the kernel Law directly.
    let target = to_order_status(start);
    apply_event(&order, target).expect("prefold to start status")
}

#[allow(unused_imports)]
use keygen as _keygen_marker;

#[cfg(all(feature = "kernel-rlib", test))]
mod tests {
    use super::*;

    /// The full legal lifecycle from Pending → Delivered.
    fn lifecycle(order_id: u64) -> Vec<(DeliveryStatus, DeliveryStatus)> {
        vec![
            (DeliveryStatus::Pending, DeliveryStatus::Confirmed),
            (DeliveryStatus::Confirmed, DeliveryStatus::Preparing),
            (DeliveryStatus::Preparing, DeliveryStatus::Ready),
            (DeliveryStatus::Ready, DeliveryStatus::InDelivery),
            (DeliveryStatus::InDelivery, DeliveryStatus::Delivered),
        ]
        .into_iter()
        .map(|(f, t)| (f, t))
        .collect::<Vec<_>>()
        // re-tag with the order id at apply time
        .iter()
        .map(|(f, t)| (*f, *t))
        .collect()
    }

    // ── AC-1 GREEN: one signed envelope, folded on TWO independent hubs
    // (owner + replica), both arrive at Delivered. Different nodes, different
    // DOD sets, different local order state — convergence comes from the protocol
    // Law, not from a shared store. This is the cross-hub proof the blueprint
    // §3 requires, exercised against the REAL kernel Law.
    #[test]
    fn ac1_owner_signed_frame_folds_on_two_hubs() {
        let edge = IntakeEdge::new(0x21);
        let mut hub_owner = edge.receiver();
        let mut hub_replica = edge.receiver();

        // Each hub owns its OWN local order state, initialized independently.
        let mut order_owner = fresh_order("ord-ac1", DeliveryStatus::Pending);
        let mut order_replica = fresh_order("ord-ac1", DeliveryStatus::Pending);

        // The owner emits the whole lifecycle; BOTH hubs fold every frame.
        for (from, to) in lifecycle(1) {
            let frame = edge.emit(1, from, to);
            let now = 1_000_000;
            let r_owner = hub_owner
                .admit_and_fold(&frame, &order_owner, now)
                .expect("owner folds");
            let r_replica = hub_replica
                .admit_and_fold(&frame, &order_replica, now)
                .expect("replica folds");

            // Both hubs agree on the resulting status.
            assert_eq!(r_owner, to);
            assert_eq!(r_replica, to);
            assert_eq!(r_owner, r_replica, "cross-hub convergence");

            // Advance each hub's local order to the new status.
            order_owner.status = to_order_status(r_owner);
            order_replica.status = to_order_status(r_replica);
        }
        assert_eq!(order_owner.status, OrderStatus::Delivered);
        assert_eq!(order_replica.status, OrderStatus::Delivered);
    }

    // ── AC-2 GREEN (extension of lib.rs:179): a VALIDLY-signed frame carrying a
    // forged state jump (Pending → Delivered) is rejected by the kernel Law on
    // EVERY receiver. The signature is real; only the *intent* is illegal. The
    // receiver validates `assert_transition(order.status, next)` against its own
    // local state and refuses.
    #[test]
    fn ac2_forged_pending_to_delivered_rejected_everywhere() {
        let edge = IntakeEdge::new(0x22);

        // Owner emits a forged skip: Pending -> Delivered (illegal jump).
        let forged = edge.emit(2, DeliveryStatus::Pending, DeliveryStatus::Delivered);

        // Receiver A (owner) and Receiver B (a different replica) both start the
        // order at Pending.
        let mut recv_a = edge.receiver();
        let mut recv_b = edge.receiver();
        let order_a = fresh_order("ord-ac2", DeliveryStatus::Pending);
        let order_b = fresh_order("ord-ac2", DeliveryStatus::Pending);

        let now = 1_000_000;
        let ra = recv_a.admit_and_fold(&forged, &order_a, now);
        let rb = recv_b.admit_and_fold(&forged, &order_b, now);

        // Both reject — and specifically at the LAW gate (not DOD/wire).
        assert!(matches!(ra, Err(FoldError::Gate(Reject::Law(_)))), "A: {:?}", ra);
        assert!(matches!(rb, Err(FoldError::Gate(Reject::Law(_)))), "B: {:?}", rb);

        // And an *unauthorized* subject (key NOT in the delegation chain) cannot
        // even pass the WIRE gate.
        let stranger_seed = [0x77u8; 32];
        let (stranger_pk, _) = ed_keygen(&stranger_seed);
        let cap = Capability::new_hybrid(
            stranger_pk,
            edge.pq_pk.clone(),
            Resource::Order,
            Action::OrderStatusChanged,
            edge.nonce,
            edge.expiry,
        );
        let mut bad = SignedFrame::new(cap, OrderTransition { order_id: 2, from: DeliveryStatus::Pending, to: DeliveryStatus::Confirmed }.to_bytes().to_vec());
        bad.delegation_chain = edge.chain().to_vec();
        bad.sign_classical(&stranger_seed).unwrap();
        bad.sign_pq(&edge.pq_sk, &[0u8; 32]).unwrap();
        let mut recv_c = edge.receiver();
        let order_c = fresh_order("ord-ac2c", DeliveryStatus::Pending);
        let rc = recv_c.admit_and_fold(&bad, &order_c, now);
        assert!(rc.is_err(), "stranger frame rejected at wire or law: {:?}", rc);
    }

    // ── AC-6 GREEN (solo-island, F50): the FULL order-to-delivery flow completes
    // with ZERO other hubs. One process, one intake edge, one receiver — the
    // owner IS the only hub. Proves the spine needs no peer to settle delivery.
    #[test]
    fn ac6_solo_island_full_flow_no_peers() {
        // A single-hub ring (R=0): the owner is the only member.
        let edge = IntakeEdge::new(0x30);
        let mut recv = edge.receiver();
        let mut order = fresh_order("ord-solo", DeliveryStatus::Pending);

        // Drive the whole lifecycle locally, no network, no second hub.
        for (from, to) in lifecycle(3) {
            let frame = edge.emit(3, from, to);
            let now = 2_000_000;
            let status = recv
                .admit_and_fold(&frame, &order, now)
                .expect("solo fold step");
            assert_eq!(status, to);
            order.status = to_order_status(status);
        }
        assert_eq!(order.status, OrderStatus::Delivered);
    }

    // ── DOD integration: an empty-payload frame (void/no-op echo) is refused by
    // the mesh-node DOD gate BEFORE the wire gate runs — fail-closed hygiene.
    #[test]
    fn ac1_dod_rejects_empty_payload_before_wire() {
        let edge = IntakeEdge::new(0x31);
        let mut recv = edge.receiver();

        // Build a frame with an EMPTY payload (the DOD's EmptyPayload fault).
        let cap = Capability::new_hybrid(
            edge.leaf_pk,
            edge.pq_pk.clone(),
            Resource::Order,
            Action::OrderStatusChanged,
            edge.nonce,
            edge.expiry,
        );
        let mut frame = SignedFrame::new(cap, Vec::new());
        frame.delegation_chain = edge.chain().to_vec();
        frame.sign_classical(&edge.leaf_seed).unwrap();
        frame.sign_pq(&edge.pq_sk, &[0u8; 32]).unwrap();

        let order = fresh_order("ord-dod", DeliveryStatus::Pending);
        let res = recv.admit_and_fold(&frame, &order, 1_000_000);
        assert!(matches!(res, Err(FoldError::Dod(DodFault::EmptyPayload))));
    }

    // ── Replay hygiene: the SAME frame folded twice is rejected the second time
    // by the DOD replay set (idempotent dedup), even though its signature is
    // still valid.
    #[test]
    fn ac1_dod_replay_rejected_on_second_apply() {
        let edge = IntakeEdge::new(0x32);
        let mut recv = edge.receiver();
        let mut order = fresh_order("ord-replay", DeliveryStatus::Pending);

        let frame = edge.emit(4, DeliveryStatus::Pending, DeliveryStatus::Confirmed);
        let now = 1_000_000;
        let first = recv.admit_and_fold(&frame, &order, now);
        assert!(first.is_ok());
        order.status = to_order_status(first.unwrap());

        // Same frame again — DOD replay fault.
        let second = recv.admit_and_fold(&frame, &order, now);
        assert!(matches!(second, Err(FoldError::Dod(DodFault::Replay))));
    }

    // ── GREEN: a frame expired at `expires_at` is refused by the DOD lifetime
    // gate (now >= expiry), proving the BPv7 lifetime check is live on the spine.
    #[test]
    fn ac1_dod_expired_frame_rejected() {
        let edge = IntakeEdge::new(0x33);
        let mut recv = edge.receiver();
        let order = fresh_order("ord-exp", DeliveryStatus::Pending);
        let frame = edge.emit(5, DeliveryStatus::Pending, DeliveryStatus::Confirmed);
        // `now` far past the capability expiry (9_999_999_999).
        let res = recv.admit_and_fold(&frame, &order, 10_000_000_000);
        assert!(matches!(res, Err(FoldError::Dod(DodFault::Expired))));
    }
}
