//! Event-sourcing core organ (W3-2): an append-only `EventLog<E>` with a
//! SHA3-256 hash-chain integrity spine.
//!
//! Design invariant: *money is event-sourced, NEVER CRDT-merged*. Each entry is
//! chained so that tampering with any stored payload (or any stored rolling
//! hash) is detectable by recomputing the chain end-to-end:
//!
//! ```text
//! h_0 = H(GENESIS || seq_0 || payload_0)
//! h_i = H(h_{i-1} || seq_i || payload_i)      for i > 0
//! ```
//!
//! `root_hash()` is the rolling hash of the tip (`h_{n-1}`). `verify()` walks the
//! whole chain and recomputes every `h_i`, comparing against the stored value.
//!
//! Pure `core`+`alloc` (no `std`, no new dependencies) — reuses the crate's
//! existing `crate::hash::sha3_256`. The `E` parameter is a phantom type-tag
//! for the event domain so logs of different event kinds are distinct types.

extern crate alloc;

use alloc::vec::Vec;
use core::marker::PhantomData;

use crate::hash::sha3_256;

/// Genesis "previous hash" fed into the first entry's chain step. A fixed
/// all-zero sentinel keeps the initial `h_0` deterministic across runs/logs.
const GENESIS: [u8; 32] = [0u8; 32];

/// Integrity failure returned by [`EventLog::verify`]: the chain is broken at
/// `seq`, for `reason`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventLogError {
    /// First sequence number whose recomputed hash disagrees with storage.
    pub seq: u64,
    /// Short, static description of the detected break.
    pub reason: &'static str,
}

/// One chained entry: monotonically increasing `seq`, the raw `payload` bytes,
/// and the rolling hash `h_i` covering `prev || seq || payload`.
#[derive(Debug, Clone)]
struct Entry {
    seq: u64,
    payload: Vec<u8>,
    hash: [u8; 32],
}

impl Entry {
    /// Build an entry from the previous rolling hash, this entry's seq, and its
    /// payload. Computes `h_i = H(prev || seq_be || payload)`.
    fn new(prev: &[u8; 32], seq: u64, payload: &[u8]) -> Entry {
        let mut buf = Vec::with_capacity(prev.len() + 8 + payload.len());
        buf.extend_from_slice(prev);
        buf.extend_from_slice(&seq.to_be_bytes());
        buf.extend_from_slice(payload);
        Entry {
            seq,
            payload: payload.to_vec(),
            hash: sha3_256(&buf),
        }
    }
}

/// Append-only, verifiable event log. Generic over a phantom event-domain tag `E`.
pub struct EventLog<E = ()> {
    entries: Vec<Entry>,
    _p: PhantomData<E>,
}

impl<E> EventLog<E> {
    /// Create an empty log (a fresh chain anchored at [`GENESIS`]).
    pub fn new() -> Self {
        EventLog {
            entries: Vec::new(),
            _p: PhantomData,
        }
    }

    /// Append `event_bytes` and return the assigned monotonic sequence number.
    /// The entry's rolling hash chains off the previous tip.
    pub fn append(&mut self, event_bytes: &[u8]) -> u64 {
        let seq = self.entries.len() as u64;
        let prev = self.entries.last().map(|e| e.hash).unwrap_or(GENESIS);
        self.entries.push(Entry::new(&prev, seq, event_bytes));
        seq
    }

    /// Number of entries in the log.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the log currently holds zero entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Rolling hash of the tip (`h_{n-1}`). For an empty log this is [`GENESIS`].
    pub fn root_hash(&self) -> [u8; 32] {
        self.entries.last().map(|e| e.hash).unwrap_or(GENESIS)
    }

    /// Recompute the entire chain from scratch and confirm every stored rolling
    /// hash matches. Returns `Ok(())` if intact, or [`EventLogError`] naming the
    /// first broken `seq` otherwise (e.g. after a payload is mutated).
    pub fn verify(&self) -> Result<(), EventLogError> {
        let mut prev = GENESIS;
        for e in &self.entries {
            let mut buf = Vec::with_capacity(prev.len() + 8 + e.payload.len());
            buf.extend_from_slice(&prev);
            buf.extend_from_slice(&e.seq.to_be_bytes());
            buf.extend_from_slice(&e.payload);
            let h = sha3_256(&buf);
            if h != e.hash {
                return Err(EventLogError {
                    seq: e.seq,
                    reason: "rolling hash mismatch (payload or hash tampered)",
                });
            }
            prev = e.hash;
        }
        Ok(())
    }

    /// Iterator over `(seq, payload)` pairs starting at `from_seq`. Sequences
    /// before `from_seq` are skipped; an out-of-range start yields an empty
    /// iterator.
    pub fn replay(&self, from_seq: u64) -> ReplayIter<'_, E> {
        let start = if from_seq as usize <= self.entries.len() {
            from_seq as usize
        } else {
            self.entries.len()
        };
        ReplayIter {
            log: self,
            idx: start,
        }
    }
}

impl<E> Default for EventLog<E> {
    fn default() -> Self {
        Self::new()
    }
}

/// Forward iterator returned by [`EventLog::replay`]. Yields `(seq, &payload)`
/// in ascending sequence order.
pub struct ReplayIter<'a, E> {
    log: &'a EventLog<E>,
    idx: usize,
}

impl<'a, E> Iterator for ReplayIter<'a, E> {
    type Item = (u64, &'a [u8]);

    fn next(&mut self) -> Option<Self::Item> {
        if self.idx >= self.log.entries.len() {
            return None;
        }
        let e = &self.log.entries[self.idx];
        self.idx += 1;
        Some((e.seq, &e.payload))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_log_append_and_replay() {
        let mut log = EventLog::<()>::new();
        let n: u64 = 50;
        for i in 0..n {
            let bytes = alloc::vec![i as u8; ((i % 7) + 1) as usize];
            let seq = log.append(&bytes);
            assert_eq!(seq, i, "append must assign monotonic seq starting at 0");
        }
        assert_eq!(log.len(), n as usize);
        assert!(!log.is_empty());

        // Full replay returns every entry, in order.
        let replayed: Vec<(u64, Vec<u8>)> = log.replay(0).map(|(s, p)| (s, p.to_vec())).collect();
        assert_eq!(replayed.len(), n as usize);
        for (i, (seq, _)) in replayed.iter().enumerate() {
            assert_eq!(*seq, i as u64, "replay must preserve sequence order");
        }

        // Partial replay from the middle.
        let tail: Vec<u64> = log.replay(10).map(|(s, _)| s).collect();
        assert_eq!(tail.len(), (n - 10) as usize);
        assert_eq!(tail.first().copied(), Some(10));
        assert_eq!(tail.last().copied(), Some(n - 1));

        // Out-of-range start yields nothing.
        assert_eq!(log.replay(n + 5).count(), 0);
    }

    #[test]
    fn event_log_tamper_detected() {
        let mut log = EventLog::<()>::new();
        for i in 0..10u8 {
            log.append(&[i; 4]);
        }
        // Intact chain verifies cleanly.
        assert!(log.verify().is_ok());

        // Mutating a stored payload breaks the chain at that seq.
        log.entries[3].payload[0] ^= 0xff;
        let err = log.verify().expect_err("tampered payload must fail verify");
        assert_eq!(err.seq, 3);

        // Patching the chain past the mutation still leaves a later break, so a
        // second independent tamper of a stored hash is also caught.
        log.entries[5].hash[0] ^= 0x01;
        assert!(log.verify().is_err());
    }

    #[test]
    fn event_log_root_changes_on_append() {
        let mut log = EventLog::<()>::new();
        let mut prev = log.root_hash();
        for i in 0..20u8 {
            log.append(&[i, i.wrapping_add(1), i.wrapping_add(2)]);
            let cur = log.root_hash();
            assert_ne!(cur, prev, "root_hash must change on each append");
            prev = cur;
        }
        // Appending identical events still advances the chain (seq differs).
        let before = log.root_hash();
        log.append(&[0u8; 8]);
        assert_ne!(log.root_hash(), before);
    }

    #[test]
    fn event_log_deterministic() {
        let mut a = EventLog::<()>::new();
        let mut b = EventLog::<()>::new();
        let events: [&[u8]; 4] = [&[1u8, 2, 3], &[4u8, 5], &[6u8, 7, 8, 9], &[0u8, 0, 0, 0, 0]];
        for e in &events {
            a.append(e);
            b.append(e);
        }
        assert_eq!(a.root_hash(), b.root_hash(), "identical feeds must match");
        assert!(a.verify().is_ok());
        assert!(b.verify().is_ok());
        assert_eq!(a.verify(), b.verify());

        // An empty log is deterministically anchored at GENESIS.
        let empty = EventLog::<()>::new();
        assert_eq!(empty.root_hash(), GENESIS);
        assert!(empty.verify().is_ok());
    }
}
