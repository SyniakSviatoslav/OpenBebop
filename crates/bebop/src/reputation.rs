//! REPUTATION — the node-trust primitive (the real blocker from the audit).
//!
//! ⚠️ P76 / D-2 FLAG (OPEN, non-blocking): this module implements
//! courier/node *scoring* via a trust ledger. That is FORBIDDEN by the
//! NO-COURIER-SCORING red line in `dowiz_kernel::event_log` — trust must come
//! from signed capability, never from a reputation score. The canonical stance is
//! that this file is removed (or re-born as append-only score events feeding the
//! event-sourced ledger) per the §4/D-2 operator ruling. It is currently
//! unreferenced anywhere in the crate (dead surface), so leaving it as-is does
//! not affect any runtime path. Do NOT expand scope here in P76 — this is a
//! marker only.
//!
//! The audit asked: is the blocker "no trust between nodes" or "no standard
//! interface"? The interface already exists (matcher JSON contract +
//! `MatcherClient`). The missing piece is TRUST: a network of strangers with a
//! perfect interface but no reputation = "whoever feeds the most convincing
//! (fake) graph wins". So we add a deterministic reputation ledger:
//!
//!   • a valid POD proof (crate::pod) RAISES a courier's trust;
//!   • a consensus suspension (crate::guard::KillSwitch) LOWERS it to floor;
//!   • trust feeds the cost surface — high-trust couriers are preferred, low/
//!     unknown trust costs more (risk premium), suspended = unreachable.
//!
//! Deterministic, additive, fully auditable (no RNG). This is the "poison"/moat
//! the investor deck needs (audit 29160, defensibility): the network's trust
//! graph is the asset competitors cannot copy. RED+GREEN falsifiable below.

/// A courier's trust record.
#[derive(Clone, Debug, Default)]
pub struct TrustRecord {
    /// Successful, verified deliveries (valid POD proofs).
    pub deliveries: u64,
    /// Number of times this node was suspended by consensus.
    pub suspensions: u64,
}

/// The reputation ledger: node id → trust record. Pure, deterministic.
#[derive(Clone, Debug, Default)]
pub struct ReputationLedger {
    records: std::collections::HashMap<String, TrustRecord>,
}

impl ReputationLedger {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a verified delivery (a valid POD proof landed). Raises trust.
    pub fn record_delivery(&mut self, node: &str) {
        self.records.entry(node.to_string()).or_default().deliveries += 1;
    }

    /// Record a consensus suspension (KillSwitch fired). Slashes trust.
    pub fn record_suspension(&mut self, node: &str) {
        let r = self.records.entry(node.to_string()).or_default();
        r.suspensions += 1;
    }

    /// Precedence / recency decay (arXiv 2104.03902 §4.2 "The Autodidactic
    /// Universe"): old deliveries fade so the trust score tracks RECENT
    /// behaviour, not lifetime totals. Multiplies each delivery count by
    /// `alpha` ∈ [0,1]; `alpha = 1` is a no-op. SUSPENSIONS are STICKY — they do
    /// not decay (a consensus suspension is a permanent safety mark, not a
    /// fading statistic). Deterministic, RNG-free.
    pub fn decay(&mut self, alpha: f64) {
        assert!((0.0..=1.0).contains(&alpha), "alpha must be in [0,1]");
        for r in self.records.values_mut() {
            r.deliveries = (r.deliveries as f64 * alpha).round() as u64;
            // suspensions intentionally NOT decayed
        }
    }

    /// Trust score in [0,1]: deliveries/(deliveries+suspensions) softened, with a
    /// floor of 0 once suspended. Unknown node (no record) = neutral 0.5 (the
    /// "prove yourself" baseline — not trusted, not distrusted). Each verified
    /// delivery lifts strictly above neutral: 0.5 + 0.5·(d/(d+1)), so 1 delivery
    /// ⇒ 0.75, 2 ⇒ 0.833 — a fresh courier is NEVER indistinguishable from an
    /// unknown one (that would let a sybil pretend to have a history).
    pub fn score(&self, node: &str) -> f64 {
        match self.records.get(node) {
            None => 0.5,
            Some(r) => {
                if r.suspensions > 0 {
                    return 0.0; // suspended ⇒ untrusted floor
                }
                let d = r.deliveries as f64;
                // logistic-ish: saturates toward 1 with deliveries, strictly above 0.5
                0.5 + 0.5 * (d / (d + 1.0))
            }
        }
    }

    /// Risk premium for the cost surface: low trust ⇒ higher cost (avoided by
    /// the router). Suspended ⇒ +inf (unreachable). Returns a multiplier ≥ 1.
    pub fn risk_premium(&self, node: &str) -> f64 {
        let s = self.score(node);
        if s <= 0.0 {
            f64::INFINITY // suspended ⇒ unreachable
        } else {
            // 1 / trust: trust 1 ⇒ ×1, trust 0.5 ⇒ ×2, trust→0 ⇒ large
            1.0 / s.max(1e-3)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_node_is_neutral_not_distrusted() {
        // GREEN: a node with no history is "prove yourself" (0.5), not blocked.
        let l = ReputationLedger::new();
        assert!((l.score("ghost") - 0.5).abs() < 1e-12);
        assert!(l.risk_premium("ghost").is_finite());
    }

    #[test]
    fn deliveries_raise_trust() {
        // GREEN: each verified delivery lifts the score monotonically toward 1.
        let mut l = ReputationLedger::new();
        let s0 = l.score("c1");
        l.record_delivery("c1");
        let s1 = l.score("c1");
        l.record_delivery("c1");
        let s2 = l.score("c1");
        assert!(s0 < s1 && s1 < s2, "trust rises with deliveries");
        assert!(s2 > 0.6, "two deliveries ⇒ clearly trusted");
    }

    #[test]
    fn suspension_slashes_trust_to_floor() {
        // RED+GREEN: a consensus suspension drops trust to 0 and makes the node
        // unreachable (∞ risk premium) in the cost surface.
        let mut l = ReputationLedger::new();
        l.record_delivery("c1");
        l.record_delivery("c1");
        l.record_suspension("c1");
        assert_eq!(l.score("c1"), 0.0, "suspended ⇒ floor 0");
        assert!(
            l.risk_premium("c1").is_infinite(),
            "suspended ⇒ unreachable"
        );
    }

    #[test]
    fn precedence_decay_tracks_recent_not_lifetime() {
        // GREEN: alpha<1 fades old deliveries ⇒ score drops toward neutral.
        // RED+GREEN: a suspension does NOT decay away (sticky safety mark).
        let mut l = ReputationLedger::new();
        for _ in 0..10 {
            l.record_delivery("c1");
        }
        let high = l.score("c1");
        l.decay(0.5); // half the deliveries fade
        let faded = l.score("c1");
        assert!(faded < high, "decay lowers recent trust");
        // suspension survives decay:
        let mut s = ReputationLedger::new();
        s.record_delivery("c2");
        s.record_suspension("c2");
        s.decay(0.5);
        assert_eq!(s.score("c2"), 0.0, "suspension never decays");
    }
}
