//! Scope — resource/action namespace the capability system understands.
//!
//! A closed enum so the gate is exhaustively checkable. Scope describes
//! OBJECTS and VERBS (route, ledger entry, delivery intent, …), never ratings.
//!
//! CI GUARD: NO-COURIER-SCORING — scope describes objects/verbs, not trust.

use serde::{Deserialize, Serialize};

/// A protocol resource a capability may target. Closed set so the gate is total.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Resource {
    /// A transport route / channel.
    Route,
    /// A ledger entry (append / read).
    Ledger,
    /// A delivery intent (drop / query).
    DeliveryIntent,
    /// A generic mesh heartbeat / presence message.
    Presence,
}

/// An action permitted on a [`Resource`]. Closed set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    /// Authorize a send on the resource.
    Send,
    /// Authorize a read/query of the resource.
    Read,
    /// Authorize an append/write to the resource.
    Append,
}

/// `(resource, action)` pair a capability authorizes. No score, no subject rating.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Scope {
    pub resource: Resource,
    pub action: Action,
}

impl Scope {
    /// Construct a scope. Placeholder until Tier-4 wiring enumerates the full
    /// resource/action matrix.
    pub fn new(resource: Resource, action: Action) -> Self {
        Scope { resource, action }
    }
}
