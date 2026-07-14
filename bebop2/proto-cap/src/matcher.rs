//! Courier matcher — coordination-free assignment via Rendezvous / HRW hashing.
//!
//! `assign(order, candidates) -> Vec<CourierKey>` returns the N candidate
//! couriers ranked by a **Highest-Random-Weight (HRW / rendezvous) hash** of
//! `(order_id, courier_pubkey)`. Every node computes the SAME ordering with NO
//! coordination, because the hash is a pure function of the inputs.
//!
//! # Why HRW (not scoring)
//! HRW gives a deterministic, stable, coordination-free priority order. It is a
//! hash, NOT a score: we never rate, rank-by-merit, trust, or reputation a
//! courier. Two nodes see identical input → identical assignment → no split
//! brain, no central coordinator. The first candidate is the "primary"; the rest
//! are fallbacks in deterministic order.
//!
//! # Structural NO-COURIER-SCORING
//! `CourierKey` is a 32-byte Ed25519 public key. The `Courier` struct carries
//! nothing else — no score / rating / trust / reputation / rank field. The CI
//! gate `scripts/ci-no-courier-scoring.sh` hard-locks this.

use crate::event_dict::CourierKey;

/// An order to be assigned. Plain data: id + source + destination.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Order {
    pub id: u64,
    pub src: String,
    pub dst: String,
}

/// A candidate courier. **NO scoring fields** — only its public key.
/// (This is the structural NO-COURIER-SCORING guarantee: the type literally
/// cannot carry a score.)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Courier {
    pub pubkey: CourierKey,
}

/// A pure FNV-1a 64-bit hash over the byte concatenation of the order id and the
/// courier key. Deterministic, dependency-free, no crypto needed for ordering.
/// Used as the HRW weight: higher hash → higher priority for this order.
pub fn hrw_weight(order_id: u64, courier: &CourierKey) -> u64 {
    let mut buf = [0u8; 8 + 32];
    buf[0..8].copy_from_slice(&order_id.to_le_bytes());
    buf[8..40].copy_from_slice(courier);
    fn fnv1a(data: &[u8]) -> u64 {
        let mut h: u64 = 0xcbf29ce484222325;
        for &b in data {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        h
    }
    fnv1a(&buf)
}

/// Assign N candidate couriers to an order, returning them in deterministic
/// HRW priority order (highest weight first). The top entry is the primary; the
/// remainder are deterministic fallbacks. `max` bounds the returned list length
/// (pass `candidates.len()` to keep all).
///
/// Coordination-free: identical `(order_id, candidate set)` on any node yields
/// the identical ordering. No network, no consensus.
pub fn assign(order: &Order, candidates: &[Courier], max: usize) -> Vec<CourierKey> {
    let mut ranked: Vec<(u64, CourierKey)> = candidates
        .iter()
        .map(|c| (hrw_weight(order.id, &c.pubkey), c.pubkey))
        .collect();
    // Deterministic tie-break: by weight DESC, then by pubkey ASC (so two equal
    // hashes still order identically on every node).
    ranked.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    ranked.truncate(max.min(ranked.len()));
    ranked.into_iter().map(|(_, k)| k).collect()
}

/// A refused order (e.g. all candidates dropped the claim) is requeued: the same
/// `assign` over the same candidate set returns the same ordering, so it is
/// **never dropped** — re-submission simply re-runs the deterministic HRW. This
/// helper returns the primary candidate for a requeued order (demonstrating the
/// never-drop invariant); callers loop over `assign(...)` to try fallbacks.
pub fn primary_for(order: &Order, candidates: &[Courier]) -> Option<CourierKey> {
    assign(order, candidates, candidates.len())
        .into_iter()
        .next()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn courier(byte: u8) -> Courier {
        Courier { pubkey: [byte; 32] }
    }

    // ── R-MESH05a: two nodes compute IDENTICAL assignment (fingerprint test).
    #[test]
    fn r_mesh05_two_nodes_identical_assignment() {
        let order = Order {
            id: 4242,
            src: "R".into(),
            dst: "C".into(),
        };
        let cands = vec![courier(1), courier(2), courier(3), courier(4), courier(5)];
        // Node A and Node B compute independently.
        let a = assign(&order, &cands, cands.len());
        let b = assign(&order, &cands, cands.len());
        assert_eq!(a, b, "HRW assignment must be identical across nodes");
        assert_eq!(a.len(), 5);
        // The ordering is stable & deterministic (not a score: just a hash order).
        assert_eq!(a, assign(&order, &cands, cands.len()));
    }

    // ── R-MESH05b: assignment is order-dependent but coordination-free.
    #[test]
    fn r_mesh05_different_order_different_but_deterministic() {
        let cands = vec![courier(10), courier(20), courier(30)];
        let o1 = Order {
            id: 1,
            src: "x".into(),
            dst: "y".into(),
        };
        let o2 = Order {
            id: 2,
            src: "x".into(),
            dst: "y".into(),
        };
        let a1 = assign(&o1, &cands, cands.len());
        let a2 = assign(&o2, &cands, cands.len());
        // Both deterministic.
        assert_eq!(a1, assign(&o1, &cands, cands.len()));
        assert_eq!(a2, assign(&o2, &cands, cands.len()));
        // They may differ; what matters is determinism (no coordination).
        assert!(!a1.is_empty());
        assert!(!a2.is_empty());
    }

    // ── R-MESH05c: a refused order is requeued and NEVER dropped.
    #[test]
    fn r_mesh05_refused_order_requeued_never_dropped() {
        let order = Order {
            id: 777,
            src: "s".into(),
            dst: "d".into(),
        };
        let cands = vec![courier(1), courier(2), courier(3)];
        // Simulate a refused order: re-run assignment repeatedly; the primary is
        // always present and the candidate set is intact (never dropped).
        for _ in 0..5 {
            let assigned = assign(&order, &cands, cands.len());
            assert_eq!(
                assigned.len(),
                cands.len(),
                "requeue must not drop candidates"
            );
            assert_eq!(primary_for(&order, &cands), assigned.first().copied());
        }
    }

    // ── GREEN: max-bounded assignment returns at most `max` couriers.
    #[test]
    fn green_mesh05_max_bounds_assignment() {
        let order = Order {
            id: 9,
            src: "a".into(),
            dst: "b".into(),
        };
        let cands = vec![courier(1), courier(2), courier(3), courier(4)];
        assert_eq!(assign(&order, &cands, 2).len(), 2);
        // primary is stable across max values (it's the top of the full order).
        let full = assign(&order, &cands, cands.len());
        let top2 = assign(&order, &cands, 2);
        assert_eq!(top2[0], full[0]);
    }

    // ── GREEN: weight is a pure function of inputs (no score/state).
    #[test]
    fn green_mesh05_hrw_weight_pure() {
        let k = [42u8; 32];
        assert_eq!(hrw_weight(123, &k), hrw_weight(123, &k));
        assert_ne!(hrw_weight(123, &k), hrw_weight(124, &k));
    }
}
