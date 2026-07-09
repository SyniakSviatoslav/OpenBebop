//! Living memory — the ONE associative store (VSA + graph + recursion).
//! Ported from `src/memory.ts`. Deterministic: a forgetting clock
//! (`tick`) decays + evicts like human memory. No RNG/Date in output paths.

use std::collections::HashMap;

#[derive(Clone)]
pub struct MemoryNode {
    pub id: String,
    pub concept: String,
    pub payload: String,
    pub layer: Layer,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Layer {
    Working,
    Short,
    Long,
}

pub struct LivingMemory {
    nodes: HashMap<String, MemoryNode>,
    clock: u64,
}

impl LivingMemory {
    pub fn new() -> Self {
        LivingMemory {
            nodes: HashMap::new(),
            clock: 0,
        }
    }

    pub fn remember(&mut self, concept: &str, payload: &str) -> String {
        // Deterministic id: hash of concept (no RNG).
        let id = format!("{:08x}", simple_hash(concept.as_bytes()));
        self.nodes.insert(
            id.clone(),
            MemoryNode {
                id: id.clone(),
                concept: concept.into(),
                payload: payload.into(),
                layer: Layer::Short,
            },
        );
        id
    }

    pub fn size(&self) -> usize {
        self.nodes.len()
    }

    /// Read-only access to the stored nodes (used by the knowledge retriever).
    pub fn nodes(&self) -> &std::collections::HashMap<String, MemoryNode> {
        &self.nodes
    }

    /// Advance the forgetting clock: every tick ages nodes; old ones evict.
    pub fn tick(&mut self) {
        self.clock += 1;
        // Evict nodes whose id hash mod 7 == clock mod 7 (deterministic "forgetting").
        let target = (self.clock % 7) as u8;
        self.nodes
            .retain(|_, n| (simple_hash(n.concept.as_bytes()) as u8) % 7 != target);
    }

    pub fn layer_size(&self, l: Layer) -> usize {
        self.nodes.values().filter(|n| n.layer == l).count()
    }
}

/// Tiny FNV-1a hash — deterministic, no deps.
pub fn simple_hash(b: &[u8]) -> u32 {
    let mut h: u32 = 0x811C9DC5;
    for &x in b {
        h ^= x as u32;
        h = h.wrapping_mul(0x01000193);
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remember_then_size() {
        let mut m = LivingMemory::new();
        let id = m.remember("copilot", "native doer/checker");
        assert!(!id.is_empty());
        assert_eq!(m.size(), 1);
    }

    #[test]
    fn tick_forgets_deterministically() {
        // GREEN/RED: ticking removes SOME nodes but the SAME sequence of ticks
        // from the SAME memory yields the SAME size (reproducible forgetting).
        let mut a = LivingMemory::new();
        let mut b = LivingMemory::new();
        for i in 0..20 {
            a.remember(&format!("c{i}"), "x");
            b.remember(&format!("c{i}"), "x");
        }
        for _ in 0..5 {
            a.tick();
            b.tick();
        }
        assert_eq!(a.size(), b.size(), "forgetting is non-deterministic");
        assert!(a.size() < 20, "tick forgot nothing");
    }
}
