//! Claim machine — the courier-claim `decide/fold` Law (MESH-04).
//!
//! Pure Law, **NO kernel dependency**. Mirrors the shape of
//! `dowiz_kernel::order_machine` (its `assert_transition` / `fold_transitions`)
//! but for the courier-claim lifecycle:
//!
//! ```text
//! Offered ──accept──▶ Claimed ──released──▶ Released (terminal-legal)
//!                          │
//!                          └──picked_up──▶ PickedUp (terminal-legal)
//! ```
//!
//! Structural constraint enforced here and NOWHERE ELSE: **NO-COURIER-SCORING**.
//! The claim state carries no score / rating / trust / reputation / rank field.
//! A claim is a pure coordination record (who is bound to which order), not a
//! judgement about the courier. The CI gate `scripts/ci-no-courier-scoring.sh`
//! double-locks this.

/// A courier claim's lifecycle state. Plain C-like enum (no scoring fields).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ClaimStatus {
    /// The order has been offered to a courier; no acceptance yet.
    Offered,
    /// The courier accepted the claim (bound to the order).
    Claimed,
    /// The claim was released (e.g. courier dropped it, or requeue). Terminal-legal.
    Released,
    /// The courier picked up the order. Terminal-legal.
    PickedUp,
}

impl ClaimStatus {
    /// Pinned wire byte (wire-stable; not compiler-chosen).
    pub fn discriminant(&self) -> u8 {
        match self {
            ClaimStatus::Offered => 0x20,
            ClaimStatus::Claimed => 0x21,
            ClaimStatus::Released => 0x22,
            ClaimStatus::PickedUp => 0x23,
        }
    }
    pub fn from_discriminant(b: u8) -> Option<ClaimStatus> {
        Some(match b {
            0x20 => ClaimStatus::Offered,
            0x21 => ClaimStatus::Claimed,
            0x22 => ClaimStatus::Released,
            0x23 => ClaimStatus::PickedUp,
            _ => return None,
        })
    }
}

/// Transition error for the claim machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClaimError {
    /// from === to
    SameStatus(ClaimStatus),
    /// not in the allowed transition table
    Illegal(ClaimStatus, ClaimStatus),
}

impl ClaimError {
    pub fn code(&self) -> &'static str {
        match self {
            ClaimError::SameStatus(_) => "SameClaimStatus",
            ClaimError::Illegal(_, _) => "IllegalClaimTransition",
        }
    }
}

/// Allowed next-states. Identical mirror of the order machine's table shape.
fn allowed_next(from: ClaimStatus) -> &'static [ClaimStatus] {
    use ClaimStatus::*;
    match from {
        Offered => &[Claimed, Released],
        Claimed => &[Released, PickedUp],
        // Terminal-legal: Released and PickedUp have no outgoing edges.
        Released => &[],
        PickedUp => &[],
    }
}

/// The `decide` half of the claim Law: validate a single transition.
/// Returns `Err` (never panics) on an illegal or same-status transition.
pub fn assert_transition(from: ClaimStatus, to: ClaimStatus) -> Result<(), ClaimError> {
    if from == to {
        return Err(ClaimError::SameStatus(from));
    }
    if allowed_next(from).contains(&to) {
        Ok(())
    } else {
        Err(ClaimError::Illegal(from, to))
    }
}

/// The `fold` half: reduce a sequence of claim transitions to a final status,
/// stopping at the first illegal transition (returns the error + status reached).
pub fn fold_transitions(
    start: ClaimStatus,
    steps: &[ClaimStatus],
) -> Result<ClaimStatus, (ClaimError, ClaimStatus)> {
    let mut cur = start;
    for &next in steps {
        assert_transition(cur, next).map_err(|e| (e, cur))?;
        cur = next;
    }
    Ok(cur)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── R-MESH04a: illegal claim transition rejected.
    #[test]
    fn r_mesh04_illegal_offered_to_pickedup() {
        // Offered cannot jump straight to PickedUp (must Claim first).
        assert!(matches!(
            assert_transition(ClaimStatus::Offered, ClaimStatus::PickedUp),
            Err(ClaimError::Illegal(_, _))
        ));
    }

    #[test]
    fn r_mesh04_released_cannot_move() {
        // Released is terminal-legal: any further transition is illegal.
        assert!(matches!(
            assert_transition(ClaimStatus::Released, ClaimStatus::Claimed),
            Err(ClaimError::Illegal(_, _))
        ));
    }

    #[test]
    fn r_mesh04_same_status_rejected() {
        assert!(matches!(
            assert_transition(ClaimStatus::Claimed, ClaimStatus::Claimed),
            Err(ClaimError::SameStatus(_))
        ));
    }

    // ── GREEN: PickedUp and Released are terminal-legal end states.
    #[test]
    fn green_mesh04_offered_claimed_pickedup_legal() {
        let path = [ClaimStatus::Claimed, ClaimStatus::PickedUp];
        assert_eq!(
            fold_transitions(ClaimStatus::Offered, &path),
            Ok(ClaimStatus::PickedUp)
        );
    }

    #[test]
    fn green_mesh04_offered_released_legal() {
        let path = [ClaimStatus::Released];
        assert_eq!(
            fold_transitions(ClaimStatus::Offered, &path),
            Ok(ClaimStatus::Released)
        );
    }

    #[test]
    fn green_mesh04_claimed_released_legal() {
        let path = [ClaimStatus::Released];
        assert_eq!(
            fold_transitions(ClaimStatus::Claimed, &path),
            Ok(ClaimStatus::Released)
        );
    }

    #[test]
    fn green_mesh04_fold_stops_at_first_illegal() {
        // Offered -> Claimed -> PickedUp -> (illegal) Released: PickedUp is terminal.
        let res = fold_transitions(
            ClaimStatus::Offered,
            &[
                ClaimStatus::Claimed,
                ClaimStatus::PickedUp,
                ClaimStatus::Released,
            ],
        );
        assert!(matches!(
            res,
            Err((ClaimError::Illegal(_, _), ClaimStatus::PickedUp))
        ));
    }

    #[test]
    fn green_mesh04_discriminants_pinned() {
        assert_eq!(ClaimStatus::Offered.discriminant(), 0x20);
        assert_eq!(ClaimStatus::Claimed.discriminant(), 0x21);
        assert_eq!(ClaimStatus::Released.discriminant(), 0x22);
        assert_eq!(ClaimStatus::PickedUp.discriminant(), 0x23);
    }
}
