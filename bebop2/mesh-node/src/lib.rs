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

pub mod boot;
pub mod breach;
pub mod dod;
pub mod hot_reload;
pub mod hub_policy;
pub mod kill_switch;
pub mod listener_reconcile;
pub mod node;

pub use boot::{boot, load_genesis, parse_genesis, BootError, BootedHub, Genesis, RevocationDecider};
pub use breach::{verify as verify_breach, BreachVerifyError};
pub use dod::{DodFault, DodGate};
pub use hot_reload::PolicyWatcher;
pub use hub_policy::{
    ApplyOutcome, BridgeSpec, HubPolicy, ListenerSpec, ModelEndpoint, PolicyError, PolicyStore,
    RateLimitConfig, RedLinePolicyData,
};
pub use kill_switch::{
    KillAnchors, KillOrder, KillReject, KillSequence, KillState, ReplayLedger, SnapshotConfirmer,
    SnapshotReceipt,
};
pub use listener_reconcile::{reconcile, ListenerAction};
pub use node::{MeshEventSink, MeshNode};
