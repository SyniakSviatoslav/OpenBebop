//! Benchmark: relational hop-distance vs Euclidean for the field gravity force.
//! No RNG (crate rule) — deterministic LCG + ring/chord graph.
//! Run: cargo run --release --example relational_vs_euclidean

use std::collections::VecDeque;
use std::time::Instant;

#[derive(Clone)]
struct B {
    x: f64,
    y: f64,
    m: f64,
}

// const-seeded LCG (mirrors crate launch anim determinism)
fn lcg(state: &mut u64) -> f64 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    ((*state >> 33) as f64) / (u64::MAX as f64)
}

fn build(n: usize, seed: u64) -> (Vec<B>, Vec<Vec<usize>>) {
    let mut s = seed;
    let mut bodies = Vec::with_capacity(n);
    for _ in 0..n {
        bodies.push(B {
            x: lcg(&mut s) * 10.0,
            y: lcg(&mut s) * 10.0,
            m: 1.0 + lcg(&mut s) * 4.0,
        });
    }
    // ring + chord (deterministic, connected)
    let mut adj = vec![Vec::new(); n];
    for i in 0..n {
        let j = (i + 1) % n;
        adj[i].push(j);
        adj[j].push(i);
        let k = (i + 3) % n;
        adj[i].push(k);
        adj[k].push(i);
    }
    (bodies, adj)
}

// Euclidean pairwise: O(N^2) with sqrt
fn euclidean_forces(b: &[B]) -> f64 {
    let n = b.len();
    let mut sum = 0.0;
    for i in 0..n {
        for j in (i + 1)..n {
            let dx = b[j].x - b[i].x;
            let dy = b[j].y - b[i].y;
            let d2 = dx * dx + dy * dy + 1e-6;
            let d = d2.sqrt();
            let f = b[i].m * b[j].m / d2; // inverse-square
            sum += f * (dx / d + dy / d);
        }
    }
    sum
}

// BFS all-pairs shortest hop distance (relational)
fn relational_dist(n: usize, adj: &[Vec<usize>]) -> Vec<Vec<usize>> {
    let mut dist = vec![vec![usize::MAX; n]; n];
    for s in 0..n {
        let mut q = VecDeque::new();
        dist[s][s] = 0;
        q.push_back(s);
        while let Some(u) = q.pop_front() {
            for &v in &adj[u] {
                if dist[s][v] == usize::MAX {
                    dist[s][v] = dist[s][u] + 1;
                    q.push_back(v);
                }
            }
        }
    }
    dist
}

// relational pairwise apply: O(N^2) lookup (topology cached)
fn relational_forces(b: &[B], d: &[Vec<usize>]) -> f64 {
    let n = b.len();
    let mut sum = 0.0;
    for i in 0..n {
        for j in (i + 1)..n {
            let hops = d[i][j].max(1) as f64;
            sum += b[i].m * b[j].m / (hops * hops);
        }
    }
    sum
}

fn main() {
    let iters = 200usize;
    // accumulators force the optimizer to keep the force loops alive
    let mut acc_euc = 0.0f64;
    let mut acc_rel = 0.0f64;
    println!("n\t| euclid/tick(µs)\t| bfs_precomp(µs)\t| rel/tick(µs)\t| rel_total(µs)");
    for n in [10usize, 50, 100, 200, 500, 1000] {
        let (bodies, adj) = build(n, 0xBEEF_u64 + n as u64);

        // warm + time euclidean
        let _ = euclidean_forces(&bodies);
        let t0 = Instant::now();
        for _ in 0..iters {
            acc_euc += euclidean_forces(&bodies);
        }
        let euc = t0.elapsed().as_micros() as f64 / iters as f64;

        // relational: precompute BFS
        let t1 = Instant::now();
        let d = relational_dist(n, &adj);
        let pre = t1.elapsed().as_micros() as f64;

        // warm + time relational apply (cached)
        let _ = relational_forces(&bodies, &d);
        let t2 = Instant::now();
        for _ in 0..iters {
            acc_rel += relational_forces(&bodies, &d);
        }
        let rel = t2.elapsed().as_micros() as f64 / iters as f64;
        let rel_total = pre + rel * iters as f64;

        println!(
            "{}\t| {:.2}\t\t| {:.2}\t\t| {:.2}\t\t| {:.0}",
            n, euc, pre, rel, rel_total
        );
    }
    println!(
        "\n(acc_euc={:.3} acc_rel={:.3} — sums kept so loops are not elided)",
        acc_euc, acc_rel
    );
    println!("Verdict: per-tick Euclidean needs NO precompute and no BFS;");
    println!("relational pays O(N*(N+E)) BFS up front (every tick if topology moves).");
}
