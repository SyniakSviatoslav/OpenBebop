//! Anti-entropy / divergence-diff (W4-3): pull-based sync layered on the
//! `EventLog` hash-chain.
//!
//! Given two views of an append-only event log, compute the divergence point and
//! the exact suffix one side is missing, so a puller can request precisely those
//! events. Pure, in-memory, deterministic, std-only — NO network, NO async. The
//! sync *transport* (iroh / separately-gated red-line unit) is out of scope; this
//! is only the algorithm that decides *what* to pull.
//!
//! The log is event-sourced (never CRDT-merged), so a fork can only be resolved
//! by truncating to the divergence point and re-appending the authoritative
//! suffix. Because `EventLog` exposes only append (no truncation), `apply_pull`
//! only resolves the "local is a clean prefix of remote" (behind) case; forks
//! are detected and reported but require an out-of-band reset.

extern crate alloc;

use alloc::vec::Vec;

use crate::event_log::{EventLog, EventLogError};
use crate::hash::sha3_256;

/// Genesis "previous hash" fed into the first chain step. Mirrors the private
/// `GENESIS` constant in `event_log.rs` so the digest can recompute each
/// rolling hash locally without reaching into `Entry`'s private fields.
const GENESIS: [u8; 32] = [0u8; 32];

/// A compact per-sequence fingerprint of an [`EventLog`]: `(seq, rolling_hash)`
/// for every entry, recomputed from the stored payloads.
///
/// Because the hash-chain is `h_i = H(prev || seq || payload)`, we can derive
/// each `h_i` locally from [`EventLog::replay`] without needing private access
/// to stored `Entry` hashes. Two logs that diverge produce differing digests at
/// the first divergent `seq`, which is exactly what [`diff`] needs.
pub fn digest<E>(log: &EventLog<E>) -> Vec<(u64, [u8; 32])> {
    let mut out = Vec::with_capacity(log.len());
    let mut prev = GENESIS;
    for (seq, payload) in log.replay(0) {
        let mut buf = Vec::with_capacity(prev.len() + 8 + payload.len());
        buf.extend_from_slice(&prev);
        buf.extend_from_slice(&seq.to_be_bytes());
        buf.extend_from_slice(payload);
        let h = sha3_256(&buf);
        out.push((seq, h));
        prev = h;
    }
    out
}

/// Result of comparing a local digest against a remote digest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncPlan {
    /// First sequence number at which `local` and `remote` disagree (or where
    /// `local` is missing entries). `None` means the two logs agree entirely and
    /// nothing needs to be pulled.
    pub divergence_seq: Option<u64>,
    /// First sequence number the local side must request from the remote.
    pub pull_from: u64,
    /// Number of entries (starting at `pull_from`) the local side must pull.
    pub pull_len: usize,
}

/// Compare `local` and `remote` digests (as produced by [`digest`]). Returns a
/// [`SyncPlan`] describing the first divergent sequence number and the exact
/// suffix (range `[pull_from, pull_from + pull_len)`) the local side must pull
/// from the remote to converge:
///
/// - Identical logs -> `divergence_seq == None`, empty `pull_len`.
/// - Local is a strict prefix of remote (behind) -> divergence at `local.len()`,
///   `pull_len` = the extra remote entries.
/// - Logs that fork mid-chain -> `divergence_seq` = first `seq` with a differing
///   hash; `pull_len` = remote entries from that `seq` onward.
/// - Local already contains everything remote has (local ahead or equal) ->
///   nothing to pull.
pub fn diff(local: &[(u64, [u8; 32])], remote: &[(u64, [u8; 32])]) -> SyncPlan {
    let overlap = core::cmp::min(local.len(), remote.len());
    let mut divergence: Option<usize> = None;
    for i in 0..overlap {
        if local[i].1 != remote[i].1 {
            divergence = Some(i);
            break;
        }
    }
    match divergence {
        Some(i) => SyncPlan {
            divergence_seq: Some(local[i].0),
            pull_from: local[i].0,
            pull_len: remote.len() - i,
        },
        None => {
            if remote.len() > local.len() {
                let from = local.len() as u64;
                SyncPlan {
                    divergence_seq: Some(from),
                    pull_from: from,
                    pull_len: remote.len() - local.len(),
                }
            } else {
                SyncPlan {
                    divergence_seq: None,
                    pull_from: 0,
                    pull_len: 0,
                }
            }
        }
    }
}

/// Append the missing suffix fetched from the remote onto `log` and re-verify
/// the resulting hash-chain.
///
/// `missing` must be the entries to pull, i.e. `remote.replay(plan.pull_from)`
/// collected into `[(seq, payload)]` form. The first pulled `seq` must equal
/// `log.len()` — the local log is a clean prefix of the remote (the "behind"
/// case). Returns the chain-verification result; on success `log.root_hash()`
/// equals the remote's.
///
/// A fork (local already holds entries past `pull_from`) cannot be resolved by
/// appending, so the first misaligned `seq` yields an error instead of silently
/// producing a broken chain.
pub fn apply_pull<E>(
    log: &mut EventLog<E>,
    missing: &[(u64, &[u8])],
) -> Result<(), EventLogError> {
    for (seq, payload) in missing {
        if *seq != log.len() as u64 {
            return Err(EventLogError {
                seq: *seq,
                reason: "pulled seq does not continue local chain (fork/overlap)",
            });
        }
        log.append(payload);
    }
    log.verify()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_log::EventLog;

    /// Append `n` deterministic events to `log`.
    fn feed(log: &mut EventLog<()>, n: u8) {
        for i in 0..n {
            log.append(&[i, i.wrapping_mul(2), i.wrapping_mul(3)]);
        }
    }

    #[test]
    fn anti_entropy_identical_logs_no_diff() {
        let mut a = EventLog::<()>::new();
        let mut b = EventLog::<()>::new();
        feed(&mut a, 20);
        feed(&mut b, 20);
        let da = digest(&a);
        let db = digest(&b);
        let plan = diff(&da, &db);
        assert_eq!(plan.divergence_seq, None, "identical logs must have no divergence");
        assert_eq!(plan.pull_len, 0, "identical logs must pull nothing");
        assert_eq!(plan.pull_from, 0);
    }

    #[test]
    fn anti_entropy_detects_missing_suffix() {
        let mut local = EventLog::<()>::new();
        let mut remote = EventLog::<()>::new();
        feed(&mut local, 10);
        feed(&mut remote, 10 + 7); // remote is 7 ahead
        let dl = digest(&local);
        let dr = digest(&remote);
        let plan = diff(&dl, &dr);
        assert_eq!(plan.divergence_seq, Some(10), "divergence at first missing seq");
        assert_eq!(plan.pull_from, 10);
        assert_eq!(plan.pull_len, 7, "must request exactly the 7 missing events");
        assert_eq!(plan.pull_len, remote.len() - local.len());
    }

    #[test]
    fn anti_entropy_apply_converges() {
        let mut local = EventLog::<()>::new();
        let mut remote = EventLog::<()>::new();
        feed(&mut local, 12);
        feed(&mut remote, 12 + 9); // remote has 9 extra
        let dl = digest(&local);
        let dr = digest(&remote);
        let plan = diff(&dl, &dr);
        // Pull exactly the missing suffix from the remote.
        let missing: Vec<(u64, &[u8])> = remote.replay(plan.pull_from).collect();
        assert_eq!(missing.len(), plan.pull_len);
        let res = apply_pull(&mut local, &missing);
        assert!(res.is_ok(), "chain must verify after pulling the suffix");
        assert_eq!(local.root_hash(), remote.root_hash(), "logs must converge");
        assert_eq!(local.len(), remote.len());
    }

    #[test]
    fn anti_entropy_divergent_fork_detected() {
        let mut local = EventLog::<()>::new();
        let mut remote = EventLog::<()>::new();
        // Shared prefix of 8 entries.
        for i in 0..8u8 {
            local.append(&[i; 3]);
            remote.append(&[i; 3]);
        }
        // Fork at seq 8 — divergent payloads.
        local.append(&[100; 3]);
        remote.append(&[200; 3]);
        // Continue each fork with distinct tails.
        local.append(&[101; 3]);
        remote.append(&[201; 3]);
        let dl = digest(&local);
        let dr = digest(&remote);
        let plan = diff(&dl, &dr);
        assert_eq!(plan.divergence_seq, Some(8), "fork point must be identified");
        assert_eq!(plan.pull_from, 8);
        assert_eq!(plan.pull_len, 2, "remote tail from seq 8..10");
        // Forks are detectable, not silently merged: root hashes differ.
        assert_ne!(local.root_hash(), remote.root_hash());
    }
}
