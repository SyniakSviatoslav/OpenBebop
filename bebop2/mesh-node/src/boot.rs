//! P10 (§4) — boot / genesis wiring, **fail-closed**.
//!
//! `boot()` is the hub's cold-start entry. It:
//!   1. loads `genesis` (anchors + kill-anchors) — REFUSES to start on an
//!      empty / missing / malformed genesis (fail-closed, §4.1),
//!   2. loads the `HubPolicy` (deny-by-default when absent) into a
//!      [`crate::hub_policy::PolicyStore`],
//!   3. wires the [`crate::kill_switch::KillSequence`] over the genesis kill
//!      anchors,
//!   4. seeds the [`RevocationDecider`] (F5).
//! The hot-reload watcher and the OperatorKill handler are attached by the async
//! runtime around this booted state (see `node.rs` docs).
//!
//! CI GUARD: NO-COURIER-SCORING — genesis lists keys/authorities, never scores.

use bebop_proto_cap::{pq_key_id, AnchorRoster, RevocationSet};

use crate::hub_policy::{HubPolicy, PolicyStore};
use crate::kill_switch::{KillAnchors, KillSequence};

/// Parsed genesis: the delegation-chain anchors + the unilateral kill anchors.
#[derive(Debug, Clone)]
pub struct Genesis {
    /// Delegation-chain anchor roster (proto-cap authority root).
    pub roster: AnchorRoster,
    /// Kill-anchor public keys (M9 unilateral halt authority, §4).
    pub kill_anchors: KillAnchors,
    /// Count of enrolled anchors (audit).
    pub anchor_count: usize,
    /// Count of enrolled kill anchors (audit).
    pub kill_anchor_count: usize,
}

/// Why boot refused to start (fail-closed).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BootError {
    /// Genesis file missing / unreadable.
    GenesisMissing(String),
    /// Genesis present but empty (no anchors) — refuse to start.
    GenesisEmpty,
    /// Genesis malformed (bad line) — refuse to start.
    GenesisMalformed(String),
    /// HubPolicy present-but-malformed at boot — refuse (a clean absent file is
    /// fine and yields deny-all).
    PolicyMalformed(String),
}

/// Parse a 32-byte hex key. `None` on any malformed input (fail-closed).
fn parse_key_hex(s: &str) -> Option<[u8; 32]> {
    if s.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    for i in 0..32 {
        out[i] = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(out)
}

/// Parse genesis text (§4). Format, one directive per line:
/// ```text
/// anchor <64-hex ed25519 pubkey>       # delegation-chain anchor
/// kill_anchor <64-hex ed25519 pubkey>  # unilateral kill authority
/// ```
/// Comments (`#`) and blanks ignored. Fail-closed: any malformed line, or a
/// genesis with ZERO anchors, returns `Err` (refuse to start).
pub fn parse_genesis(text: &str) -> Result<Genesis, BootError> {
    let mut roster = AnchorRoster::new();
    let mut kill_anchors = KillAnchors::new();
    let mut anchor_count = 0usize;
    let mut kill_anchor_count = 0usize;

    for (i, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let toks: Vec<&str> = line.split_whitespace().collect();
        let bad = |m: &str| BootError::GenesisMalformed(format!("line {}: {}", i + 1, m));
        match toks.as_slice() {
            ["anchor", key] => {
                let k = parse_key_hex(key).ok_or_else(|| bad("bad anchor key hex"))?;
                roster.enroll(&k);
                anchor_count += 1;
            }
            ["kill_anchor", key] => {
                let k = parse_key_hex(key).ok_or_else(|| bad("bad kill_anchor key hex"))?;
                kill_anchors.enroll(k);
                kill_anchor_count += 1;
            }
            _ => return Err(bad("unrecognized genesis directive")),
        }
    }

    // Fail-closed: a genesis with no delegation anchors cannot vouch for
    // anything — refuse to start rather than run wide-open.
    if roster.is_empty() {
        return Err(BootError::GenesisEmpty);
    }

    Ok(Genesis {
        roster,
        kill_anchors,
        anchor_count,
        kill_anchor_count,
    })
}

/// Load genesis from disk (fail-closed). Missing/unreadable => `GenesisMissing`;
/// empty/malformed => the respective error. NEVER returns an empty roster.
pub fn load_genesis(path: &str) -> Result<Genesis, BootError> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| BootError::GenesisMissing(format!("{}: {}", path, e)))?;
    parse_genesis(&text)
}

/// The fully-booted hub runtime state (the pieces the async loops share).
#[derive(Debug)]
pub struct BootedHub {
    /// Genesis authority roots.
    pub genesis: Genesis,
    /// Live, hot-swappable policy.
    pub policy: PolicyStore,
    /// Kill-switch sequencer (unilateral, no quorum).
    pub kill: KillSequence,
    /// Revocation decision path (F5).
    pub revocations: RevocationDecider,
}

/// Cold-start the hub (§4). Fail-closed on genesis; deny-by-default on policy.
///
/// `genesis_path` MUST exist and enroll ≥1 anchor. `policy_path` may be absent
/// (=> deny-all policy); a present-but-malformed policy refuses to start.
pub fn boot(genesis_path: &str, policy_path: &str) -> Result<BootedHub, BootError> {
    // 1. Genesis first — fail-closed.
    let genesis = load_genesis(genesis_path)?;
    // 2. Policy (deny-by-default when absent; refuse on malformed).
    let policy = match HubPolicy::load(policy_path) {
        Ok(p) => p,
        Err(e) => return Err(BootError::PolicyMalformed(format!("{:?}", e))),
    };
    let policy = PolicyStore::new(policy);
    // 3. Kill sequencer over the genesis kill anchors.
    let kill = KillSequence::new(genesis.kill_anchors.clone());
    // 4. Revocation decider (starts empty; operator verbs mutate it).
    let revocations = RevocationDecider::new();

    Ok(BootedHub {
        genesis,
        policy,
        kill,
        revocations,
    })
}

/// P10 (F5) — the revocation decision path. An operator verb mutates the local
/// [`RevocationSet`]; the change is enforced locally AND handed as a delta to
/// the P9 gossip seam so peers converge. This struct owns the local set and
/// records the pending outbound deltas.
#[derive(Debug, Default)]
pub struct RevocationDecider {
    set: RevocationSet,
    /// Deltas (revoked pq-key-ids) to hand to the P9 gossip seam.
    pending_gossip: Vec<[u8; 32]>,
}

impl RevocationDecider {
    /// New, empty decider.
    pub fn new() -> Self {
        RevocationDecider {
            set: RevocationSet::new(),
            pending_gossip: Vec::new(),
        }
    }

    /// Operator verb: revoke a subject's PQ public key. Enforced locally (added
    /// to the set) and queued as an outbound gossip delta (F5). Returns `true`
    /// if newly revoked, `false` if it was already revoked (idempotent).
    pub fn revoke(&mut self, subject_key_pq: &[u8]) -> bool {
        let id = pq_key_id(subject_key_pq);
        let newly = !self.set.is_revoked_key(&id);
        self.set.revoke_key(id);
        if newly {
            self.pending_gossip.push(id);
        }
        newly
    }

    /// Whether a subject's PQ key is locally revoked (enforcement query).
    pub fn is_revoked(&self, subject_key_pq: &[u8]) -> bool {
        self.set.is_revoked_key(&pq_key_id(subject_key_pq))
    }

    /// Borrow the live revocation set (to hand to `PeerDirectory::evict_revoked`
    /// and the frame-verify gate).
    pub fn set(&self) -> &RevocationSet {
        &self.set
    }

    /// Drain the pending gossip deltas for the P9 seam. After this the deltas
    /// are considered handed off (enforced locally regardless).
    pub fn drain_gossip(&mut self) -> Vec<[u8; 32]> {
        std::mem::take(&mut self.pending_gossip)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bebop2_core::sign;

    fn hex_key(b: u8) -> String {
        let (pk, _) = sign::keygen(&[b; 32]);
        pk.iter().map(|x| format!("{:02x}", x)).collect()
    }

    // ── §6.1 GREEN: boot loads a valid genesis + policy ──
    #[test]
    fn boot_with_valid_genesis_succeeds() {
        let dir = std::env::temp_dir();
        let gp = dir.join(format!("p10-genesis-{}.txt", std::process::id()));
        let pp = dir.join(format!("p10-policy-{}.txt", std::process::id()));
        std::fs::write(
            &gp,
            format!(
                "# genesis\nanchor {}\nkill_anchor {}\n",
                hex_key(0x10),
                hex_key(0x11)
            ),
        )
        .unwrap();
        std::fs::write(&pp, "listener 0.0.0.0:9443 true\n").unwrap();

        let hub = boot(gp.to_str().unwrap(), pp.to_str().unwrap()).unwrap();
        assert_eq!(hub.genesis.anchor_count, 1);
        assert_eq!(hub.genesis.kill_anchor_count, 1);
        assert_eq!(hub.policy.load().enabled_listener_binds(), vec!["0.0.0.0:9443"]);
        let _ = std::fs::remove_file(&gp);
        let _ = std::fs::remove_file(&pp);
    }

    // ── §6.2 RED: boot REFUSES on a missing genesis (fail-closed) ──
    #[test]
    fn boot_refuses_missing_genesis() {
        let err = boot("/nonexistent/genesis-xyz.txt", "/nonexistent/policy.txt");
        assert!(matches!(err, Err(BootError::GenesisMissing(_))));
    }

    // ── §6.2 RED: boot REFUSES on an empty genesis (no anchors) ──
    #[test]
    fn boot_refuses_empty_genesis() {
        assert!(matches!(parse_genesis("# only a comment\n"), Err(BootError::GenesisEmpty)));
        assert!(matches!(parse_genesis(""), Err(BootError::GenesisEmpty)));
    }

    // ── §6.2 RED: boot REFUSES on a malformed genesis ──
    #[test]
    fn boot_refuses_malformed_genesis() {
        assert!(matches!(
            parse_genesis("anchor not-hex\n"),
            Err(BootError::GenesisMalformed(_))
        ));
        assert!(matches!(
            parse_genesis("garbage line\n"),
            Err(BootError::GenesisMalformed(_))
        ));
    }

    // ── §6.14 F5: operator revoke enforces locally AND queues a gossip delta ──
    #[test]
    fn revocation_decision_enforces_and_queues_gossip() {
        let mut dec = RevocationDecider::new();
        let subject = vec![7u8; 1952]; // a stand-in ML-DSA pubkey blob
        assert!(!dec.is_revoked(&subject));
        assert!(dec.revoke(&subject)); // newly revoked
        assert!(dec.is_revoked(&subject)); // enforced locally
        assert!(!dec.revoke(&subject)); // idempotent
        let deltas = dec.drain_gossip();
        assert_eq!(deltas.len(), 1); // one outbound delta for P9
        assert!(dec.drain_gossip().is_empty()); // drained
    }
}
