//! P78 B3 — Merkle digest ingest/root benchmark.
//!
//! Bench-id convention `<group>/<n>` owned by P75 (cite, don't redefine):
//! group = `merkle_root`, `n` = event-log / leaf-set size (event-log scale).
//!
//! Proves the win from deferring the per-insert `sort_unstable` to a single
//! sort in `root()`:
//!   - `merkle_root/ingest/<n>`  — fold n content-ids via `MerkleLog::add`.
//!   - `merkle_root/root/<n>`    — compute the Merkle root over n leaves.
//!
//! Run: `cargo bench -p bebop-proto-wire --bench merkle_root`
//! (criterion harness). Behavior is unchanged — only the asymptotics.

use bebop_proto_wire::sync_pull::MerkleLog;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

/// Event-log-scale leaf counts (matches real mesh/order fold volume).
const SIZES: &[usize] = &[256, 1024, 4096, 16384];

fn leaf(i: usize) -> [u8; 32] {
    let mut id = [0u8; 32];
    id[..8].copy_from_slice(&(i as u64).to_le_bytes());
    id
}

fn bench_ingest(c: &mut Criterion) {
    let mut group = c.benchmark_group("merkle_root");
    for &n in SIZES {
        group.bench_with_input(BenchmarkId::new("ingest", n), &n, |b, &n| {
            b.iter(|| {
                let mut log = MerkleLog::new();
                for i in 0..n {
                    log.add(leaf(i));
                }
                black_box(log.len());
            });
        });
    }
    group.finish();
}

fn bench_root(c: &mut Criterion) {
    let mut group = c.benchmark_group("merkle_root");
    for &n in SIZES {
        // Pre-build a log of n leaves once; bench only the root computation.
        let mut log = MerkleLog::new();
        for i in 0..n {
            log.add(leaf(i));
        }
        group.bench_with_input(BenchmarkId::new("root", n), &n, |b, &_n| {
            b.iter(|| {
                black_box(log.root());
            });
        });
    }
    group.finish();
}

criterion_group!(benches, bench_ingest, bench_root);
criterion_main!(benches);
