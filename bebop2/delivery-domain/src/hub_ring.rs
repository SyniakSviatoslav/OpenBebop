//! P13 · §2 Hub-ring ownership overlay (AC-11).
//!
//! Orders are owned by a **hub** so the mesh has a single writer per order —
//! no two hubs race to mutate the same `OrderStatus`. The owner is chosen with
//! **Highest-Random-Weight (HRW)** over the set of hub public keys:
//!
//! ```text
//! owner(order_id) = argmax_hub hrw_weight(order_id, hub.pubkey)
//! replicas(order_id) = the next (R-1) highest-weight hubs
//! ```
//!
//! HRW is **rendezvous hashing**: every node computes the owner from the same
//! `(order_id, hub_set)` inputs and arrives at the identical answer with ZERO
//! coordination. Adding/removing a hub only steals/reassigns the orders whose
//! weight ranking changes — the rest are unaffected. There is **no single point
//! of failure**: if the owner hub is down, the next replica in the sorted list
//! transparently takes over, and the assignment is deterministic for every
//! observer (the blueprint's "No SPOF" requirement, §2).
//!
//! This reuses `bebop_proto_cap::matcher::hrw_weight` VERBATIM — the very same
//! function the courier-routing overlay uses — so we add **zero** new code for
//! the weighting math (operator DECART: "build to the contract, zero new code").

use bebop_proto_cap::matcher::hrw_weight;

/// A mesh hub, identified by its Ed25519 public key (32 bytes). The key IS the
/// stable identity the HRW weight is computed over; it is never a score.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Hub {
    /// Ed25519 public key — the hub's self-certifying identity.
    pub pubkey: [u8; 32],
}

impl Hub {
    pub fn new(pubkey: [u8; 32]) -> Self {
        Hub { pubkey }
    }
}

/// A deterministic ownership assignment for one order over a hub set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ownership {
    /// The owner hub (top HRW weight).
    pub owner: Hub,
    /// Replica hubs in descending weight order (the next `R-1` after the owner).
    pub replicas: Vec<Hub>,
}

/// Rank every hub by `hrw_weight(order_id, hub.pubkey)` (descending). Ties are
/// broken by pubkey bytes (lexicographic) so the ordering is total and stable
/// across nodes. Returns the hubs in owner-first order.
fn ranked(order_id: u64, hubs: &[Hub]) -> Vec<Hub> {
    let mut ranked: Vec<Hub> = hubs.to_vec();
    ranked.sort_by(|a, b| {
        hrw_weight(order_id, &b.pubkey)
            .cmp(&hrw_weight(order_id, &a.pubkey))
            .then_with(|| b.pubkey.cmp(&a.pubkey))
    });
    ranked
}

/// Compute the `(owner, replicas)` assignment for `order_id` over `hubs`.
///
/// `replicas` is bounded by `replica_count`, clamped to `hubs.len().saturating_sub(1)`
/// so we never claim a replica that does not exist. With `replica_count = 0` the
/// assignment has no replicas (single-owner, F50 solo-island falls into this
/// degenerate-but-valid case).
pub fn assign(order_id: u64, hubs: &[Hub], replica_count: usize) -> Ownership {
    let ranked = ranked(order_id, hubs);
    let owner = ranked[0];
    let max_replicas = hubs.len().saturating_sub(1);
    let take = replica_count.min(max_replicas);
    let replicas = ranked[1..=take].to_vec();
    Ownership { owner, replicas }
}

/// Convenience: just the owner hub.
pub fn owner_hub(order_id: u64, hubs: &[Hub]) -> Hub {
    assign(order_id, hubs, 0).owner
}

/// Is `hub` the computed owner of `order_id`?
pub fn is_owner(order_id: u64, hubs: &[Hub], hub: &Hub) -> bool {
    owner_hub(order_id, hubs) == *hub
}

/// Is `hub` within the replica set (owner excluded) of `order_id`?
pub fn is_replica(order_id: u64, hubs: &[Hub], replica_count: usize, hub: &Hub) -> bool {
    assign(order_id, hubs, replica_count)
        .replicas
        .contains(hub)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bebop2_core::sign::keygen;

    fn hubs(n: u8) -> Vec<Hub> {
        (0..n).map(|i| Hub::new(keygen(&[i; 32]).0)).collect()
    }

    // ── AC-11 GREEN: every node agrees on the owner with zero coordination.
    #[test]
    fn ac11_owner_is_rendezvous_deterministic() {
        let h = hubs(7);
        let a = owner_hub(42, &h);
        let b = owner_hub(42, &h);
        let c = owner_hub(42, &h);
        assert_eq!(a, b);
        assert_eq!(b, c);
    }

    // ── AC-11: owner is always within the hub set (no phantom owner).
    #[test]
    fn ac11_owner_belongs_to_set() {
        let h = hubs(5);
        let owner = owner_hub(99, &h);
        assert!(h.contains(&owner), "owner must be a real member hub");
    }

    // ── AC-11 GREEN: killing the owner -> a replica takes over (No SPOF).
    // The blueprint requires that no single hub is a point of failure: removing
    // the owner must promote a deterministic successor, and a *different* order
    // need not move at all (HRW locality — only affected orders reassign).
    #[test]
    fn ac11_no_spof_owner_removal_promotes_replica() {
        let h = hubs(5);
        let order = 1234u64;
        let owner_before = owner_hub(order, &h);

        // Drop the owner; the surviving set still has members.
        let survivors: Vec<Hub> = h.iter().filter(|x| **x != owner_before).copied().collect();
        assert_eq!(survivors.len(), 4);

        let owner_after = owner_hub(order, &survivors);
        // The new owner is one of the survivors (never the removed one).
        assert!(survivors.contains(&owner_after));
        assert_ne!(owner_after, owner_before);

        // And the assignment is still deterministic for the survivors.
        assert_eq!(owner_after, owner_hub(order, &survivors));
    }

    // ── AC-11 GREEN: replicas are distinct from the owner and from each other.
    #[test]
    fn ac11_replica_set_is_distinct() {
        let h = hubs(6);
        let o = assign(7, &h, 2);
        assert_ne!(o.owner, o.replicas[0]);
        assert_ne!(o.replicas[0], o.replicas[1]);
        assert_ne!(o.owner, o.replicas[1]);
        // All replicas are members.
        for r in &o.replicas {
            assert!(h.contains(r));
        }
    }

    // ── AC-11 GREEN: with R-1 replicas over N hubs, every order has exactly one
    // owner and N-1 distinct candidate writers (owner + replicas) — full cover.
    #[test]
    fn ac11_full_replica_cover() {
        let h = hubs(4);
        let o = assign(55, &h, 3); // R=4 -> 3 replicas
        assert_eq!(o.replicas.len(), 3);
        let mut all = vec![o.owner];
        all.extend(o.replicas.iter().copied());
        // 4 distinct hubs = the whole set.
        assert_eq!(all.len(), 4);
        for hub in &h {
            assert!(all.contains(hub));
        }
    }

    // ── AC-11 locality: removing a NON-owner hub must NOT change the owner of an
    // order that didn't rank the removed hub at the top (HRW stability). This is
    // what makes hub churn cheap — most orders keep their owner.
    #[test]
    fn ac11_locality_owner_stable_unless_removed() {
        let h = hubs(6);
        let order = 2024u64;
        let owner = owner_hub(order, &h);

        // Remove a hub that is NOT the owner.
        let victim = h.iter().find(|x| **x != owner).copied().unwrap();
        let survivors: Vec<Hub> = h.iter().filter(|x| **x != victim).copied().collect();

        // Owner unchanged (the removed hub was not it).
        assert_eq!(owner_hub(order, &survivors), owner);
    }
}
