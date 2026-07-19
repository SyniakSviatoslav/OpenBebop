//! Zenoh — deterministic mesh transport (local broker stand-in).
//!
//! Replaces the Research-slot "Zenoh mesh" as real, tested Rust. This is the
//! *offline* mesh: a process-local pub/sub broker that mirrors the `Portkey`
//! envelope interface, so the two are swappable behind the same call pattern.
//! A real Zenoh (`zenoh` crate) would implement the same `Mesh` trait over the
//! network; here we prove the routing/dispatch logic deterministically with no
//! network, no rng, no clock.
//!
//! This is the seam, not the wire protocol. Verified by in-process tests.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Re-use Portkey's envelope shape so transports are interchangeable.
pub use crate::portkey::Envelope;

/// A mesh transport: same contract as Portkey, different topology (mesh vs bus).
#[derive(Clone, Default)]
pub struct Mesh {
    inner: Arc<Mutex<MeshInner>>,
}

struct MeshInner {
    /// topic -> list of (node, handler)
    subs: HashMap<String, Vec<(String, usize)>>,
    /// `Arc<dyn Fn>` (not `Box`) so `publish` can CLONE the subscribed handles out
    /// under the lock and dispatch them AFTER the guard is dropped (snapshot-under-lock).
    handlers: HashMap<usize, Arc<dyn Fn(&Envelope) + Send + Sync>>,
    next_id: usize,
    /// per-node delivery log for deterministic assertions
    log: Vec<(String, String, String)>, // (node, topic, body)
}

impl Default for MeshInner {
    fn default() -> Self {
        MeshInner {
            subs: HashMap::new(),
            handlers: HashMap::new(),
            next_id: 1,
            log: Vec::new(),
        }
    }
}

impl Mesh {
    pub fn new() -> Self {
        Mesh::default()
    }

    /// Subscribe `node` to a topic. Returns a handle id.
    pub fn join(
        &self,
        node: &str,
        topic: &str,
        f: impl Fn(&Envelope) + Send + Sync + 'static,
    ) -> usize {
        let mut g = self.inner.lock().unwrap();
        let id = g.next_id;
        g.next_id += 1;
        g.handlers.insert(id, Arc::new(f));
        g.subs
            .entry(topic.to_string())
            .or_default()
            .push((node.to_string(), id));
        id
    }

    pub fn leave(&self, topic: &str, id: usize) {
        let mut g = self.inner.lock().unwrap();
        if let Some(v) = g.subs.get_mut(topic) {
            v.retain(|(_, x)| *x != id);
        }
        g.handlers.remove(&id);
    }

    /// Publish to a topic across the mesh. Every subscribed node receives a copy
    /// (that's the mesh fan-out). Returns the number of node-deliveries.
    ///
    /// Concurrency (G-C1 fix, `OPUS-PERF-BESTPRACTICES-PROPAGATION-2026-07-18.md`):
    /// the subscribed node-handlers are SNAPSHOTTED (`Arc` clones) under the lock and
    /// the delivery log is written under that same lock, THEN the guard is dropped and
    /// the handlers are invoked outside it. The old code held the single mesh `Mutex`
    /// across the whole per-node dispatch loop (and mutated `g.log` inside it), which
    /// serialized every publish and SELF-DEADLOCKED on any handler that re-entered the
    /// mesh (join/leave/publish re-locks the same non-reentrant `std::sync::Mutex`).
    /// Fan-out count and delivery-log order are preserved.
    pub fn publish(&self, env: &Envelope) -> usize {
        let dispatch: Vec<Arc<dyn Fn(&Envelope) + Send + Sync>> = {
            let mut g = self.inner.lock().unwrap();
            let targets: Vec<(String, usize)> = match g.subs.get(&env.topic) {
                Some(v) => v.clone(),
                None => return 0,
            };
            let mut out = Vec::with_capacity(targets.len());
            for (node, id) in &targets {
                if let Some(h) = g.handlers.get(id).cloned() {
                    g.log
                        .push((node.clone(), env.topic.clone(), env.body.clone()));
                    out.push(h);
                }
            }
            out
        }; // ← guard dropped here, BEFORE any handler runs
        let mut count = 0;
        for h in &dispatch {
            h(env);
            count += 1;
        }
        count
    }

    /// Total node-deliveries recorded.
    pub fn delivery_count(&self) -> usize {
        self.inner.lock().unwrap().log.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mesh_fanout_to_all_nodes() {
        // GREEN: two nodes on "telemetry" both receive the publication.
        let m = Mesh::new();
        let a = Arc::new(Mutex::new(0usize));
        let b = Arc::new(Mutex::new(0usize));
        let a2 = a.clone();
        let b2 = b.clone();
        m.join("nodeA", "telemetry", move |_| *a2.lock().unwrap() += 1);
        m.join("nodeB", "telemetry", move |_| *b2.lock().unwrap() += 1);
        let n = m.publish(&Envelope {
            topic: "telemetry".into(),
            from: "sensor".into(),
            to: "".into(),
            body: "tick".into(),
        });
        assert_eq!(n, 2);
        assert_eq!(*a.lock().unwrap(), 1);
        assert_eq!(*b.lock().unwrap(), 1);
    }

    #[test]
    fn leave_stops_node_receiving() {
        // RED: after a node leaves, it no longer receives mesh fan-out.
        let m = Mesh::new();
        let hits = Arc::new(Mutex::new(0usize));
        let h2 = hits.clone();
        let id = m.join("nodeC", "alerts", move |_| *h2.lock().unwrap() += 1);
        m.publish(&Envelope {
            topic: "alerts".into(),
            from: "x".into(),
            to: "".into(),
            body: "1".into(),
        });
        m.leave("alerts", id);
        m.publish(&Envelope {
            topic: "alerts".into(),
            from: "x".into(),
            to: "".into(),
            body: "2".into(),
        });
        assert_eq!(*hits.lock().unwrap(), 1, "node received after leaving mesh");
    }

    // ── G-C1 correctness: snapshot-under-lock / dispatch-outside-lock ──

    #[test]
    fn reentrant_handler_does_not_deadlock() {
        // A node handler that re-publishes into the mesh from inside its dispatch must
        // NOT deadlock. Old shape held the mesh Mutex across dispatch → re-lock hang.
        let m = Mesh::new();
        let downstream = Arc::new(Mutex::new(0usize));
        let d2 = downstream.clone();
        m.join("sink", "b", move |_| *d2.lock().unwrap() += 1);
        let m2 = m.clone();
        m.join("relay", "a", move |_| {
            m2.publish(&Envelope {
                topic: "b".into(),
                from: "relay".into(),
                to: "".into(),
                body: "cascade".into(),
            });
        });
        let n = m.publish(&Envelope {
            topic: "a".into(),
            from: "src".into(),
            to: "".into(),
            body: "trigger".into(),
        });
        assert_eq!(n, 1, "the 'a' relay fired");
        assert_eq!(
            *downstream.lock().unwrap(),
            1,
            "re-entrant mesh publish completed without deadlock"
        );
    }
}
