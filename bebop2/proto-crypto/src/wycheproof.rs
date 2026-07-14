//! Wycheproof — cross-check primitives against Google Wycheproof vectors.
//!
//! TODO(P0-6/H): harness skeleton that loads Wycheproof JSON vectors and asserts
//! `bebop2-core` primitives match (KAT-style, committed test vectors). No
//! network: vectors are vendored under `kat/` like `bebop2-core/src/kat/`
//! (which already holds `vectors.rs` + `vectors_long.rs`).
//!
//! CI GUARD: NO-COURIER-SCORING — test vectors verify math, not reputation.

#[derive(Debug, Default, Clone, Copy)]
pub struct Placeholder;
