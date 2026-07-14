//! SPIKE (eval-gated): codebase-memory-mcp / cognee pattern — graph-first retrieval.
//!
//! Hypothesis: spreading-activation over a relationship graph beats flat
//! bag-of-bytes cosine WHEN the relevant answer is edge-connected but lexically
//! dissimilar (exactly the case graph RAG targets). We measure this on a
//! ground-truth oracle BEFORE wiring graph_rank anywhere. The eval gate below
//! asserts graph recall@k strictly beats flat — if it fails, we do NOT integrate
//! (YAGNI). GATE RESULT: graph won → `spread_activate` is reused by
//! `knowledge::recall_graph`.

use crate::knowledge::{bag_vec, cosine};

/// Core spreading-activation primitive (deterministic, 0 deps).
/// `base[i]` = node i's standalone score (e.g. cosine to query).
/// Returns a score where each node is also boosted by neighbors reachable
/// within `hops`, weighted by `decay^(hop)`. A node keeps the max activation it
/// receives (not a sum), so a 2-hop neighbor of a strongly-matching node still
/// surfaces even if it is lexically unlike the query.
pub fn spread_activate(base: &[f64], edges: &[Vec<usize>], hops: usize, decay: f64) -> Vec<f64> {
    let mut score = base.to_vec();
    for start in 0..base.len() {
        if base[start] <= 0.0 {
            continue;
        }
        let mut frontier = vec![(start, 0usize)];
        while let Some((node, dist)) = frontier.pop() {
            if dist >= hops {
                continue;
            }
            for &nb in &edges[node] {
                if nb == start {
                    continue;
                }
                let w = decay.powi((dist as i32) + 1);
                let add = w * base[start];
                if add > score[nb] {
                    score[nb] = add;
                }
                frontier.push((nb, dist + 1));
            }
        }
    }
    score
}

/// A relationship graph of string concepts (the codebase-memory-mcp shape).
pub struct Graph {
    concepts: Vec<String>,
    edges: Vec<Vec<usize>>,
}

impl Graph {
    pub fn new() -> Self {
        Graph {
            concepts: Vec::new(),
            edges: Vec::new(),
        }
    }
    pub fn add_node(&mut self, concept: &str) -> usize {
        let id = self.concepts.len();
        self.concepts.push(concept.to_string());
        self.edges.push(Vec::new());
        id
    }
    pub fn connect(&mut self, a: usize, b: usize) {
        if !self.edges[a].contains(&b) {
            self.edges[a].push(b);
        }
        if !self.edges[b].contains(&a) {
            self.edges[b].push(a);
        }
    }
    /// Graph-first rank: cosine + spreading activation over `hops`.
    pub fn graph_rank(&self, query: &str, k: usize, hops: usize, decay: f64) -> Vec<(usize, f64)> {
        let qv = bag_vec(query.as_bytes());
        let base: Vec<f64> = self
            .concepts
            .iter()
            .map(|c| cosine(&qv, &bag_vec(c.as_bytes())))
            .collect();
        let score = spread_activate(&base, &self.edges, hops, decay);
        let mut ranked: Vec<(usize, f64)> = score.into_iter().enumerate().collect();
        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        ranked.into_iter().take(k).collect()
    }
}

/// Flat (current approach) rank — cosine only, edges ignored.
pub fn flat_rank(g: &Graph, query: &str, k: usize) -> Vec<(usize, f64)> {
    let qv = bag_vec(query.as_bytes());
    let mut scored: Vec<(usize, f64)> = g
        .concepts
        .iter()
        .enumerate()
        .map(|(i, c)| (i, cosine(&qv, &bag_vec(c.as_bytes()))))
        .collect();
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    scored.into_iter().take(k).collect()
}

fn recall_at(ranked: &[(usize, f64)], gold: &[usize]) -> f64 {
    let hit: usize = ranked
        .iter()
        .map(|(i, _)| if gold.contains(i) { 1 } else { 0 })
        .sum();
    if gold.is_empty() {
        1.0
    } else {
        hit as f64 / gold.len() as f64
    }
}

/// Ground-truth oracle: a tiny codebase graph + gold relevant sets.
/// `auth` connects to `oauth`,`session`; `session`→`token`. `render`→`layout`.
/// `cargo`→`ship`. Queries ask for a component; gold = the connected component.
fn build_oracle() -> (Graph, Vec<(String, Vec<usize>)>) {
    let mut g = Graph::new();
    let auth = g.add_node("auth");
    let oauth = g.add_node("oauth");
    let session = g.add_node("session");
    let token = g.add_node("token");
    let render = g.add_node("render");
    let layout = g.add_node("layout");
    let cargo = g.add_node("cargo");
    let ship = g.add_node("ship");
    g.connect(auth, oauth);
    g.connect(auth, session);
    g.connect(session, token);
    g.connect(render, layout);
    g.connect(cargo, ship);
    let oracle = vec![
        ("auth".into(), vec![auth, oauth, session, token]),
        ("render".into(), vec![render, layout]),
        ("ship".into(), vec![cargo, ship]),
    ];
    (g, oracle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eval_gate_graph_beats_flat_on_oracle() {
        // THE GATE: integrate graph_rank only if graph recall@k > flat recall@k.
        let (g, oracle) = build_oracle();
        let n = oracle.len() as f64;
        let mut flat = 0.0;
        let mut graph = 0.0;
        for (q, gold) in &oracle {
            flat += recall_at(&flat_rank(&g, q, 4), gold);
            graph += recall_at(&g.graph_rank(q, 4, 2, 0.5), gold);
        }
        flat /= n;
        graph /= n;
        eprintln!(
            "EVAL GATE (codebase-memory-mcp): flat recall@4={:.3} graph recall@4={:.3}",
            flat, graph
        );
        assert!(
            graph > flat,
            "graph retrieval did NOT beat flat — do not integrate"
        );
        assert!((0.0..=1.0).contains(&graph));
    }

    #[test]
    fn graph_surfaces_edge_related_node_flat_misses() {
        // RED for the gate: a 1-hop neighbor that is lexically dissimilar must
        // surface under graph but be missed by flat.
        let (g, _) = build_oracle();
        let session = g.concepts.iter().position(|c| c == "session").unwrap();
        let flat_ids: Vec<usize> = flat_rank(&g, "auth", 4).iter().map(|(i, _)| *i).collect();
        assert!(
            !flat_ids.contains(&session),
            "flat should miss 'session' (no letter overlap)"
        );
        let gr_ids: Vec<usize> = g
            .graph_rank("auth", 4, 2, 0.5)
            .iter()
            .map(|(i, _)| *i)
            .collect();
        assert!(
            gr_ids.contains(&session),
            "graph must surface edge-connected 'session'"
        );
    }
}
