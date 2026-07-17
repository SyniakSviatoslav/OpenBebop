//! Wire frame-kind registry (P9-owned; P10 requests one discriminant).
//!
//! Phase 9 owns the on-the-wire `FrameKind` registry (M6/M10). Every frame
//! carried on the mesh is tagged with a single-byte, **pinned** discriminant so
//! carriers and handlers can dispatch a frame to the right handler without
//! guessing from payload shape. The bytes are part of the wire contract — never
//! renumber an existing variant; only append.
//!
//! # P10 request — `FrameKind::OperatorKill`
//! Phase 10 (HUB RUNTIME — kill-switch) requests exactly one new discriminant:
//! [`FrameKind::OperatorKill`] (0x02). The kill order rides inside the existing
//! `proto-cap::SignedFrame` envelope (which already carries the hybrid
//! Ed25519⊕ML-DSA signature legs), so no new crypto is invented on the wire —
//! only a new payload KIND and a hub-side handler (in `mesh-node`).
//!
//! Fail-closed: an unknown discriminant byte decodes to `None`, never a default.
//!
//! CI GUARD: NO-COURIER-SCORING — a frame kind is a dispatch tag, never a score.

/// A pinned, single-byte frame-kind discriminant carried on the wire.
///
/// Closed set so dispatch is exhaustively checkable. Append-only: adding a
/// variant is a forward-compatible wire change; renumbering an existing one is a
/// breaking change and must never happen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameKind {
    /// A normal data / capability frame (the default carried payload today).
    Data,
    /// P10 (M9/F28): an operator kill order. Anchor-signed; carries a
    /// canonical-TLV `KillOrder` payload. The hub-side handler (mesh-node)
    /// verifies it against the genesis roster's `kill`-scoped anchors and runs
    /// the COLD-backup-THEN-halt sequence. Pinned discriminant 0x02.
    OperatorKill,
    /// P10 (M5/F1): a signed operator policy update — a new `HubPolicy` revision
    /// pushed over the mesh (same signature path as `OperatorKill`). Pinned 0x03.
    PolicyUpdate,
}

impl FrameKind {
    /// Explicit discriminant byte (pinned; not compiler-chosen).
    pub fn discriminant(&self) -> u8 {
        match self {
            FrameKind::Data => 0x01,
            FrameKind::OperatorKill => 0x02,
            FrameKind::PolicyUpdate => 0x03,
        }
    }

    /// Inverse of [`FrameKind::discriminant`]. `None` for unknown bytes so the
    /// dispatch boundary is fail-closed (no default/panic on a malformed kind).
    pub fn from_discriminant(b: u8) -> Option<FrameKind> {
        match b {
            0x01 => Some(FrameKind::Data),
            0x02 => Some(FrameKind::OperatorKill),
            0x03 => Some(FrameKind::PolicyUpdate),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // The discriminant bytes are part of the wire contract — pin them.
    #[test]
    fn frame_kind_discriminants_are_stable() {
        assert_eq!(FrameKind::Data.discriminant(), 0x01);
        assert_eq!(FrameKind::OperatorKill.discriminant(), 0x02);
        assert_eq!(FrameKind::PolicyUpdate.discriminant(), 0x03);
    }

    #[test]
    fn frame_kind_roundtrips_and_is_fail_closed() {
        for k in [
            FrameKind::Data,
            FrameKind::OperatorKill,
            FrameKind::PolicyUpdate,
        ] {
            assert_eq!(FrameKind::from_discriminant(k.discriminant()), Some(k));
        }
        // Unknown byte => None (fail-closed, never a default).
        assert_eq!(FrameKind::from_discriminant(0x00), None);
        assert_eq!(FrameKind::from_discriminant(0xFF), None);
    }

    // P10 acceptance: the OperatorKill kind exists and is distinct from Data.
    #[test]
    fn operator_kill_kind_exists_and_is_distinct() {
        assert_ne!(FrameKind::OperatorKill, FrameKind::Data);
        assert_ne!(
            FrameKind::OperatorKill.discriminant(),
            FrameKind::Data.discriminant()
        );
    }
}
