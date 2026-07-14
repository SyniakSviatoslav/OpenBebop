//! Error types for the wire transport line.
//!
//! `WireError` describes transport-level faults only: frame too large, decode
//! failure, carrier disconnect, handshake rejection. It never carries or derives
//! a courier/agent score.
//!
//! CI GUARD: NO-COURIER-SCORING — errors describe transport faults, never scores.

use core::fmt;

/// Transport / framing error. Neutral plumbing: a frame is moved or not; there is
/// no reputation surface.
#[derive(Debug, Clone)]
pub enum WireError {
    /// A frame exceeded the maximum allowed size.
    FrameTooLarge(usize),
    /// A frame payload exceeded the transport policy limit (MESH-10 DoS gate).
    PayloadTooLarge(usize),
    /// The transport requires a TLS/QUIC channel binding but the frame has none
    /// (MESH-10: plaintext rejected when TLS required).
    InsecureTransport(&'static str),
    /// Envelope protocol version is unsupported / tampered on the wire.
    VersionMismatch(u8),
    /// Failed to (de)serialize the envelope / inner frame.
    Encode(String),
    /// The WebSocket carrier errored or disconnected.
    Carrier(String),
    /// Handshake (upgrade / accept) was rejected.
    HandshakeRejected(String),
    /// A received frame failed capability verification (bad signature / replay /
    /// scope). The underlying auth error is preserved verbatim.
    CapabilityVerify(String),
    /// Peer sent a clean WebSocket Close handshake — EOF, not a fault.
    Closed,
    /// No carrier connection is established (call connect/accept first).
    NotConnected,
    /// I/O error on the underlying stream.
    Io(String),
}

impl fmt::Display for WireError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WireError::FrameTooLarge(n) => write!(f, "frame too large ({n} bytes)"),
            WireError::PayloadTooLarge(n) => write!(f, "payload too large ({n} bytes)"),
            WireError::InsecureTransport(s) => write!(f, "insecure transport: {s}"),
            WireError::VersionMismatch(v) => write!(f, "unsupported envelope version {v}"),
            WireError::Encode(s) => write!(f, "frame encode/decode error: {s}"),
            WireError::Carrier(s) => write!(f, "websocket carrier error: {s}"),
            WireError::HandshakeRejected(s) => write!(f, "handshake rejected: {s}"),
            WireError::CapabilityVerify(s) => write!(f, "capability verification failed: {s}"),
            WireError::Closed => write!(f, "peer closed the connection cleanly"),
            WireError::NotConnected => write!(f, "transport not connected"),
            WireError::Io(s) => write!(f, "io error: {s}"),
        }
    }
}

impl core::error::Error for WireError {}

impl From<serde_json::Error> for WireError {
    fn from(e: serde_json::Error) -> Self {
        WireError::Encode(e.to_string())
    }
}

impl From<bebop_proto_cap::CapError> for WireError {
    fn from(e: bebop_proto_cap::CapError) -> Self {
        WireError::CapabilityVerify(e.to_string())
    }
}

impl From<tokio::io::Error> for WireError {
    fn from(e: tokio::io::Error) -> Self {
        WireError::Io(e.to_string())
    }
}

/// Convenience `Result` alias for the wire line.
pub type WireResult<T> = Result<T, WireError>;
