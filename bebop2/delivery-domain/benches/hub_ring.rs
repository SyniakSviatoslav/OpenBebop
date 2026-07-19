//! P78 B4 — hub-ring HRW ranking benchmark.
//!
//! Bench-id convention `<group>/<n>` owned by P75 (cite, don't redefine):
//! group = `hub_ring`, `n` = hub-set size.
//!
//! Proves the win from the Schwartzian transform in `ranked` (precompute the
//! HRW weight once, sort tuples) and the `max_by` scan in `owner_hub` (no full
//! sort just to take `[0]`):
//!   - `hub_ring/ranked/<n>`  — full HRW ordering over n hubs.
//!   - `hub_ring/owner/<n>`   — just the owner hub.
//!
//! Run: `cargo bench -p bebop-delivery-domain --features kernel-rlib --bench hub_ring`
//! (the `hub_ring` module is gated behind `kernel-rlib`).
//! Behavior is unchanged — only the asymptotics.

#![cfg(feature = "kernel-rlib")]

use bebop_delivery_domain::hub_ring::{owner_hub, ranked, Hub};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

/// Hub-set sizes (bounded-small in practice, but called per-order).
const SIZES: &[usize] = &[4, 8, 16, 32];

fn hub_set(n: usize) -> Vec<Hub> {
    (0..n).map(|i| Hub::new([(i as u8).wrapping_mul(7); 32])).collect()
}

fn bench_ranked(c: &mut Criterion) {
    let mut group = c.benchmark_group("hub_ring");
    for &n in SIZES {
        let hubs = hub_set(n);
        let order = 0xABCDEFu64;
        group.bench_with_input(BenchmarkId::new("ranked", n), &n, |b, &_n| {
            b.iter(|| {
                black_box(ranked(order, &hubs));
            });
        });
    }
    group.finish();
}

fn bench_owner(c: &mut Criterion) {
    let mut group = c.benchmark_group("hub_ring");
    for &n in SIZES {
        let hubs = hub_set(n);
        let order = 0xABCDEFu64;
        group.bench_with_input(BenchmarkId::new("owner", n), &n, |b, &_n| {
            b.iter(|| {
                black_box(owner_hub(order, &hubs));
            });
        });
    }
    group.finish();
}

criterion_group!(benches, bench_ranked, bench_owner);
criterion_main!(benches);
