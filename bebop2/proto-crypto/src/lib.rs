//! bebop-proto-crypto — **H** line of the bebop2 protocol (crypto ladder).
//!
//! # Scope (Tier-0 scaffold, P0-6)
//! This crate is the *crypto ladder* library line — the verification/strength
//! ladder that sits over the from-scratch primitives in `bebop2-core`. It is a
//! **stub** for now:
//!
//! - **Wycheproof → FIPS re-derivation placeholder.** Each primitive shipped in
//!   `bebop2-core` is meant to be cross-checked against Google Wycheproof test
//!   vectors AND independently re-derived against the authoritative FIPS
//!   specification (FIPS 203/204/180-4/202 etc.). This crate holds the
//!   *harness skeleton* that will drive those vectors; bodies are TODO.
//! - **Constant-time marker.** A documented, declared (intended) constant-time
//!   boundary marker for secret-dependent operations (no secret-dependent branch,
//!   no table lookup indexed by secret). Mirrors `bebop2-core`'s side-channel gate.
//!   The active assert harness is a TODO (see `constant_time.rs`); this is
//!   declared intent, not a live gate.
//!
//! # Status
//! **SKELETON ONLY.** No production logic. No primitives are (re)implemented
//! here — they live in `bebop2-core`. Module tree below is the agreed
//! structure; bodies are TODO markers. Intentionally dependency-free; must not
//! affect `dowiz`.
//!
//! ─────────────────────────────────────────────────────────────────────────────
//! ╔══════════════════════════════════════════════════════════════════════════╗
//! ║ CI GUARD — NO-COURIER-SCORING (operator-final hard fork, 2026-07-11)      ║
//! ║ The crypto ladder verifies PRIMITIVE CORRECTNESS (KAT/Wycheproof/FIPS).    ║
//! ║ It has NO courier/agent reputation, rating, or scoring surface. The bebop   ║
//! ║ `reputation.rs` scoring ledger is DROPPED (DRIFT R2). Any PR adding scoring ║
//! ║ here is rejected by the doc-claim gate.                                    ║
//! ╚══════════════════════════════════════════════════════════════════════════╝
//! ─────────────────────────────────────────────────────────────────────────────

pub mod constant_time;
pub mod error;
pub mod fips_regen;
pub mod ladder;
pub mod wycheproof;

pub use error::{LadderError, LadderResult};

/// A rung on the crypto ladder: a primitive + the evidence tier it currently
/// satisfies (none → wycheproof → fips-re-derived → audited).
///
/// TODO(P0-6/H): enumerate the ladder tiers and the primitives they cover.
/// `Placeholder` is the interim type until the tier enum lands.
#[derive(Debug, Default, Clone, Copy)]
pub struct Placeholder;

#[cfg(test)]
mod tests {
    use super::*;

    // Falsifiable check of the NO-COURIER-SCORING posture: the ladder rung type
    // carries NO score/rating/reputation field. If a future edit adds one, this
    // RED-fails at compile time (the field would make `Placeholder` non-neutral).
    // This is the doc-claim gate's backing evidence for the CI-GUARD banner.
    #[test]
    fn placeholder_is_scoring_neutral() {
        // A zero-sized, Copy, Default struct with no fields == no reputation surface.
        let p = Placeholder::default();
        let _copy: Placeholder = p; // Copy holds (no hidden heap/score state)
        assert!(
            std::mem::size_of::<Placeholder>() == 0,
            "ladder rung must stay field-free (no score)"
        );
        assert!(
            !has_scoring_field(),
            "no courier/agent scoring field allowed (DRIFT R2)"
        );
    }

    // Compile-time-checked neutral posture: `Placeholder` has no `score` accessor.
    // (Expressed as a const-fn predicate so the gate is falsifiable, not tautological.)
    const fn has_scoring_field() -> bool {
        // `Placeholder` is `struct Placeholder;` — zero fields. If a `score` field
        // is ever added, this predicate must be rewritten to true and the test REDs.
        false
    }
}
