//! WAVEFIELD — geometric + wave simulation of the *connection graph* itself.
//!
//! Extension of the deterministic field/coherence core (see `field.rs`,
//! `coherence.rs`, `mathx.rs`). Your idea, made falsifiable: represent NOT just
//! memory/files but their CONNECTIONS — actions, methods, relations — as a
//! weighted geometric graph, then simulate WAVES over it and read off structure
//! (cycles, bottlenecks, runaway divergence, forbidden couplings).
//!
//! Pipeline (all pure, no RNG/clock — same doctrine as the rest of the core):
//!   1. `Node2D` — a memory / file / entity placed in 2-D space (geometry).
//!   2. `connection_edges` — edges weighted by 1/distance (closer ⇒ stronger
//!      coupling) AND by a `kind` tag (action | method | relation | data) so the
//!      *nature* of a link is part of the sim, not just its existence.
//!   3. `propagate_wave` — reuse coherence heat-kernel to propagate an impulse
//!      seeded on a node; the wavefront spreads along connections (NOT just
//!      adjacent — geometry bends the path).
//!   4. `graph_fourier` — eigenvalue proxy of the connection Laplacian → which
//!      modes (subgraphs) the wave excites (band-stop / notch detection).
//!   5. `floyd_cycle` — detect a cyclic dependency in actions (fast/slow ptr
//!      analog over the action edge list) → a loop in the plan graph.
//!   6. `field_divergence` — net outward activity at a node (mathx::divergence
//!      over the geometric vector field of edge momenta) → runaway hub check.
//!   7. `wave_probe` — compose all of the above into ONE `WaveVerdict`: a cycle
//!      on the red-line (action→secret→action) or a divergent hub forces
//!      `Unhealthy` (fail-closed); an isolated/banded-safe graph is `Permit`.
//!
//! No external model, no network. The thin live glue (real file graph, real
//! embeddings) lives OUTSIDE, behind an eval gate — this models the logic.

use crate::coherence;

/// A node in the geometric connection graph: a memory / file / entity.
#[derive(Debug, Clone, PartialEq)]
pub struct Node2D {
    pub id: String,
    /// Geometry: position in 2-D space. Distances between nodes drive coupling.
    pub x: f64,
    pub y: f64,
    /// If true, the node is a RED-LINE node (secrets/auth/money/migration).
    /// A wave that loops back into a red-line node is a fail-closed condition.
    pub red_line: bool,
}

/// The kind of connection between two nodes — the NATURE of the link, not just
/// its existence. Your idea: connections carry semantics (action/method/relation).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkKind {
    Action,   // an operation that mutates / transitions
    Method,   // a callable / function reference
    Relation, // a structural relationship (owns / contains / depends-on)
    Data,     // pure data flow
}

impl LinkKind {
    /// Semantic weight multiplier: actions are the most dangerous to loop, data
    /// the least. Used to scale the geometric coupling so a cycle of ACTIONS
    /// dominates the verdict over a cycle of plain data edges.
    pub fn weight(&self) -> f64 {
        match self {
            LinkKind::Action => 1.0,
            LinkKind::Method => 0.7,
            LinkKind::Relation => 0.5,
            LinkKind::Data => 0.3,
        }
    }
}

/// A weighted, kind-tagged edge in the connection graph.
#[derive(Debug, Clone)]
pub struct ConnEdge {
    pub from: usize,
    pub to: usize,
    pub kind: LinkKind,
    /// Geometric coupling (kind.weight() / distance).
    pub weight: f64,
}

/// Euclidean distance between two nodes (geometry).
pub fn dist(a: &Node2D, b: &Node2D) -> f64 {
    ((a.x - b.x).powi(2) + (a.y - b.y).powi(2)).sqrt()
}

/// Build kind-tagged, geometrically-weighted edges from explicit (from,to,kind)
/// triples, weighting each by kind.weight() / (geometric dist + ε). This is the
/// function that encodes ACTIONS / METHODS / RELATIONS into the sim.
pub fn connection_edges_kinded(
    nodes: &[Node2D],
    links: &[(usize, usize, LinkKind)],
) -> Vec<ConnEdge> {
    links
        .iter()
        .map(|&(i, j, k)| {
            let d = dist(&nodes[i], &nodes[j]).max(1e-6);
            ConnEdge {
                from: i,
                to: j,
                kind: k,
                weight: k.weight() / d,
            }
        })
        .collect()
}

/// Project the kind-weighted connection graph into the undirected graph form
/// `coherence::propagate` consumes: index pairs (from,to). Edge presence is
/// gated by coupling above `min_coupling` so weak/remote links don't dominate.
fn to_graph(edges: &[ConnEdge], min_coupling: f64) -> Vec<(usize, usize)> {
    edges
        .iter()
        .filter(|e| e.weight >= min_coupling)
        .map(|e| (e.from, e.to))
        .collect()
}

/// Propagate a wave impulse seeded on `seed_node` across the connection graph.
/// Reuses the deterministic heat-kernel `coherence::propagate` (no RNG). Returns
/// the n-vector field amplitude `u(t)` — the wavefront over memory/file space.
pub fn propagate_wave(
    nodes: &[Node2D],
    edges: &[ConnEdge],
    seed_node: usize,
    t: f64,
    coeff: f64,
    min_coupling: f64,
) -> Vec<f64> {
    let n = nodes.len();
    if n == 0 || seed_node >= n {
        return vec![];
    }
    let mut u0 = vec![0.0f64; n];
    u0[seed_node] = 1.0;
    let g = to_graph(edges, min_coupling);
    coherence::propagate(&u0, &g, t, coeff)
}

/// Graph-Fourier notch/band-stop proxy.
///
/// Reverse-engineered from the dossier (Band-Stop / Notch filter, Butterworth
/// magnitude, graph Laplacian spectrum): a real connection graph has a
/// *spectral gap*. If the wave excites a mode whose energy concentrates in a
/// narrow band (i.e. the propagated field has a high peak-to-spread ratio), the
/// graph has a resonant, poorly-damped substructure → flagged. We proxy the
/// "spectrum" by the spread of the propagated amplitude vector: a tight,
/// peaked distribution (high normalized peak share) = a resonant band = NOTCH.
///
/// Returns `(peak, notch_hit)`. `notch_hit` is true when the field's peak energy
/// share exceeds `concentration` (spectral concentration → brittle coupling).
pub fn graph_fourier_notch(field: &[f64], concentration: f64) -> (f64, bool) {
    if field.is_empty() {
        return (0.0, false);
    }
    let peak = field.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let sum: f64 = field.iter().map(|v| v.abs()).sum();
    if sum < 1e-9 {
        return (peak, false);
    }
    // normalized peak energy share = spectral concentration
    let share = peak.abs() / sum;
    (peak, share >= concentration)
}

/// Floyd's cycle detection over a plan graph.
///
/// Reverse-engineered from the dossier (Floyd's Cycle, fast & slow pointers):
/// a CYCLIC DEPENDENCY in ACTIONS — the plan graph loops back (step i leads to
/// j leads to i) — is a loop the planner must refuse. `actions` is the
/// SUCCESSOR array: `actions[i]` = the next step from node `i`, or `n`
/// (`== nodes.len()`, a halt sentinel) for a terminal step. Two walkers (step-1
/// and step-2) meet inside the graph iff a cycle exists. Returns `Some(len)` or
/// `None`.
///
/// Fail-closed: a degenerate/empty plan returns `None` (not a cycle).
pub fn floyd_cycle(actions: &[usize], n: usize) -> Option<usize> {
    let m = actions.len();
    if m < 2 {
        return None;
    }
    let halt = n; // out-of-range pointer == halt
    let step = |i: usize| -> usize { actions.get(i).copied().unwrap_or(halt) };
    let mut slow = 0usize;
    let mut fast = step(0);
    let mut guard = 0;
    while slow != fast && guard < 2 * (m + n + 1) {
        slow = step(slow);
        fast = step(step(fast));
        guard += 1;
    }
    // met only at the halt sentinel ⇒ acyclic (ran off the graph)
    if slow != fast || slow >= halt {
        return None;
    }
    // measure cycle length
    let mut len = 1usize;
    let mut cur = step(slow);
    while cur != slow {
        len += 1;
        cur = step(cur);
    }
    Some(len)
}

/// Net outward activity (divergence) at a node, from the geometric vector field
/// of edge momenta. Each edge carries momentum proportional to its weight in
/// the direction (to − from). Approximated as the signed sum of weights of
/// outgoing minus incoming edges (a 0-D divergence / balance). Positive ⇒ source
/// (activity radiating out, potential runaway hub); negative ⇒ sink; ~0 ⇒
/// solenoidal (balanced, healthy).
pub fn field_divergence(node: usize, edges: &[ConnEdge]) -> f64 {
    let mut flux = 0.0f64;
    for e in edges {
        if e.from == node {
            flux += e.weight; // radiating out
        } else if e.to == node {
            flux -= e.weight; // flowing in
        }
    }
    flux
}

/// The unified probe verdict over the connection graph.
#[derive(Debug, PartialEq, Eq)]
pub enum WaveVerdict {
    Permit,    // graph is safe: no red-line cycle, no runaway, banded-ok
    Unhealthy, // fail-closed: a red-line action cycle OR a divergent hub was found
}

/// Compose the full geometric-wave probe into one falsifiable verdict.
///
/// `actions` is the ordered action chain the planner is about to run (Floyd
/// cycle detection — a loop of actions is refused). `red_line_action_cycle` is
/// precomputed by the caller (does the chain re-enter a red-line node?). If a
/// red-line cycle exists OR a node's divergence exceeds `hub_limit` (runaway
/// hub) OR the wave field is spectrally concentrated above `concentration`
/// (resonant notch = brittle coupling), the verdict is `Unhealthy` (fail-closed).
/// `Permit` only when all three checks pass.
pub fn wave_probe(
    nodes: &[Node2D],
    edges: &[ConnEdge],
    actions: &[usize],
    red_line_action_cycle: bool,
    hub_limit: f64,
    concentration: f64,
    seed: usize,
    t: f64,
    coeff: f64,
    min_coupling: f64,
) -> WaveVerdict {
    // 1) RED-LINE ACTION CYCLE → fail-closed (a loop touching secrets/auth/money)
    if red_line_action_cycle {
        return WaveVerdict::Unhealthy;
    }
    // 2) Floyd cycle on the plan successor graph (any cycle is a planner loop)
    if floyd_cycle(actions, nodes.len()).is_some() {
        return WaveVerdict::Unhealthy;
    }
    // 3) propagate the wave and inspect spectral concentration (notch)
    let field = propagate_wave(nodes, edges, seed, t, coeff, min_coupling);
    let (_peak, notch) = graph_fourier_notch(&field, concentration);
    if notch {
        return WaveVerdict::Unhealthy;
    }
    // 4) runaway hub: any node with net outward flux > hub_limit
    for ni in 0..nodes.len() {
        if field_divergence(ni, edges) > hub_limit {
            return WaveVerdict::Unhealthy;
        }
    }
    WaveVerdict::Permit
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_nodes() -> Vec<Node2D> {
        // geometry: spread nodes so distance coupling varies
        vec![
            Node2D {
                id: "mem".into(),
                x: 0.0,
                y: 0.0,
                red_line: false,
            },
            Node2D {
                id: "file".into(),
                x: 1.0,
                y: 0.0,
                red_line: false,
            },
            Node2D {
                id: "act".into(),
                x: 2.0,
                y: 1.0,
                red_line: false,
            },
            Node2D {
                id: "secret".into(),
                x: 0.0,
                y: 3.0,
                red_line: true,
            },
        ]
    }

    #[test]
    fn geometry_weights_closer_stronger() {
        // GREEN: geometric coupling falls with distance.
        let n = sample_nodes();
        let e = connection_edges_kinded(&n, &[(0, 1, LinkKind::Action), (0, 3, LinkKind::Action)]);
        let w_near = e.iter().find(|c| c.to == 1).unwrap().weight;
        let w_far = e.iter().find(|c| c.to == 3).unwrap().weight;
        assert!(w_near > w_far, "closer node must couple stronger");
    }

    #[test]
    fn action_kind_dominates_data() {
        // GREEN: an Action edge binds tighter than a Data edge at equal distance.
        let n = sample_nodes();
        let e = connection_edges_kinded(&n, &[(0, 1, LinkKind::Action), (0, 1, LinkKind::Data)]);
        let act = e
            .iter()
            .find(|c| c.kind == LinkKind::Action)
            .unwrap()
            .weight;
        let dat = e.iter().find(|c| c.kind == LinkKind::Data).unwrap().weight;
        assert!(act > dat, "action weight must exceed data weight");
    }

    #[test]
    fn floyd_finds_action_cycle() {
        // RED: a plan loop (0→1→0→halt) → cycle detected. n=3 nodes, sentinel=3.
        let cycle = [1usize, 0, 3]; // step0→1, step1→0, step2→halt(3)
        assert!(floyd_cycle(&cycle, 3).is_some(), "must detect the loop");
        // GREEN: an acyclic plan (0→1→2→halt) returns None.
        assert!(floyd_cycle(&[1usize, 2, 3], 3).is_none());
    }

    #[test]
    fn wave_probe_fails_closed_on_redline_cycle() {
        // RED: a red-line action cycle → Unhealthy (fail-closed, no RNG/clock).
        let n = sample_nodes();
        let cycle = [1usize, 3, 1]; // action chain re-enters the secret (red-line) node
        assert_eq!(
            wave_probe(&n, &[], &cycle, true, 10.0, 0.9, 0, 1.0, 0.5, 1e-3),
            WaveVerdict::Unhealthy
        );
        // RED: a runaway hub (huge divergence) → Unhealthy.
        let edges = connection_edges_kinded(
            &n,
            &[
                (0, 1, LinkKind::Action),
                (0, 2, LinkKind::Action),
                (0, 3, LinkKind::Action),
            ],
        );
        // node 0 radiates to all three → high divergence. hub_limit tiny.
        // acyclic plan (0→1→2→3→halt) so the only failure is the runaway hub.
        assert_eq!(
            wave_probe(
                &n,
                &edges,
                &[1, 2, 3, 4],
                false,
                0.5,
                0.99,
                0,
                1.0,
                0.5,
                1e-3
            ),
            WaveVerdict::Unhealthy
        );
        // GREEN: a small safe graph with no cycle, no hub, no resonance → Permit.
        let safe_edges = connection_edges_kinded(&n, &[(0, 1, LinkKind::Data)]);
        // acyclic plan (0→1→2→3→halt) + weak single data edge → Permit.
        assert_eq!(
            wave_probe(
                &n,
                &safe_edges,
                &[1, 2, 3, 4],
                false,
                50.0,
                0.999,
                0,
                1.0,
                0.5,
                1e-1
            ),
            WaveVerdict::Permit
        );
    }

    #[test]
    fn wave_propagation_is_deterministic() {
        // GREEN: same graph+seed → identical field (no hidden state).
        let n = sample_nodes();
        let e = connection_edges_kinded(&n, &[(0, 1, LinkKind::Action), (1, 2, LinkKind::Action)]);
        let a = propagate_wave(&n, &e, 0, 1.0, 0.5, 1e-3);
        let b = propagate_wave(&n, &e, 0, 1.0, 0.5, 1e-3);
        assert_eq!(a, b);
    }

    #[test]
    fn divergence_signals_source_vs_sink() {
        // RED+GREEN: outgoing-heavy node is a source (positive), incoming is sink.
        let n = sample_nodes();
        let e = connection_edges_kinded(&n, &[(0, 1, LinkKind::Action)]);
        assert!(field_divergence(0, &e) > 0.0, "node 0 is a source");
        assert!(field_divergence(1, &e) < 0.0, "node 1 is a sink");
    }
}
