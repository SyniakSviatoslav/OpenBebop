//! bebop-mesh-node — the mesh node runtime (port layer).
//!
//! Ties the already-built `proto-wire` carriers (WSS, real rustls/ring) and
//! `proto-cap` `KernelFacade`/`EventSink` into a RUNNING bidirectional
//! async mesh, with a per-event Definition-of-Done gate on every inbound
//! event. See `node.rs` / `dod.rs` module docs for the design law.
//!
//! CI GUARD: NO-COURIER-SCORING — this node moves signed frames and gates
//! events on DOD; it never derives, consults, or encodes a courier/agent
//! score.

pub mod breach;
pub mod dod;
pub mod node;

pub use breach::{verify as verify_breach, BreachVerifyError};
pub use dod::{DodFault, DodGate};
pub use node::{MeshEventSink, MeshNode};
