//! AGENTIC GIT — GCC pattern (arXiv:2508.00031 "Manage the Context of Agents
//! by Agentic Git"), reverse-engineered native + 0 deps.
//!
//! Treat an agent's memory like a git repo: every action snapshots the live
//! `LivingMemory` into a content-addressed, append-only COMMIT. `CONTEXT(hash)`
//! reconstructs the EXACT memory state at any point → a deterministic,
//! tamper-evident audit trail of the agent's own actions. `MERGE` joins two
//! histories. No RNG/clock in hashes → reproducible.
//!
//! This is the MAX-EV finding from the agentic-git-history theme: the 7 listed
//! repos (Aisdkagents, cult-ui, aliimam, styles-refero, skiper-ui, yt-dlb,
//! mgchev/skills-best-practices) are component/DESIGN.md tooling, NOT
//! agentic-git tools — so we implement the pattern ourselves, on top of the
//! existing `LivingMemory` + `vault` + `AuditLog` core, rather than integrate
//! any of them.

use crate::memory::{simple_hash, LivingMemory, MemoryNode};
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq)]
pub struct Commit {
    pub hash: String,
    pub parents: Vec<String>,
    pub message: String,
    /// content hash of the serialized memory state at commit time
    pub state_hash: String,
    pub seq: u64,
}

/// A content-addressed chain of memory snapshots.
pub struct AgenticGit {
    head: Option<String>,
    commits: HashMap<String, Commit>,
    /// concept → payload snapshot, keyed by commit hash
    states: HashMap<String, HashMap<String, String>>,
    seq: u64,
}

impl AgenticGit {
    pub fn new() -> Self {
        AgenticGit {
            head: None,
            commits: HashMap::new(),
            states: HashMap::new(),
            seq: 0,
        }
    }

    /// COMMIT: snapshot `mem` as a child of the current head.
    pub fn commit(&mut self, mem: &LivingMemory, message: &str) -> String {
        let parent = self.head.clone();
        self.commit_with(message, parent, mem)
    }

    /// MERGE: a commit with two parents (self.head + other.head), snapshotting `mem`.
    pub fn merge(&mut self, other: &AgenticGit, mem: &LivingMemory, message: &str) -> String {
        let mut parents = Vec::new();
        if let Some(h) = &self.head {
            parents.push(h.clone());
        }
        if let Some(h) = &other.head {
            parents.push(h.clone());
        }
        self.commit_with(
            message,
            if parents.is_empty() {
                None
            } else {
                Some(parents.join("+"))
            },
            mem,
        )
    }

    fn commit_with(
        &mut self,
        message: &str,
        parent_key: Option<String>,
        mem: &LivingMemory,
    ) -> String {
        let parents: Vec<String> = match &parent_key {
            None => vec![],
            Some(k) => k.split('+').map(|s| s.to_string()).collect(),
        };
        let state = snapshot(mem);
        let serialized = serialize(&state);
        let state_hash = format!("{:08x}", simple_hash(serialized.as_bytes()));
        let seq = self.seq;
        let hash_input = format!("{:?}|{}|{}|{}", parents, state_hash, message, seq);
        let hash = format!("{:08x}", simple_hash(hash_input.as_bytes()));
        self.commits.insert(
            hash.clone(),
            Commit {
                hash: hash.clone(),
                parents: parents.clone(),
                message: message.into(),
                state_hash,
                seq,
            },
        );
        self.states.insert(hash.clone(), state);
        self.head = Some(hash.clone());
        self.seq += 1;
        hash
    }

    /// CONTEXT(hash): reconstruct the exact memory state at a commit (GREEN:
    /// returns the snapshot; RED: unknown hash → None).
    pub fn context(&self, hash: &str) -> Option<HashMap<String, String>> {
        self.states.get(hash).cloned()
    }

    /// LOG: commits from root → head (chronological). Honors merge parents by
    /// walking the primary (first) parent chain; full DAG reachable via `all`.
    pub fn log(&self) -> Vec<Commit> {
        let mut out = Vec::new();
        let mut cur = self.head.clone();
        while let Some(h) = cur {
            let c = match self.commits.get(&h) {
                Some(c) => c.clone(),
                None => break,
            };
            out.push(c.clone());
            cur = c.parents.first().cloned();
        }
        out.reverse();
        out
    }

    /// All commits (full DAG), ordered by seq.
    pub fn all(&self) -> Vec<Commit> {
        let mut v: Vec<Commit> = self.commits.values().cloned().collect();
        v.sort_by_key(|c| c.seq);
        v
    }

    pub fn head(&self) -> Option<&str> {
        self.head.as_deref()
    }

    /// Tamper-evident integrity check: every stored state must still hash to its
    /// commit's `state_hash`, and every commit hash must reproduce. Returns false
    /// if ANY state was mutated after the fact (the load-bearing audit property).
    pub fn verify_integrity(&self) -> bool {
        for c in self.commits.values() {
            let state = match self.states.get(&c.hash) {
                Some(s) => s,
                None => return false,
            };
            let recomputed_state = format!("{:08x}", simple_hash(serialize(state).as_bytes()));
            if recomputed_state != c.state_hash {
                return false;
            }
            let parents_key = if c.parents.is_empty() {
                None
            } else {
                Some(c.parents.join("+"))
            };
            let hash_input = format!("{:?}|{}|{}|{}", c.parents, c.state_hash, c.message, c.seq);
            let recomputed_hash = format!(
                "{:08x}",
                simple_hash(if parents_key.is_some() {
                    hash_input.as_bytes()
                } else {
                    hash_input.as_bytes()
                })
            );
            if recomputed_hash != c.hash {
                return false;
            }
        }
        true
    }
}

impl Default for AgenticGit {
    fn default() -> Self {
        Self::new()
    }
}

/// Deterministically serialize a memory snapshot (sorted by concept → stable hash).
fn snapshot(mem: &LivingMemory) -> HashMap<String, String> {
    let mut m = HashMap::new();
    for (_, n) in mem.nodes() {
        m.insert(n.concept.clone(), n.payload.clone());
    }
    m
}

fn serialize(state: &HashMap<String, String>) -> String {
    let mut v: Vec<(&String, &String)> = state.iter().collect();
    v.sort_by_key(|(k, _)| *k);
    v.iter()
        .map(|(k, p)| format!("{}:{}", k, p))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Rebuild a `LivingMemory` from a reconstructed snapshot (replay utility).
pub fn replay(state: &HashMap<String, String>) -> LivingMemory {
    let mut m = LivingMemory::new();
    for (concept, payload) in state {
        m.remember(concept, payload);
    }
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mem_with(items: &[(&str, &str)]) -> LivingMemory {
        let mut m = LivingMemory::new();
        for (c, p) in items {
            m.remember(c, p);
        }
        m
    }

    #[test]
    fn commit_is_content_addressed_and_deterministic() {
        // GREEN: same parent+state+msg+seq → identical hash (reproducible).
        let m = mem_with(&[("auth", "login boundary")]);
        let mut g1 = AgenticGit::new();
        let h1 = g1.commit(&m, "add auth");
        let mut g2 = AgenticGit::new();
        let h2 = g2.commit(&m, "add auth");
        assert_eq!(h1, h2, "content-addressed hash must be deterministic");
        // RED: different message → different hash (no collisions masquerading as equal)
        let mut g3 = AgenticGit::new();
        let h3 = g3.commit(&m, "add auth (edited)");
        assert_ne!(h1, h3, "different message must change the hash");
    }

    #[test]
    fn context_reconstructs_exact_state() {
        // GREEN: CONTEXT(hash) returns the precise memory at that commit.
        let mut m = mem_with(&[("auth", "login boundary")]);
        let mut g = AgenticGit::new();
        let h = g.commit(&m, "seed");
        m.remember("session", "token lifetime");
        let h2 = g.commit(&m, "add session");

        let at_h = g.context(&h).expect("seed context present");
        assert_eq!(at_h.len(), 1);
        assert!(at_h.contains_key("auth"));
        assert!(
            !at_h.contains_key("session"),
            "seed snapshot predates session"
        );

        let at_h2 = g.context(&h2).expect("session context present");
        assert_eq!(at_h2.len(), 2);
        // RED: unknown hash → None (no fabricated history)
        assert!(g.context("deadbeef").is_none());
    }

    #[test]
    fn log_walks_chronologically() {
        let mut m = mem_with(&[("a", "1")]);
        let mut g = AgenticGit::new();
        g.commit(&m, "c0");
        m.remember("b", "2");
        g.commit(&m, "c1");
        let log = g.log();
        assert_eq!(log.len(), 2);
        assert_eq!(log[0].message, "c0");
        assert_eq!(log[1].message, "c1");
        assert_eq!(g.head(), Some(log[1].hash.as_str()));
    }

    #[test]
    fn integrity_detects_tamper() {
        // GREEN: pristine chain verifies
        let m = mem_with(&[("auth", "login boundary")]);
        let mut g = AgenticGit::new();
        g.commit(&m, "seed");
        assert!(g.verify_integrity(), "clean chain must verify");

        // RED: mutate a stored state → integrity must FAIL (tamper-evident)
        let head = g.head().unwrap().to_string();
        if let Some(s) = g.states.get_mut(&head) {
            s.insert("auth".into(), "PWNED".into());
        }
        assert!(!g.verify_integrity(), "mutated state must break integrity");
    }

    #[test]
    fn merge_joins_two_histories() {
        let mut ma = mem_with(&[("a", "1")]);
        let mut ga = AgenticGit::new();
        ga.commit(&ma, "a0");
        ma.remember("a2", "x");
        ga.commit(&ma, "a1");

        let mut mb = mem_with(&[("b", "2")]);
        let mut gb = AgenticGit::new();
        gb.commit(&mb, "b0");

        // merged chain snapshots union state, with two parents
        let mut union = mem_with(&[("a", "1"), ("a2", "x"), ("b", "2")]);
        let mh = ga.merge(&gb, &union, "merge");
        let mc = ga.commits.get(&mh).unwrap();
        assert_eq!(mc.parents.len(), 2, "merge commit has two parents");
        assert_eq!(ga.context(&mh).unwrap().len(), 3, "merged state = union");
        assert!(ga.verify_integrity());
    }
}
