//! iroh transport — QUIC + DHT hole-punching node-to-node carrier.
//!
//! This REPLACES the legacy `crates/bebop/src/zenoh.rs` process-local pub/sub
//! stub. The zenoh stub proved routing/dispatch logic deterministically in
//! process; this line is where that same contract is realised over a real
//! swarmed, NAT-traversal transport.
//!
//! # Status: TODO (deferred)
//! `impl Transport for IrohTransport` using the `iroh` crate (quinn QUIC +
//! discovery DHT) is **not wired in this change**. The `iroh` crate is heavy and
//! intentionally NOT a dependency here (offline-air-gap build policy for the
//! scaffold; Tier-4 wiring gated on G11 GREEN). The module provides the SAME
//! `Transport` shape as [`crate::wss_transport`] so it is a drop-in carrier once
//! the dependency is added — the only carrier-specific parts are connect/accept
//! (ticket/peer-id exchange) and the stream segmentation framing (already shared
//! via [`crate::framing`]).
//!
//! CI GUARD: NO-COURIER-SCORING — transport neutrality: moves frames only. No
//! reputation, no scoring, no trust ranking.

#![allow(dead_code)]

use bebop_proto_cap::SignedFrame;

use crate::error::{WireError, WireResult};
use crate::Transport;

/// iroh endpoint descriptor (TODO: real iroh `NodeId` / ticket once wired).
#[derive(Debug, Clone)]
pub enum IrohEndpoint {
    /// A node ticket / URL to dial as a client.
    Ticket(String),
    /// A bind address for an iroh node accepting connections.
    Bind(String),
}

/// Placeholder iroh transport. Carries no stream yet; `connect`/`accept`/`send`/
/// `recv` are intentionally unimplemented (return `NotConnected`). The type
/// exists so the `Transport` contract is satisfied structurally and the module
/// compiles offline without the `iroh` dependency.
pub struct IrohTransport {
    _endpoint: IrohEndpoint,
}

impl IrohTransport {
    /// Construct a placeholder (no connection). Real wiring in Tier-4.
    pub fn new(endpoint: IrohEndpoint) -> Self {
        IrohTransport {
            _endpoint: endpoint,
        }
    }
}

impl Transport for IrohTransport {
    type Endpoint = IrohEndpoint;

    async fn connect(_endpoint: &Self::Endpoint) -> WireResult<Self> {
        // TODO(iroh): dial the node ticket via `iroh::Endpoint::connect`, then
        // open a QUIC stream and run the signed-handshake. Until the `iroh` dep is
        // added (Tier-4), this is unimplemented.
        Err(WireError::NotConnected)
    }

    async fn accept(_endpoint: &Self::Endpoint) -> WireResult<Self> {
        // TODO(iroh): bind an `iroh::Endpoint`, accept an inbound connection, open
        // the stream. Unimplemented until the `iroh` dep is added (Tier-4).
        Err(WireError::NotConnected)
    }

    async fn send(&mut self, _frame: SignedFrame) -> WireResult<()> {
        // TODO(iroh): frame `_frame` via `crate::framing`, write to the QUIC stream.
        Err(WireError::NotConnected)
    }

    async fn recv(&mut self) -> WireResult<SignedFrame> {
        // TODO(iroh): read a length-prefixed frame from the QUIC stream, decode via
        // `crate::framing`, then verify via the hybrid gate. Unimplemented for now.
        Err(WireError::NotConnected)
    }
}
