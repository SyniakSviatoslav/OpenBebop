//! Handshake — mutual transport setup (key confirmation, routing bootstrap).
//!
//! The handshake authenticates endpoints via the signed capability in each frame
//! (see `bebop-proto-cap`), never via a bearer token or accumulated score. The
//! WSS carrier's HTTP Upgrade is handled by `tokio-tungstenite`; iroh's ticket /
//! peer-id exchange is a TODO in `iroh_transport`.
//!
//! This module currently exposes the neutral bootstrap types shared by carriers.
//! No scoring surface.
//!
//! CI GUARD: NO-COURIER-SCORING — handshake authenticates endpoints via signed
//! capability, never via accumulated score.

use serde::{Deserialize, Serialize};

/// A bootstrap greeting exchanged (inside the first signed envelope) at connect
/// time. Carries endpoint identity + protocol version only — never a rating.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Handshake {
    /// Protocol envelope version the peer speaks.
    pub version: u8,
    /// Opaque endpoint identity (e.g. a node id / public key). Not a score.
    pub peer_id: Vec<u8>,
}

impl Handshake {
    pub fn new(version: u8, peer_id: Vec<u8>) -> Self {
        Handshake { version, peer_id }
    }
}
