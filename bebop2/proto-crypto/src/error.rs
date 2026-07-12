//! Error types for the crypto ladder line.
//!
//! `LadderError` describes primitive-verification faults (vector mismatch,
//! missing evidence tier, constant-time assertion failure). `thiserror`-free
//! (dep-free) so the line cannot affect `dowiz`.
//!
//! CI GUARD: NO-COURIER-SCORING — errors describe primitive-verification faults.

/// Primitive-verification fault on the crypto ladder. No scoring/reputation variant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LadderError {
    /// A Wycheproof/FIPS vector did not match the `bebop2-core` primitive output.
    VectorMismatch { primitive: &'static str, case: u32 },
    /// A primitive has no evidence tier registered yet (ladder not populated).
    MissingEvidenceTier(&'static str),
    /// A constant-time assertion on a secret-dependent op failed.
    ConstantTimeViolation(&'static str),
}

/// Result alias used by ladder checks. Mirrors sibling W/A lines.
pub type LadderResult<T> = core::result::Result<T, LadderError>;

#[derive(Debug, Default, Clone, Copy)]
pub struct Placeholder;
