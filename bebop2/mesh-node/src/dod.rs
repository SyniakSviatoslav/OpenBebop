//! Definition-of-Done gate for mesh-carried events.
//!
//! The operator's standing architecture rule: **every event that crosses the mesh
//! MUST satisfy its Definition-of-Done before it is applied on the receiving
//! node.** This module is that gate, factored out so it is unit-testable in
//! isolation (RED→GREEN) and so the node runtime cannot "forget" to call it.
//!
//! A carried [`Event`] is DOD-clean only if ALL of:
//!   1. **Authored** — `payload` is non-empty (an event with no body is a
//!      no-op echo; reject it so a reconnect cannot replay a void frame).
//!   2. **Scoped** — `id` is non-zero (a zero id is the facade's
//!      `Projection` placeholder bag, never a real mutation; refuse it on the
//!      mesh so the placeholder can't masquerade as state change).
//!   3. **Fresh / non-replay** — `id` has not been applied before on THIS
//!      node (idempotent dedup; a replayed bundle collapses to one apply).
//!   4. **Within lifetime** — `expires_at == 0` (immortal control event) OR
//!      `now < expires_at` (the bundle has not expired in the BPv7 sense).
//!
//! Any single failure => `DodFault` and the node MUST NOT apply the event
//! (fail-closed, exactly like the facade's Wire→Law→money ordering).

use bebop_proto_cap::Event;
use std::collections::HashSet;

/// Why an event was refused at the DOD gate. Never a score; never grades the mover.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DodFault {
    /// Empty payload — a void frame.
    EmptyPayload,
    /// Zero id — the facade's placeholder `Projection`, not a real mutation.
    PlaceholderId,
    /// `id` already applied on this node (replay / duplicate delivery).
    Replay,
    /// Bundle expired (`now >= expires_at` and `expires_at != 0`).
    Expired,
}

/// The per-event Definition-of-Done gate.
///
/// Interior-mutable (`&self`) so it satisfies the same `EventSink`-style
/// contract the facade uses, and so a node can share one gate across its
/// concurrent recv task.
#[derive(Debug, Default)]
pub struct DodGate {
    /// `id`s already applied on this node (replay dedup set).
    applied: HashSet<u64>,
}

impl DodGate {
    /// New, empty gate (no events applied yet).
    pub fn new() -> Self {
        Self::default()
    }

    /// Check `event` against the DOD. On `Ok` the event is recorded as
    /// applied (so a future replay is refused). On `Err` nothing is recorded
    /// and the caller MUST drop the event.
    pub fn admit(&mut self, event: &Event, now: u64, expires_at: u64) -> Result<(), DodFault> {
        if event.payload.is_empty() {
            return Err(DodFault::EmptyPayload);
        }
        if event.id == 0 {
            return Err(DodFault::PlaceholderId);
        }
        if self.applied.contains(&event.id) {
            return Err(DodFault::Replay);
        }
        if expires_at != 0 && now >= expires_at {
            return Err(DodFault::Expired);
        }
        self.applied.insert(event.id);
        Ok(())
    }

    /// True iff `id` has already been admitted (used by tests / introspection).
    pub fn already_applied(&self, id: u64) -> bool {
        self.applied.contains(&id)
    }

    /// Number of distinct events admitted (used by the DOD-driven test oracle).
    pub fn admitted_count(&self) -> usize {
        self.applied.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bebop_proto_cap::Event;

    fn ev(id: u64, body: &[u8]) -> Event {
        Event {
            id,
            payload: body.to_vec(),
        }
    }

    // ── DOD-1: an event with no body is refused (REPLAY/void guard) ──────────
    #[test]
    fn red_empty_payload_refused() {
        let mut g = DodGate::new();
        let r = g.admit(&ev(1, b""), 0, 0);
        assert!(matches!(r, Err(DodFault::EmptyPayload)));
        assert_eq!(g.admitted_count(), 0);
    }

    // ── DOD-2: a zero-id placeholder is refused (never a real mutation) ───────
    #[test]
    fn red_placeholder_id_refused() {
        let mut g = DodGate::new();
        let r = g.admit(&ev(0, b"body"), 0, 0);
        assert!(matches!(r, Err(DodFault::PlaceholderId)));
        assert_eq!(g.admitted_count(), 0);
    }

    // ── DOD-3: a replayed id is refused on the SECOND apply (exactly-once) ───
    #[test]
    fn red_replay_refused_second_time() {
        let mut g = DodGate::new();
        assert!(g.admit(&ev(7, b"x"), 0, 0).is_ok());
        let r = g.admit(&ev(7, b"x"), 1, 0); // same id, later tick
        assert!(matches!(r, Err(DodFault::Replay)));
        assert_eq!(g.admitted_count(), 1); // only counted once
    }

    // ── DOD-4: an expired bundle (now >= expires_at, non-zero) is refused ─────
    #[test]
    fn red_expired_refused() {
        let mut g = DodGate::new();
        let r = g.admit(&ev(3, b"y"), 100, 50); // expires_at=50, now=100
        assert!(matches!(r, Err(DodFault::Expired)));
        assert_eq!(g.admitted_count(), 0);
    }

    // ── GREEN: a fresh, in-lifetime, well-formed event is admitted ───────────
    #[test]
    fn green_fresh_event_admitted() {
        let mut g = DodGate::new();
        assert!(g.admit(&ev(11, b"state"), 10, 9999).is_ok());
        assert_eq!(g.admitted_count(), 1);
        assert!(g.already_applied(11));
    }

    // ── GREEN: an immortal control event (expires_at == 0) never expires ─────
    #[test]
    fn green_immortal_event_never_expires() {
        let mut g = DodGate::new();
        assert!(g.admit(&ev(12, b"ctrl"), 9_999_999, 0).is_ok());
        assert_eq!(g.admitted_count(), 1);
    }
}
