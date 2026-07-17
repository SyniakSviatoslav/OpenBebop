//! P10 (M5/F1) — `HubPolicy` as *data*: the single operator-editable runtime
//! entity holding every rule that is today a compile-time constant.
//!
//! `HubPolicy` is loaded at boot from `config/hub-policy.txt` and hot-reloaded
//! (via a stat-`mtime` poll — DECART §8.2, zero new dependency). It is
//! deny-by-default: an absent file yields [`HubPolicy::deny_all_default`] (no
//! listeners, no bridges, red-line default-DENY). It NEVER fails open.
//!
//! # Concurrent swap (DECART §8.2 decision)
//! The live policy is held in a [`std::sync::RwLock<Arc<HubPolicy>>`] (zero new
//! dependency; `arc-swap` was DECART-rejected here). Readers `load()` an
//! `Arc` snapshot; [`apply_revision`](PolicyStore::apply_revision) validates a
//! candidate, floor-gates it, then atomically swaps the `Arc` under the write
//! lock. A frame in flight completes against the snapshot it began with.
//!
//! # Floor-gate (M12)
//! A revision may **never widen a red-line scope** (Auth / Money / Secrets /
//! Migrations). A candidate that adds a red-line allow-list entry not already
//! present is REFUSED and logged `REJECTED`; the last-good revision stays live.
//! This is the invariant Phase 15 inherits when it lets a hub self-author.
//!
//! CI GUARD: NO-COURIER-SCORING — a policy names ports/bridges/scopes, no score.

use std::sync::{Arc, RwLock};

use bebop2_core::hash::sha3_256;
use bebop_proto_cap::HybridPolicy;

/// A single inbound listener the hub may open (F2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListenerSpec {
    /// Bind address, e.g. `0.0.0.0:9443`. Opaque string (carrier resolves it).
    pub bind: String,
    /// Whether this listener is enabled. A disabled (or absent) listener is
    /// never opened — deny-by-default.
    pub enabled: bool,
}

/// A bridge to a peer endpoint (F8). Each bridge carries its OWN `HybridPolicy`
/// — a hub may run one `RequireBoth` and one `ClassicalUntilPqAudit` bridge at
/// once; the latter is surfaced as an insecure-bridge telemetry flag (§5.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BridgeSpec {
    /// Stable id for the bridge (used in telemetry).
    pub bridge_id: String,
    /// Peer endpoint the bridge connects to.
    pub endpoint: String,
    /// Per-bridge hybrid policy. `ClassicalUntilPqAudit` => insecure flag.
    pub hybrid: HybridPolicy,
}

/// A model endpoint manifest reference (M5). Ingestion/verification is Phase 15;
/// P10 only carries the `{url, sha3}` reference as data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelEndpoint {
    /// Model manifest URL.
    pub url: String,
    /// Expected SHA3-256 (hex) of the manifest (verified in Phase 15).
    pub sha3_hex: String,
}

/// Red-line allow-list, as *data* (M12). Each entry is a red-line category name
/// the operator has explicitly allow-listed. Empty = deny-all (the floor). The
/// floor-gate refuses any revision that ADDS an entry not already present.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RedLinePolicyData {
    /// Explicitly allow-listed red-line category names (e.g. "money").
    /// Empty means deny-by-default (fail-closed).
    pub allow: Vec<String>,
}

/// Rate-limit config (F2): sizes for the new-listener bucket and the per-IP
/// accept bucket. Data only; the buckets themselves live in `transport_policy`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RateLimitConfig {
    /// New-listener-open bucket capacity.
    pub new_listener_capacity: u32,
    /// New-listener-open bucket refill per second.
    pub new_listener_refill_per_sec: u32,
    /// Per-IP accept bucket capacity.
    pub accept_capacity: u32,
    /// Per-IP accept bucket refill per second.
    pub accept_refill_per_sec: u32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        RateLimitConfig {
            new_listener_capacity: 4,
            new_listener_refill_per_sec: 1,
            accept_capacity: 32,
            accept_refill_per_sec: 8,
        }
    }
}

/// The single operator-editable runtime policy entity (§2.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HubPolicy {
    /// Monotonic revision; bumped on every successful apply.
    pub revision: u64,
    /// Canonical hash of this revision (audit).
    pub policy_sha3: [u8; 32],
    /// Inbound listeners (F2).
    pub listeners: Vec<ListenerSpec>,
    /// Bridges (F8).
    pub bridges: Vec<BridgeSpec>,
    /// Model endpoint manifest refs (M5).
    pub model_endpoints: Vec<ModelEndpoint>,
    /// Red-line allow-list (M12).
    pub red_line_policy: RedLinePolicyData,
    /// Rate-limit sizes (F2).
    pub rate_limits: RateLimitConfig,
}

impl HubPolicy {
    /// The deny-by-default policy (§2.2): no listeners, no bridges, red-line
    /// deny-all. Used when `config/hub-policy.txt` is absent. Never fails open.
    pub fn deny_all_default() -> Self {
        let mut p = HubPolicy {
            revision: 0,
            policy_sha3: [0u8; 32],
            listeners: Vec::new(),
            bridges: Vec::new(),
            model_endpoints: Vec::new(),
            red_line_policy: RedLinePolicyData::default(),
            rate_limits: RateLimitConfig::default(),
        };
        p.policy_sha3 = p.canonical_hash();
        p
    }

    /// Canonical-ish hash of the policy content for audit. Not on any signed
    /// wire path — it identifies a revision for telemetry/audit. Deterministic:
    /// concatenate a stable textual serialization and SHA3-256 it.
    pub fn canonical_hash(&self) -> [u8; 32] {
        sha3_256(self.canonical_string().as_bytes())
    }

    /// Deterministic textual serialization (also the on-disk format, minus the
    /// derived `revision`/`policy_sha3`). Round-trips with [`HubPolicy::parse`].
    pub fn canonical_string(&self) -> String {
        let mut s = String::new();
        for l in &self.listeners {
            s.push_str(&format!("listener {} {}\n", l.bind, l.enabled));
        }
        for b in &self.bridges {
            let h = match b.hybrid {
                HybridPolicy::RequireBoth => "require_both",
                HybridPolicy::ClassicalUntilPqAudit => "classical_until_pq_audit",
            };
            s.push_str(&format!("bridge {} {} {}\n", b.bridge_id, b.endpoint, h));
        }
        for m in &self.model_endpoints {
            s.push_str(&format!("model {} {}\n", m.url, m.sha3_hex));
        }
        for a in &self.red_line_policy.allow {
            s.push_str(&format!("redline_allow {}\n", a));
        }
        s.push_str(&format!(
            "rate_limits {} {} {} {}\n",
            self.rate_limits.new_listener_capacity,
            self.rate_limits.new_listener_refill_per_sec,
            self.rate_limits.accept_capacity,
            self.rate_limits.accept_refill_per_sec
        ));
        s
    }

    /// Parse a `HubPolicy` from the operator-editable text format. Fail-closed:
    /// any malformed line yields `Err`, and the caller keeps the last-good
    /// revision (a bad edit never takes the hub down). Comments (`#`) and blank
    /// lines are ignored. `revision`/`policy_sha3` are derived, not parsed.
    pub fn parse(text: &str) -> Result<HubPolicy, PolicyError> {
        let mut p = HubPolicy::deny_all_default();
        for (i, raw) in text.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let toks: Vec<&str> = line.split_whitespace().collect();
            let err = |m: &str| PolicyError::Malformed(format!("line {}: {}", i + 1, m));
            match toks.as_slice() {
                ["listener", bind, enabled] => {
                    let enabled = parse_bool(enabled).ok_or_else(|| err("bad enabled bool"))?;
                    p.listeners.push(ListenerSpec {
                        bind: bind.to_string(),
                        enabled,
                    });
                }
                ["bridge", id, endpoint, hybrid] => {
                    let hybrid = match *hybrid {
                        "require_both" => HybridPolicy::RequireBoth,
                        "classical_until_pq_audit" => HybridPolicy::ClassicalUntilPqAudit,
                        _ => return Err(err("bad hybrid policy")),
                    };
                    p.bridges.push(BridgeSpec {
                        bridge_id: id.to_string(),
                        endpoint: endpoint.to_string(),
                        hybrid,
                    });
                }
                ["model", url, sha3] => {
                    p.model_endpoints.push(ModelEndpoint {
                        url: url.to_string(),
                        sha3_hex: sha3.to_string(),
                    });
                }
                ["redline_allow", cat] => {
                    p.red_line_policy.allow.push(cat.to_string());
                }
                ["rate_limits", a, b, c, d] => {
                    p.rate_limits = RateLimitConfig {
                        new_listener_capacity: a.parse().map_err(|_| err("bad rate"))?,
                        new_listener_refill_per_sec: b.parse().map_err(|_| err("bad rate"))?,
                        accept_capacity: c.parse().map_err(|_| err("bad rate"))?,
                        accept_refill_per_sec: d.parse().map_err(|_| err("bad rate"))?,
                    };
                }
                _ => return Err(err("unrecognized directive")),
            }
        }
        p.policy_sha3 = p.canonical_hash();
        Ok(p)
    }

    /// Load a policy from disk. Absent file => [`HubPolicy::deny_all_default`]
    /// (never fails open). A present-but-malformed file => `Err` (the caller
    /// keeps last-good). Returns the mtime so a watcher can poll for changes.
    pub fn load(path: &str) -> Result<HubPolicy, PolicyError> {
        match std::fs::read_to_string(path) {
            Ok(text) => HubPolicy::parse(&text),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(HubPolicy::deny_all_default()),
            Err(e) => Err(PolicyError::Io(e.to_string())),
        }
    }

    /// Which bridges are running a classical-until-PQ-audit (insecure) bridge
    /// (F8). Never silent: the caller emits/maintains a telemetry flag for each.
    pub fn insecure_bridges(&self) -> Vec<&BridgeSpec> {
        self.bridges
            .iter()
            .filter(|b| b.hybrid == HybridPolicy::ClassicalUntilPqAudit)
            .collect()
    }

    /// The set of enabled listeners' bind addresses (deny-by-default: a bind not
    /// present here is never opened).
    pub fn enabled_listener_binds(&self) -> Vec<&str> {
        self.listeners
            .iter()
            .filter(|l| l.enabled)
            .map(|l| l.bind.as_str())
            .collect()
    }
}

/// Errors from policy parse/apply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyError {
    /// A line of the policy file was malformed.
    Malformed(String),
    /// The policy file could not be read.
    Io(String),
    /// The candidate widens a red-line scope — floor-gate REFUSED (§2.2 step 2).
    RedLineWidened(String),
}

fn parse_bool(s: &str) -> Option<bool> {
    match s {
        "true" | "1" | "on" | "yes" => Some(true),
        "false" | "0" | "off" | "no" => Some(false),
        _ => None,
    }
}

/// The result of an `apply_revision`, for audit/telemetry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApplyOutcome {
    /// Applied; carries the new revision + sha3 (emit a `PolicyRevision` event).
    Applied { revision: u64, sha3: [u8; 32] },
    /// Rejected by the floor-gate (log `REJECTED`); last-good stays live.
    Rejected(String),
}

/// The live policy store: `RwLock<Arc<HubPolicy>>` (DECART §8.2). Readers take a
/// cheap `Arc` snapshot; writers validate + floor-gate + atomically swap.
#[derive(Debug)]
pub struct PolicyStore {
    inner: RwLock<Arc<HubPolicy>>,
}

impl PolicyStore {
    /// Build a store around an initial policy.
    pub fn new(initial: HubPolicy) -> Self {
        PolicyStore {
            inner: RwLock::new(Arc::new(initial)),
        }
    }

    /// Take a cheap `Arc` snapshot of the current policy (RCU-style read).
    pub fn load(&self) -> Arc<HubPolicy> {
        self.inner.read().expect("policy lock poisoned").clone()
    }

    /// Current revision number.
    pub fn revision(&self) -> u64 {
        self.load().revision
    }

    /// Apply a candidate revision (§2.2 step 2-3). Floor-gate: the candidate may
    /// NOT widen the red-line allow-list beyond the current live policy — any
    /// added red-line category is REFUSED and the last-good stays live. On a
    /// clean candidate: bump `revision`, recompute `policy_sha3`, atomically
    /// swap the `Arc`. Returns the outcome for audit.
    pub fn apply_revision(&self, mut candidate: HubPolicy) -> ApplyOutcome {
        let current = self.load();
        // Floor-gate: no NEW red-line allow entry beyond what is already live.
        for cat in &candidate.red_line_policy.allow {
            if !current.red_line_policy.allow.contains(cat) {
                return ApplyOutcome::Rejected(format!(
                    "REJECTED: revision widens red-line scope '{}' (floor-gate)",
                    cat
                ));
            }
        }
        // Clean: bump revision, recompute hash, atomically swap.
        candidate.revision = current.revision + 1;
        candidate.policy_sha3 = candidate.canonical_hash();
        let revision = candidate.revision;
        let sha3 = candidate.policy_sha3;
        let mut w = self.inner.write().expect("policy lock poisoned");
        *w = Arc::new(candidate);
        ApplyOutcome::Applied { revision, sha3 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── §6.10 RED (malformed): a malformed edit is rejected; last-good stays ──
    #[test]
    fn malformed_policy_is_rejected() {
        assert!(matches!(
            HubPolicy::parse("listener 0.0.0.0:1 maybe\n"),
            Err(PolicyError::Malformed(_))
        ));
        assert!(matches!(
            HubPolicy::parse("nonsense directive here\n"),
            Err(PolicyError::Malformed(_))
        ));
    }

    #[test]
    fn deny_all_default_opens_nothing() {
        let p = HubPolicy::deny_all_default();
        assert!(p.listeners.is_empty());
        assert!(p.bridges.is_empty());
        assert!(p.red_line_policy.allow.is_empty());
        assert!(p.enabled_listener_binds().is_empty());
    }

    #[test]
    fn parse_roundtrips_via_canonical_string() {
        let text = "\
listener 0.0.0.0:9443 true
listener 0.0.0.0:9444 false
bridge b1 peer:1 require_both
bridge b2 peer:2 classical_until_pq_audit
model https://m/x deadbeef
redline_allow money
rate_limits 4 1 32 8
";
        let p = HubPolicy::parse(text).unwrap();
        assert_eq!(p.listeners.len(), 2);
        assert_eq!(p.bridges.len(), 2);
        assert_eq!(p.enabled_listener_binds(), vec!["0.0.0.0:9443"]);
        assert_eq!(p.insecure_bridges().len(), 1);
        // canonical_string re-parses to an equal policy (idempotent).
        let p2 = HubPolicy::parse(&p.canonical_string()).unwrap();
        assert_eq!(p.listeners, p2.listeners);
        assert_eq!(p.bridges, p2.bridges);
    }

    // ── §6.8 GREEN: editing a field takes effect without restart (swap) ──
    #[test]
    fn apply_revision_swaps_live_policy_without_restart() {
        let store = PolicyStore::new(HubPolicy::deny_all_default());
        assert_eq!(store.revision(), 0);
        assert!(store.load().enabled_listener_binds().is_empty());

        // A revision that opens a port applies and bumps revision.
        let cand = HubPolicy::parse("listener 0.0.0.0:9443 true\n").unwrap();
        let out = store.apply_revision(cand);
        assert!(matches!(out, ApplyOutcome::Applied { revision: 1, .. }));
        assert_eq!(store.revision(), 1);
        assert_eq!(store.load().enabled_listener_binds(), vec!["0.0.0.0:9443"]);

        // Removing the port stops it while a still-listed port keeps working.
        let cand2 = HubPolicy::parse("listener 0.0.0.0:9444 true\n").unwrap();
        assert!(matches!(
            store.apply_revision(cand2),
            ApplyOutcome::Applied { revision: 2, .. }
        ));
        assert_eq!(store.load().enabled_listener_binds(), vec!["0.0.0.0:9444"]);
    }

    // ── §6.9 RED (floor-gate): widening a red-line scope is REFUSED ──
    #[test]
    fn floor_gate_refuses_red_line_widening() {
        let store = PolicyStore::new(HubPolicy::deny_all_default());
        // Candidate adds a red-line allow entry not present in the live policy.
        let widen = HubPolicy::parse("redline_allow money\n").unwrap();
        let out = store.apply_revision(widen);
        match out {
            ApplyOutcome::Rejected(msg) => assert!(msg.contains("REJECTED")),
            other => panic!("expected Rejected, got {:?}", other),
        }
        // Last-good stays live (revision unchanged, still deny-all).
        assert_eq!(store.revision(), 0);
        assert!(store.load().red_line_policy.allow.is_empty());

        // A floor-clean revision applies and bumps revision.
        let clean = HubPolicy::parse("listener 0.0.0.0:1 true\n").unwrap();
        assert!(matches!(
            store.apply_revision(clean),
            ApplyOutcome::Applied { revision: 1, .. }
        ));
    }

    // ── §6.16 F8: insecure-bridge flag is observable, never silent ──
    #[test]
    fn insecure_bridge_flag_tracks_classical_until_pq() {
        let p = HubPolicy::parse("bridge b1 peer:1 classical_until_pq_audit\n").unwrap();
        assert_eq!(p.insecure_bridges().len(), 1);
        assert_eq!(p.insecure_bridges()[0].bridge_id, "b1");
        // Flip to RequireBoth clears the flag.
        let p2 = HubPolicy::parse("bridge b1 peer:1 require_both\n").unwrap();
        assert!(p2.insecure_bridges().is_empty());
        // Removing the bridge also clears it.
        let p3 = HubPolicy::deny_all_default();
        assert!(p3.insecure_bridges().is_empty());
    }

    #[test]
    fn load_absent_file_is_deny_all_never_fails_open() {
        let p = HubPolicy::load("/nonexistent/hub-policy-xyz.txt").unwrap();
        assert_eq!(p, HubPolicy::deny_all_default());
    }
}
