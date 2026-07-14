//! Constant-time marker — documented boundary for secret-dependent operations.
//!
//! Mirrors `bebop2-core`'s side-channel gate: no secret-dependent branch, no
//! secret-indexed table lookup. This module is the *marker + assert harness* so
//! the ladder can record which primitives are proven constant-time.
//!
//! TODO(P0-6/H): constant-time assertion shims / `#[cfg]` markers tying each
//! primitive to its constant-time evidence (clippy disallowed + asserts).
//!
//! CI GUARD: NO-COURIER-SCORING — side-channel posture is primitive-level, never
//! tied to any mover score.

#[derive(Debug, Default, Clone, Copy)]
pub struct Placeholder;
