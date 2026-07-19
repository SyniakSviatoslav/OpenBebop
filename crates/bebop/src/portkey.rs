//! Portkey — deterministic local transport / gateway abstraction.
//!
//! Replaces the TS-retired `Portkey gateway` behavior as real, tested Rust.
//! This is the *offline* gateway: an in-process pub/sub + request/reply router
//! keyed by topic, with deterministic routing (no network, no rng, no clock).
//! The wire shape is JSON over a string envelope so the same API can later sit
//! on top of a real mesh (e.g. Zenoh) without changing call sites.
//!
//! Design note: this is NOT the network stack. It is the *abstraction* — the
//! seam where a real mesh transport would plug in. Verified by in-process tests.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// A routed message envelope.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Envelope {
    pub topic: String,
    pub from: String,
    pub to: String, // "" = broadcast
    pub body: String,
}

/// A subscriber callback handle id.
pub type SubId = usize;

/// Portkey: in-process message bus. `Arc<Mutex<..>>` so it can be shared across
/// "peers" in a single process (the offline stand-in for a mesh node).
#[derive(Clone, Default)]
pub struct Portkey {
    inner: Arc<Mutex<Inner>>,
}

struct Inner {
    subs: HashMap<String, Vec<SubId>>,
    /// `Arc<dyn Fn>` (not `Box`) so `publish` can CLONE the subscribed handles out
    /// under the lock and invoke them AFTER the guard is dropped — the snapshot-under-
    /// lock / dispatch-outside-lock discipline. A `Box<dyn Fn>` can't be cloned, which
    /// is why the old code held the bus lock across the whole dispatch loop.
    handlers: HashMap<SubId, Arc<dyn Fn(&Envelope) + Send + Sync>>,
    next_id: SubId,
    /// delivery log (topic, body) — for deterministic assertions in tests.
    log: Vec<(String, String)>,
}

impl Default for Inner {
    fn default() -> Self {
        Inner {
            subs: HashMap::new(),
            handlers: HashMap::new(),
            next_id: 1,
            log: Vec::new(),
        }
    }
}

impl Portkey {
    pub fn new() -> Self {
        Portkey::default()
    }

    /// Subscribe to a topic. Returns a handle id (used to unsubscribe).
    pub fn subscribe<F>(&self, topic: &str, f: F) -> SubId
    where
        F: Fn(&Envelope) + Send + Sync + 'static,
    {
        let mut g = self.inner.lock().unwrap();
        let id = g.next_id;
        g.next_id += 1;
        g.handlers.insert(id, Arc::new(f));
        g.subs.entry(topic.to_string()).or_default().push(id);
        id
    }

    pub fn unsubscribe(&self, topic: &str, id: SubId) {
        let mut g = self.inner.lock().unwrap();
        if let Some(v) = g.subs.get_mut(topic) {
            v.retain(|x| *x != id);
        }
        g.handlers.remove(&id);
    }

    /// Publish to a topic. Delivers to every subscriber (and to `to`-matched
    /// subscribers). Returns the count of handlers invoked.
    ///
    /// Concurrency (G-C1 fix, `OPUS-PERF-BESTPRACTICES-PROPAGATION-2026-07-18.md`):
    /// the subscribed handlers are SNAPSHOTTED (cheap `Arc` clones) under the lock, the
    /// guard is DROPPED, and only THEN are they invoked — snapshot-under-lock /
    /// dispatch-outside-lock. The old code held the single bus `Mutex` across the whole
    /// dispatch loop, which (a) serialized every publish behind the slowest handler and
    /// (b) SELF-DEADLOCKED the instant a handler re-entered the bus (subscribe/publish/
    /// unsubscribe re-locks the same non-reentrant `std::sync::Mutex`). Delivery order
    /// is preserved (snapshot is in `subs` order); the delivery log is written under the
    /// lock, before dispatch, exactly as before.
    pub fn publish(&self, env: &Envelope) -> usize {
        let handlers: Vec<Arc<dyn Fn(&Envelope) + Send + Sync>> = {
            let mut g = self.inner.lock().unwrap();
            g.log.push((env.topic.clone(), env.body.clone()));
            match g.subs.get(&env.topic) {
                Some(ids) => ids
                    .iter()
                    .filter_map(|id| g.handlers.get(id).cloned())
                    .collect(),
                None => return 0,
            }
        }; // ← guard dropped here, BEFORE any handler runs
        let mut count = 0;
        for h in &handlers {
            h(env);
            count += 1;
        }
        count
    }

    /// Request/reply over the same bus: publish on `topic`, but addressed `to`
    /// a specific peer. Convenience wrapper; routing is still topic-based.
    pub fn send(&self, env: &Envelope) -> usize {
        self.publish(env)
    }

    /// Number of deliveries recorded (for deterministic test assertions).
    pub fn delivery_count(&self) -> usize {
        self.inner.lock().unwrap().log.len()
    }

    /// All delivered (topic, body) pairs (for assertions).
    pub fn deliveries(&self) -> Vec<(String, String)> {
        self.inner.lock().unwrap().log.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn publish_reaches_subscriber() {
        // GREEN: a subscriber on "helm" receives the published envelope.
        let bus = Portkey::new();
        let got = Arc::new(Mutex::new(String::new()));
        let g2 = got.clone();
        bus.subscribe("helm", move |e| {
            *g2.lock().unwrap() = e.body.clone();
        });
        let n = bus.publish(&Envelope {
            topic: "helm".into(),
            from: "copilot".into(),
            to: "".into(),
            body: "turn to port".into(),
        });
        assert_eq!(n, 1);
        assert_eq!(*got.lock().unwrap(), "turn to port");
    }

    #[test]
    fn no_sub_no_delivery() {
        // RED: publishing on a topic with no subscriber delivers 0.
        let bus = Portkey::new();
        let n = bus.publish(&Envelope {
            topic: "void".into(),
            from: "x".into(),
            to: "".into(),
            body: "silence".into(),
        });
        assert_eq!(n, 0);
        assert_eq!(bus.delivery_count(), 1); // the envelope is still logged
    }

    #[test]
    fn unsubscribe_stops_delivery() {
        // RED: after unsubscribe the handler must not fire.
        let bus = Portkey::new();
        let hits = Arc::new(Mutex::new(0usize));
        let h2 = hits.clone();
        let id = bus.subscribe("engines", move |_| {
            *h2.lock().unwrap() += 1;
        });
        bus.publish(&Envelope {
            topic: "engines".into(),
            from: "a".into(),
            to: "".into(),
            body: "burn".into(),
        });
        bus.unsubscribe("engines", id);
        bus.publish(&Envelope {
            topic: "engines".into(),
            from: "a".into(),
            to: "".into(),
            body: "burn again".into(),
        });
        assert_eq!(*hits.lock().unwrap(), 1, "handler fired after unsubscribe");
    }

    // ── G-C1 correctness: snapshot-under-lock / dispatch-outside-lock ──

    #[test]
    fn publish_preserves_order_and_loses_no_dispatch() {
        // Three subscribers on one topic must ALL fire, in subscription order.
        // Proves the snapshot (Arc clones under the lock) preserves fan-out + order.
        let bus = Portkey::new();
        let order = Arc::new(Mutex::new(Vec::<u8>::new()));
        for tag in [1u8, 2, 3] {
            let o = order.clone();
            bus.subscribe("nav", move |_| o.lock().unwrap().push(tag));
        }
        let n = bus.publish(&Envelope {
            topic: "nav".into(),
            from: "helm".into(),
            to: "".into(),
            body: "mark".into(),
        });
        assert_eq!(n, 3, "no dispatch lost — all three subscribers fired");
        assert_eq!(
            *order.lock().unwrap(),
            vec![1, 2, 3],
            "delivery order preserved (subscription order)"
        );
    }

    #[test]
    fn reentrant_handler_does_not_deadlock() {
        // A handler that RE-PUBLISHES to the bus from inside its own dispatch is the
        // natural "react to a message by emitting another" pattern. With the old
        // lock-across-dispatch shape this re-locked the same non-reentrant Mutex and
        // deadlocked (the test would HANG). With snapshot-under-lock it completes.
        let bus = Portkey::new();
        let downstream = Arc::new(Mutex::new(0usize));
        let d2 = downstream.clone();
        bus.subscribe("b", move |_| *d2.lock().unwrap() += 1);
        let bus2 = bus.clone();
        bus.subscribe("a", move |_| {
            // re-enter the bus from within a handler — must NOT deadlock.
            bus2.publish(&Envelope {
                topic: "b".into(),
                from: "a-handler".into(),
                to: "".into(),
                body: "cascade".into(),
            });
        });
        let n = bus.publish(&Envelope {
            topic: "a".into(),
            from: "src".into(),
            to: "".into(),
            body: "trigger".into(),
        });
        assert_eq!(n, 1, "the 'a' handler fired");
        assert_eq!(
            *downstream.lock().unwrap(),
            1,
            "re-entrant publish to 'b' completed without deadlock"
        );
    }
}
