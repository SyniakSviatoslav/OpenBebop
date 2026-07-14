//! Ladder — the verification strength ladder over `bebop2-core` primitives.
//!
//! TODO(P0-6/H): define tiers (none → wycheproof-checked → fips-re-derived →
//! independently-audited) and the registry mapping each primitive to its tier.
//! This is the "H" (crypto ladder) spine — NOT a scoring system.
//!
//! CI GUARD: NO-COURIER-SCORING — ladder grades primitives, never movers.

#[derive(Debug, Default, Clone, Copy)]
pub struct Placeholder;
