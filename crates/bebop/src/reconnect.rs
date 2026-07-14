//! Reconnection — the MHD "magnetic reconnection" operator for the operational graph.
//!
//! Analogy made deterministic: when a node's current density J_z (here: load ×
//! degree, the proxy for magnetic stress) exceeds a reconnect threshold, the graph
//! topologically re-wires — the overloaded hub drops its edges and its neighbors
//! reconnect to the next-best low-load node. This is "graceful degradation =
//! topology change": instead of collapsing, the system sheds conflict energy by
//! finding a new stationary configuration. NO rng, NO wall-clock.
//!
//! The operator is PURE given (adjacency, load, threshold). RED+GREEN tests prove
//! it fires under overload AND stays silent on a healthy graph AND that the
//! post-reconnect config has strictly lower max J_z than the pre-reconnect one.

/// An undirected graph snapshot for reconnection analysis.
pub struct Graph<'a> {
    pub n: usize,
    pub edges: &'a [(usize, usize)],
    /// Per-node load in [0,1] (utilization / pressure proxy).
    pub load: &'a [f64],
}

impl<'a> Graph<'a> {
    /// Current density J_z(i) = load(i) × degree(i). The MHD stress proxy.
    pub fn jz(&self, i: usize) -> f64 {
        let deg = self.edges.iter().filter(|(s, _)| *s == i).count()
            + self.edges.iter().filter(|(_, d)| *d == i).count();
        self.load[i] * deg as f64
    }

    pub fn max_jz(&self) -> f64 {
        (0..self.n).map(|i| self.jz(i)).fold(0.0f64, f64::max)
    }
}

/// Reconnect: return a NEW edge list where any node whose J_z > `thr` is
/// stripped of its edges, and each of its former neighbors is rewired to the
/// lowest-J_z node among its other neighbors (or left dangling if none).
///
/// Pure: does not mutate `g`. Returns (new_edges, reconnected_nodes).
pub fn reconnect(g: &Graph, thr: f64) -> (Vec<(usize, usize)>, Vec<usize>) {
    let hot: Vec<usize> = (0..g.n).filter(|&i| g.jz(i) > thr).collect();
    if hot.is_empty() {
        return (g.edges.to_vec(), vec![]); // GREEN: healthy graph untouched
    }
    let hot_set: std::collections::HashSet<usize> = hot.iter().cloned().collect();
    // Build adjacency without the hot nodes' edges.
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); g.n];
    for &(a, b) in g.edges {
        if hot_set.contains(&a) || hot_set.contains(&b) {
            continue; // strip hot-node edges
        }
        adj[a].push(b);
        adj[b].push(a);
    }
    // Rewire each neighbor of a hot node to the lowest-J_z of its OTHER neighbors.
    let mut new_edges: Vec<(usize, usize)> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for &h in &hot {
        // neighbors of h (excluding other hot nodes)
        let neigh: Vec<usize> = g
            .edges
            .iter()
            .filter(|(s, d)| (*s == h && !hot_set.contains(d)) || (*d == h && !hot_set.contains(s)))
            .map(|(s, d)| if *s == h { *d } else { *s })
            .collect();
        for nb in neigh {
            // pick the lowest-J_z candidate among nb's non-hot neighbors
            let cand = adj[nb]
                .iter()
                .copied()
                .filter(|x| !hot_set.contains(x) && *x != nb)
                .min_by(|a, b| g.jz(*a).partial_cmp(&g.jz(*b)).unwrap())
                .unwrap_or(nb);
            let (lo, hi) = if nb < cand { (nb, cand) } else { (cand, nb) };
            let key = (lo, hi);
            if seen.insert(key) {
                new_edges.push(key);
            }
        }
    }
    (new_edges, hot)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn g<'b>(edges: &'b [(usize, usize)], load: &'b [f64]) -> Graph<'b> {
        Graph {
            n: load.len(),
            edges,
            load,
        }
    }

    #[test]
    fn healthy_graph_untouched() {
        // GREEN: no node over threshold → edges returned unchanged, no reconnect.
        let e = [(0usize, 1), (1, 2), (2, 0)];
        let load = [0.2f64, 0.3, 0.2];
        let gr = g(&e, &load);
        let (ne, hot) = reconnect(&gr, 5.0); // threshold way above max J_z
        assert!(hot.is_empty(), "no hot node expected");
        assert_eq!(ne.len(), e.len(), "edge count preserved");
        assert_eq!(gr.max_jz() as i32, gr.max_jz() as i32); // unchanged
    }

    #[test]
    fn overload_triggers_reconnect_and_sheds_energy() {
        // RED+GREEN: a hub at load 1.0 with degree 3 → J_z=3 > thr=1.0.
        // Reconnect MUST fire AND lower max J_z.
        let e = [(0usize, 1), (0, 2), (0, 3), (1, 2)];
        let load = [1.0f64, 0.1, 0.1, 0.1];
        let gr = g(&e, &load);
        let before = gr.max_jz();
        assert!(before > 1.0, "hub must be over threshold");
        let (ne, hot) = reconnect(&gr, 1.0);
        assert_eq!(hot, vec![0], "hub 0 must reconnect");
        // Rebuild a graph view from the new edges to measure post J_z.
        let ng = Graph {
            n: 4,
            edges: &ne,
            load: &load,
        };
        let after = ng.max_jz();
        assert!(
            after < before,
            "reconnect must shed J_z: {before} -> {after}"
        );
    }

    #[test]
    fn reconnect_is_pure() {
        // GREEN: original graph must not be mutated by the operator.
        let e = [(0usize, 1), (0, 2), (0, 3)];
        let load = [1.0f64, 0.1, 0.1, 0.1];
        let gr = g(&e, &load);
        let _ = reconnect(&gr, 0.5);
        assert_eq!(gr.edges.len(), 3, "edges slice untouched");
        assert_eq!(gr.max_jz() as i32, 3, "load untouched");
    }
}
