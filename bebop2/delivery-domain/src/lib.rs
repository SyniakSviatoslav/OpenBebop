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

    /// Fail-closed decode.
    pub fn from_bytes(b: &[u8; 10]) -> Option<OrderTransition> {
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
