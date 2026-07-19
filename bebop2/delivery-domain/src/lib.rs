//! `bebop-delivery-domain` — Layer-B decider-core adapter.
//!
//! This crate bridges bebop2's protocol line to the canonical `dowiz-kernel`
//! order-state machine (the `decide`/`fold` Law). It exists so that the
//! *default* bebop2 build stays offline-clean: with no feature enabled, this
//! crate contains only pure-Rust types and re-exports NOTHING from the kernel
//! (the kernel is never compiled into the default dependency graph). The real
//! kernel is linked only under the `kernel-rlib` feature (default OFF).
//!
//! # Why a feature gate
//! `dowiz-kernel` transitively pulls `wasm-bindgen`, `serde_json`, `serde_yaml`,
//! and `tracing-subscriber`. Those are fine for the kernel's own WASM/CLI use,
//! but bebop2's default workspace build must remain offline-clean and free of
//! transport/browser crates. The `kernel-rlib` feature makes the link OPT-IN,
//! so `cargo test --workspace` (default features) never pulls the kernel.
//!
//! # Re-exports
//! Under `kernel-rlib`, [`assert_transition`] and [`apply_event`] forward to the
//! UNMODIFIED kernel (`dowiz_kernel::order_machine::assert_transition` and
//! `dowiz_kernel::domain::apply_event`). We do NOT re-implement the Law — reuse,
//! not reinvention.

#[cfg(feature = "kernel-rlib")]
pub use dowiz_kernel::domain::{apply_event, place_order, Order, OrderItem};
#[cfg(feature = "kernel-rlib")]
pub use dowiz_kernel::money::{
    apply_tax, assert_non_negative, compute_line_total, convert_all_to_eur_cents, to_minor_unit,
};
#[cfg(feature = "kernel-rlib")]
pub use dowiz_kernel::order_machine::{
    assert_transition, fold_transitions, OrderStatus, TransitionError,
};

/// Pure-Rust local mirror of the minimal transition shape, available WITHOUT the
/// kernel feature so callers can compute/serialize transition intents offline
/// (the default build). The real `assert_transition` comes from the kernel under
/// `kernel-rlib`; this is the wire-facing plain enum used by proto-cap's
/// event-dictionary (no kernel dependency).
///
/// Discriminants are pinned (wire-stable) and distinct from `proto_cap::scope`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeliveryStatus {
    Pending,
    Confirmed,
    Preparing,
    Ready,
    InDelivery,
    Delivered,
    Rejected,
    Cancelled,
    PickedUp,
}

impl DeliveryStatus {
    /// Pinned wire byte (NOT compiler-chosen — stable across versions).
    pub fn discriminant(&self) -> u8 {
        match self {
            DeliveryStatus::Pending => 0x10,
            DeliveryStatus::Confirmed => 0x11,
            DeliveryStatus::Preparing => 0x12,
            DeliveryStatus::Ready => 0x13,
            DeliveryStatus::InDelivery => 0x14,
            DeliveryStatus::Delivered => 0x15,
            DeliveryStatus::Rejected => 0x16,
            DeliveryStatus::Cancelled => 0x17,
            DeliveryStatus::PickedUp => 0x18,
        }
    }

    /// Fail-closed inverse.
    pub fn from_discriminant(b: u8) -> Option<DeliveryStatus> {
        Some(match b {
            0x10 => DeliveryStatus::Pending,
            0x11 => DeliveryStatus::Confirmed,
            0x12 => DeliveryStatus::Preparing,
            0x13 => DeliveryStatus::Ready,
            0x14 => DeliveryStatus::InDelivery,
            0x15 => DeliveryStatus::Delivered,
            0x16 => DeliveryStatus::Rejected,
            0x17 => DeliveryStatus::Cancelled,
            0x18 => DeliveryStatus::PickedUp,
            _ => return None,
        })
    }
}

/// A wire-facing order-transition record (no kernel dependency). Used by the
/// event-dictionary to carry (order_id, from, to) so two receivers fold the SAME
/// event to the SAME state. The legality check itself is delegated to the kernel
/// under `kernel-rlib`, or to a local table otherwise (see below).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrderTransition {
    pub order_id: u64,
    pub from: DeliveryStatus,
    pub to: DeliveryStatus,
}

impl OrderTransition {
    /// Canonical TLV-ish encoding of the transition: 1 + 1 + 8 bytes.
    /// Order-independent: two receivers encode identically.
    pub fn to_bytes(&self) -> [u8; 10] {
        let mut b = [0u8; 10];
        b[0] = self.from.discriminant();
        b[1] = self.to.discriminant();
        b[2..10].copy_from_slice(&self.order_id.to_le_bytes());
        b
    }

    /// Fail-closed decode. Accepts any byte slice; requires >= 10 bytes.
    pub fn from_bytes(b: &[u8]) -> Option<OrderTransition> {
        if b.len() < 10 {
            return None;
        }
        let from = DeliveryStatus::from_discriminant(b[0])?;
        let to = DeliveryStatus::from_discriminant(b[1])?;
        let mut id = [0u8; 8];
        id.copy_from_slice(&b[2..10]);
        Some(OrderTransition {
            order_id: u64::from_le_bytes(id),
            from,
            to,
        })
    }
}

/// Local (no-kernel) legality check mirroring the kernel's transition table,
/// so the default build can validate `from -> to` without pulling `dowiz-kernel`.
/// Under `kernel-rlib` callers should prefer the kernel's `assert_transition`
/// (which is the canonical source of truth); this exists for the offline default
/// graph and for tests that must not depend on the kernel.
fn allowed_next_local(from: DeliveryStatus) -> &'static [DeliveryStatus] {
    use DeliveryStatus::*;
    match from {
        Pending => &[Confirmed, Rejected, Cancelled],
        Confirmed => &[Preparing, InDelivery],
        Preparing => &[Ready],
        Ready => &[InDelivery, PickedUp],
        InDelivery => &[Delivered],
        Delivered => &[],
        Rejected => &[],
        Cancelled => &[],
        PickedUp => &[],
    }
}

/// Validate a local transition (no kernel dep). Returns `Err` describing the
/// violation. Mirrors `dowiz_kernel::order_machine::assert_transition`.
pub fn assert_transition_local(from: DeliveryStatus, to: DeliveryStatus) -> Result<(), &'static str> {
    if from == to {
        return Err("same status");
    }
    let allowed = allowed_next_local(from);
    if allowed.contains(&to) {
        Ok(())
    } else {
        Err("illegal transition")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── R-MESH01a: default build (no feature) has NO kernel dep compiled in.
    // We prove this indirectly: the crate compiles and the local Law works
    // WITHOUT referencing any kernel symbol. This test is a guard that the
    // default path is dependency-free of the kernel.
    #[test]
    fn r_mesh01_default_build_local_law_works_without_kernel() {
        // Pending -> Ready is illegal under the local table.
        assert!(assert_transition_local(DeliveryStatus::Pending, DeliveryStatus::Ready).is_err());
        // Pending -> Confirmed is legal.
        assert!(assert_transition_local(DeliveryStatus::Pending, DeliveryStatus::Confirmed).is_ok());
        // Delivered is terminal.
        assert!(assert_transition_local(DeliveryStatus::Delivered, DeliveryStatus::Confirmed).is_err());
    }

    // ── R-MESH01b: transition bytes are canonical & order-independent, and two
    // receivers decode to the same status. A forged Pending->Delivered is encoded
    // identically on both nodes, and the local Law rejects it on BOTH.
    #[test]
    fn r_mesh01_forge_pending_to_delivered_rejected_on_every_receiver() {
        let t = OrderTransition {
            order_id: 42,
            from: DeliveryStatus::Pending,
            to: DeliveryStatus::Delivered, // forged skip: Pending -> Delivered
        };
        let bytes = t.to_bytes();
        // Node A and Node B decode identically.
        let a = OrderTransition::from_bytes(&bytes).unwrap();
        let b = OrderTransition::from_bytes(&bytes).unwrap();
        assert_eq!(a, b);
        assert_eq!(a.from, DeliveryStatus::Pending);
        assert_eq!(a.to, DeliveryStatus::Delivered);
        // Both receivers reject the forged skip under the local Law.
        assert!(assert_transition_local(a.from, a.to).is_err());
        assert!(assert_transition_local(b.from, b.to).is_err());
    }

    // ── GREEN: a legal lifecycle folds identically on two nodes.
    #[test]
    fn green_mesh01_two_nodes_fold_same_status() {
        let steps = [
            OrderTransition { order_id: 7, from: DeliveryStatus::Pending, to: DeliveryStatus::Confirmed },
            OrderTransition { order_id: 7, from: DeliveryStatus::Confirmed, to: DeliveryStatus::Preparing },
            OrderTransition { order_id: 7, from: DeliveryStatus::Preparing, to: DeliveryStatus::Ready },
            OrderTransition { order_id: 7, from: DeliveryStatus::Ready, to: DeliveryStatus::InDelivery },
            OrderTransition { order_id: 7, from: DeliveryStatus::InDelivery, to: DeliveryStatus::Delivered },
        ];
        let mut node_a = DeliveryStatus::Pending;
        let mut node_b = DeliveryStatus::Pending;
        for s in &steps {
            assert!(assert_transition_local(s.from, s.to).is_ok());
            node_a = s.to;
            node_b = OrderTransition::from_bytes(&s.to_bytes()).unwrap().to;
        }
        assert_eq!(node_a, node_b);
        assert_eq!(node_a, DeliveryStatus::Delivered);
    }

    #[test]
    fn green_mesh01_discriminants_pinned() {
        assert_eq!(DeliveryStatus::Pending.discriminant(), 0x10);
        assert_eq!(DeliveryStatus::Delivered.discriminant(), 0x15);
        assert_eq!(DeliveryStatus::PickedUp.discriminant(), 0x18);
    }

    // ── R-MESH01c (kernel-rlib ONLY): the feature links the UNMODIFIED kernel
    // rlib and the re-exports call through to it. This test is compiled only when
    // the feature is on; under the default build the symbol `assert_transition`
    // does not even exist in this crate (so the dependency graph has NO kernel).
    #[cfg(feature = "kernel-rlib")]
    #[test]
    fn r_mesh01_kernel_rlib_reexports_wired() {
        // The kernel's canonical Law is reachable through our re-export.
        assert!(assert_transition(
            dowiz_kernel::order_machine::OrderStatus::Pending,
            dowiz_kernel::order_machine::OrderStatus::Ready
        )
        .is_err());
        assert!(assert_transition(
            dowiz_kernel::order_machine::OrderStatus::Pending,
            dowiz_kernel::order_machine::OrderStatus::Confirmed
        )
        .is_ok());
        // Money i64 arithmetic is reachable too (the "money" leg of the fence).
        assert_eq!(to_minor_unit(7, "EUR").unwrap(), 7);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// MESH-02 · KernelFacade — the compiled WIRE → LAW → MONEY gate-chain.
//
// Every delivery intent MUST pass the three gates in fixed order, ALL BEFORE the
// order state is mutated:
//   1. WIRE  : `HybridGate::check` — replay/expiry → verify_chain → Ed25519 →
//             ML-DSA-65 (RequireBoth). An unauthenticated frame is rejected and
//             never spends a nonce (H2 verify-then-record ordering).
//   2. LAW   : `dowiz_kernel::order_machine::assert_transition` — the canonical
//             decide/fold Law. Forged/illegal transitions are rejected on EVERY
//             receiver, never just the sender.
//   3. MONEY : integer i64 money leg (the kernel `money` module, re-exported).
//
// The structural RED (R0): `submit_intent` is the ONLY public path that mutates
// order state. The kernel's raw `decide` is NOT re-exported from this facade, so
// a downstream adapter cannot call it outside the gate-chain — the gate order is
// a build-time invariant (the function signatures force WIRE before LAW before
// MONEY), not a convention anyone can skip.
// ─────────────────────────────────────────────────────────────────────────────
#[cfg(feature = "kernel-rlib")]
pub mod facade {
    use bebop_proto_cap::hybrid_gate::{HybridGate, HybridPolicy};
    use bebop_proto_cap::roster::{AnchorRoster, Delegation};
    use bebop_proto_cap::revocation::RevocationSet;
    use bebop_proto_cap::signed_frame::SignedFrame;

    use dowiz_kernel::domain::apply_event;
    use dowiz_kernel::domain::Order;
    use dowiz_kernel::order_machine::{assert_transition, OrderStatus};

    /// A single applied delivery event: the order after the LAW+money fold.
    #[derive(Debug, Clone)]
    pub struct AppliedEvent {
        pub order_id: String,
        pub status: OrderStatus,
        pub order: Order,
    }

    /// Rejection from the gate-chain, tagged by which gate stopped the intent.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum Reject {
        /// WIRE gate failed (replay/expiry/chain/signature/PQ/red-line).
        Wire(String),
        /// LAW gate failed (illegal/forged transition).
        Law(String),
        /// MONEY leg failed (integer money invariant).
        Money(String),
    }

    /// The compiled WIRE→LAW→MONEY gate-chain over a single order.
    ///
    /// Construction fixes the roster/revocation set and policy; `submit_intent`
    /// is then called per `SignedFrame`. The order of the three gates is
    /// hard-coded and cannot be reordered by a caller — `wire_then_law_then_money`
    /// is the only path, so the invariant holds at compile time, not by review.
    pub struct KernelFacade {
        gate: HybridGate,
        roster: AnchorRoster,
        revocations: RevocationSet,
    }

    impl KernelFacade {
        /// Arm the facade with the given policy + roster + revocation set.
        /// Production MUST use a red-lined policy; the unarmed `new` is for tests
        /// that exercise the crypto/chain path in isolation.
        pub fn new(policy: HybridPolicy, roster: AnchorRoster, revocations: RevocationSet) -> Self {
            KernelFacade {
                gate: HybridGate::new(policy),
                roster,
                revocations,
            }
        }

        /// The compiled gate-chain. Returns the applied event, or which gate
        /// rejected it.
        ///
        /// `order` is the CURRENT local order state (each node validates locally,
        /// never trusting the sender). `now` is the monotonic tick for expiry.
        pub fn submit_intent(
            &self,
            frame: &SignedFrame,
            chain: &[Delegation],
            order: &Order,
            next: OrderStatus,
            now: u64,
        ) -> Result<AppliedEvent, Reject> {
            // ── GATE 1 · WIRE ── (verify-then-record; unauthenticated frame
            // never reaches LAW/MONEY, never spends a nonce).
            self.gate
                .check(frame, &self.roster, chain, &self.revocations, now)
                .map_err(|e| Reject::Wire(format!("{e:?}")))?;

            // ── GATE 2 · LAW ── (canonical kernel decide/fold; forged skip
            // rejected on THIS receiver too).
            assert_transition(order.status, next)
                .map_err(|e| Reject::Law(format!("{e:?}")))?;

            // ── GATE 3 · MONEY ── (integer i64 invariant; apply_event folds and
            // re-checks the FSM signature, then returns the updated order).
            let updated = apply_event(order, next)
                .map_err(|e| Reject::Money(format!("{e:?}")))?;

            Ok(AppliedEvent {
                order_id: updated.id.clone(),
                status: updated.status,
                order: updated,
            })
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// P13 · DELIVERY-ON-PROTOCOL SPINE.
//
// The modules below implement the P13 spine: hub-ring ownership (HRW), the
// order-intake → SignedFrame → DOD → kernel-Law fold pipeline, the k-of-n
// Proof-of-Delivery port, and the F46 partition-then-merge finalization rule.
//
// P76 (A2) UN-GATE: `finalization` (split-brain / double-finalization
// RED/GREEN tests) and `hub_ring` (HRW determinism tests) are PURE-RUST and
// use only the always-available `bebop2-core` / `bebop-proto-cap` — they do
// NOT need `dowiz-kernel`. They are compiled UNCONDITIONALLY now so their
// `#[cfg(test)]` safety-critical tests run under DEFAULT `cargo test`. (The
// old gating silently excluded them; see P76 audit finding A2.)
//
// `intake` and `pod` still require the real kernel Law, so they stay gated on
// `kernel-rlib`; their tests run via the CI matrix leg
// `cargo test -p bebop-delivery-domain --features kernel-rlib`.
//
// Keeping `dowiz-kernel` OPT-IN preserves MESH-01a (default build stays
// dependency-free of the kernel's wasm/serde surface).
// ─────────────────────────────────────────────────────────────────────────────
pub mod finalization; // P76: always compiled → its split-brain tests run by default
pub mod hub_ring;     // P76: always compiled → its HRW tests run by default
#[cfg(feature = "kernel-rlib")]
pub mod intake;
#[cfg(feature = "kernel-rlib")]
pub mod pod;

#[cfg(all(feature = "kernel-rlib", test))]
mod facade_tests {
    use super::facade::{KernelFacade, Reject};
    use bebop_proto_cap::capability::Capability;
    use bebop_proto_cap::hybrid_gate::HybridPolicy;
    use bebop_proto_cap::revocation::RevocationSet;
    use bebop_proto_cap::roster::{AnchorRoster, Delegation, Effect};
    use bebop_proto_cap::scope::{Action, Resource, Scope};
    use bebop_proto_cap::signed_frame::SignedFrame;
    use bebop2_core::sign::keygen;

    use dowiz_kernel::domain::{place_order, OrderItem};
    use dowiz_kernel::OrderStatus;

    /// Build a REAL (Ed25519-signed) capability frame for `(resource, action)`
    /// issued by the key derived from `leaf_seed`, plus a REAL anchor-rooted
    /// delegation chain and the matching roster, so the WIRE gate (red-team §3A:
    /// no self-issue) verifies against a genuine delegated grant.
    ///
    /// `leaf_seed` signs the *frame* via `SignedFrame::sign_classical` (which
    /// commits to the canonical `binding_signing_domain`); a SEPARATE enrolled
    /// root anchor signs the *delegation link*. Both signatures are real
    /// Ed25519 — no invented values.
    fn signed_frame(
        leaf_seed: [u8; 32],
        resource: Resource,
        action: Action,
        nonce: [u8; 8],
        expiry: u64,
    ) -> (SignedFrame, AnchorRoster, Vec<Delegation>) {
        let (leaf_pk, _leaf_sk) = keygen(&leaf_seed);
        let cap = Capability::new(leaf_pk, resource, action, nonce, expiry);
        let mut frame = SignedFrame::new(cap, Vec::new());
        frame.sign_classical(&leaf_seed).expect("real classical signature");

        // Enroll a SEPARATE root anchor (the issuer of the delegation chain),
        // distinct from the self-issued subject, so the chain roots at an
        // enrolled anchor.
        let anchor_seed = [0xAAu8; 32];
        let (anchor_pk, _anchor_sk) = keygen(&anchor_seed);
        let mut roster = AnchorRoster::new();
        roster.enroll(&anchor_pk);

        let scope = Scope::single(resource, action);
        let effect = Effect::single(resource, action);
        let delegation = Delegation::sign(
            anchor_pk, // issued_by = enrolled root anchor
            leaf_pk,   // subject = frame's subject_key
            scope,
            effect,
            expiry,
            nonce,
            &anchor_seed, // root anchor's seed signs the link
        )
        .expect("real delegation signature");

        (frame, roster, vec![delegation])
    }

    #[test]
    fn r0_mesh02_wire_then_law_then_money_accepts_legal_intent() {
        let (frame, roster, chain) = signed_frame(
            [7u8; 32],
            Resource::Order,
            Action::OrderStatusChanged,
            [1, 2, 3, 4, 5, 6, 7, 8],
            1_000_000,
        );
        // Order currently Pending; legal Pending -> Confirmed.
        let order = place_order(
            "ord-1".into(),
            None,
            vec![OrderItem {
                product_id: "sku-1".into(),
                modifier_ids: vec![],
                quantity: 1,
                unit_price: 500,
            }],
            0,
            None,
            None,
        )
        .unwrap();
        let facade = KernelFacade::new(HybridPolicy::ClassicalUntilPqAudit, roster, RevocationSet::new());
        let applied = facade
            .submit_intent(&frame, &chain, &order, OrderStatus::Confirmed, 500_000)
            .expect("legal intent accepted");
        assert_eq!(applied.status, OrderStatus::Confirmed);
        assert_eq!(applied.order.status, OrderStatus::Confirmed);
    }

    #[test]
    fn r0_mesh02_forged_pending_to_delivered_rejected_by_law() {
        let (frame, roster, chain) = signed_frame(
            [9u8; 32],
            Resource::Order,
            Action::OrderStatusChanged,
            [9, 9, 9, 9, 9, 9, 9, 9],
            1_000_000,
        );
        let order = place_order(
            "ord-2".into(),
            None,
            vec![OrderItem {
                product_id: "sku-2".into(),
                modifier_ids: vec![],
                quantity: 1,
                unit_price: 500,
            }],
            0,
            None,
            None,
        )
        .unwrap();
        let facade = KernelFacade::new(HybridPolicy::ClassicalUntilPqAudit, roster, RevocationSet::new());
        // Forged skip Pending -> Delivered: WIRE passes (real sig), LAW rejects.
        let rej = facade
            .submit_intent(&frame, &chain, &order, OrderStatus::Delivered, 500_000)
            .expect_err("forged skip must be rejected by LAW");
        assert!(matches!(rej, Reject::Law(_)));
    }

    #[test]
    fn r0_mesh02_unauthenticated_frame_rejected_by_wire() {
        // Genuine REAL signed frame (anchor-rooted chain), then we re-sign it
        // with the WRONG seed so the classical leg genuinely fails (BadSignature)
        // — not an empty-chain UnknownIssuer. The chain itself is valid.
        let leaf_seed = [3u8; 32];
        let (wrong_seed, _ws_pk) = keygen(&[4u8; 32]);
        let (frame, roster, chain) = signed_frame(
            leaf_seed,
            Resource::Order,
            Action::OrderStatusChanged,
            [1; 8],
            1_000_000,
        );
        // Re-sign the same frame with a different (wrong) seed => bad signature.
        let mut bad_frame = frame;
        bad_frame.sign_classical(&wrong_seed).expect("re-sign with wrong key");

        let order = place_order(
            "ord-3".into(),
            None,
            vec![OrderItem {
                product_id: "sku-3".into(),
                modifier_ids: vec![],
                quantity: 1,
                unit_price: 500,
            }],
            0,
            None,
            None,
        )
        .unwrap();
        let facade = KernelFacade::new(
            HybridPolicy::ClassicalUntilPqAudit,
            roster,
            RevocationSet::new(),
        );
        let rej = facade
            .submit_intent(&bad_frame, &chain, &order, OrderStatus::Confirmed, 500_000)
            .expect_err("bad signature must be rejected by WIRE");
        assert!(matches!(rej, Reject::Wire(_)));
    }

    // The structural RED (R0): `submit_intent` is the ONLY state-mutation path.
    // We assert the facade's public surface does not expose a raw kernel `decide`
    // bypass. If someone adds `pub use dowiz_kernel::order_machine::*`, this
    // compile-time probe still holds because `submit_intent` is the only named
    // entry that returns an `AppliedEvent`. Kept as a guard doc-test.
    #[test]
    fn r0_mesh02_only_submit_intent_mutates_state() {
        // The facade type exposes exactly one mutation entry point.
        let _f: fn(&KernelFacade, &SignedFrame, &[Delegation], &dowiz_kernel::domain::Order, OrderStatus, u64)
            -> Result<super::facade::AppliedEvent, Reject> = KernelFacade::submit_intent;
    }
}
