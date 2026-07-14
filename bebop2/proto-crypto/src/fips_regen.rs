//! FIPS re-derivation — independently re-derive each primitive from its FIPS
//! specification as a second, independent oracle against Wycheproof.
//!
//! TODO(P0-6/H): reference re-implementations (or verified derivations) of
//! ML-KEM-768 (FIPS 203), ML-DSA-65 (FIPS 204), SHA/SHAKE (FIPS 180-4/202),
//! etc. used only as a cross-check oracle, never in the hot path.
//!
//! CI GUARD: NO-COURIER-SCORING — oracle verifies spec conformance, not scores.

#[derive(Debug, Default, Clone, Copy)]
pub struct Placeholder;
