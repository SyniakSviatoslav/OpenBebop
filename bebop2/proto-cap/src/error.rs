//! Error types for the authorization line.
//!
//! `CapError` describes authentication faults only — bad signature, expired
//! nonce, scope violation, missing hybrid proof. It NEVER encodes or derives a
//! courier/agent score.
//!
//! CI GUARD: NO-COURIER-SCORING — errors describe auth faults, never scores.

use core::fmt;

/// Authentication / capability error. Neutral plumbing: a frame is accepted or
/// rejected on its signature + nonce + scope; there is no reputation surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CapError {
    /// The classical (Ed25519) signature failed to verify.
    ClassicalVerifyFailed,
    /// The post-quantum (ML-DSA-65) signature failed to verify.
    PqVerifyFailed,
    /// The hybrid gate requires BOTH a classical and a PQ signature, but one or
    /// both are missing (or the PQ leg is still a TODO on this build).
    HybridIncomplete,
    /// The capability nonce has already been seen (replay) or is invalid.
    NonceRejected,
    /// The capability is past its expiry.
    Expired,
    /// The capability references a resource/action outside the scope enum.
    ScopeViolation,
    /// Cannot (de)serialize the capability for canonical signing.
    Encode,
    /// The signature or key buffer had the wrong length.
    BadLength,
}

impl fmt::Display for CapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            CapError::ClassicalVerifyFailed => "classical (Ed25519) signature verification failed",
            CapError::PqVerifyFailed => "post-quantum (ML-DSA-65) signature verification failed",
            CapError::HybridIncomplete => {
                "hybrid gate requires BOTH classical + PQ signatures (one missing or PQ leg TODO)"
            }
            CapError::NonceRejected => "capability nonce rejected (replay or invalid)",
            CapError::Expired => "capability expired",
            CapError::ScopeViolation => "capability references a resource/action outside scope",
            CapError::Encode => "capability (de)serialization failed",
            CapError::BadLength => "signature or key buffer had the wrong length",
        };
        f.write_str(s)
    }
}

impl core::error::Error for CapError {}

/// Convenience `Result` alias for the authorization line.
pub type CapResult<T> = Result<T, CapError>;
