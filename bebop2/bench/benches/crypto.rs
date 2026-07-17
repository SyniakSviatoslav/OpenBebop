//! B4 — criterion statistical harness (the DECART-chosen bench harness).
//!
//! Provides the defensible warm-up/outlier/CI statistics for the same five §2.1
//! benches + the batch legs. The durable ledger row (p99, run_key) is produced
//! separately by `bin/record-ledger` (criterion does not expose p99). Config
//! matches the blueprint: warm-up 3 s, measurement 5 s, sample-size 100.
//!
//! Run:  cargo bench --offline -p bebop2-bench

use std::hint::black_box;
use std::time::Duration;

use bebop2_bench as b;
use bebop2_core::{hash, pq_dsa, sign};
use bebop_proto_cap::revocation::RevocationSet;
use criterion::{criterion_group, criterion_main, BatchSize, Criterion};

fn crypto_benches(c: &mut Criterion) {
    let mut group = c.benchmark_group("b4-crypto");
    group
        .warm_up_time(Duration::from_secs(3))
        .measurement_time(Duration::from_secs(5))
        .sample_size(100);

    // Ed25519 single leg (real ~400 B frame domain).
    let (ed_pk, ed_msg, ed_sig) = b::ed25519_single_fixture();
    group.bench_function("ed25519_verify_single", |bch| {
        bch.iter(|| black_box(sign::verify(black_box(&ed_pk), black_box(&ed_msg), black_box(&ed_sig))))
    });

    // ML-DSA-65 single leg (same ~400 B message).
    let (mpk_b, mmsg, msig_b) = b::mldsa_frame_fixture();
    let mpk = pq_dsa::MlDsa65Pk { bytes: mpk_b };
    let msig = pq_dsa::MlDsa65Sig { bytes: msig_b };
    group.bench_function("mldsa65_verify_single", |bch| {
        bch.iter(|| black_box(pq_dsa::verify(black_box(&mpk), black_box(&mmsg), black_box(&msig))))
    });

    // ML-DSA-65 at ~3.4 KB (SHAKE mu scales with message).
    let (lpk_b, lmsg, lsig_b) = b::mldsa_large_fixture(3400);
    let lpk = pq_dsa::MlDsa65Pk { bytes: lpk_b };
    let lsig = pq_dsa::MlDsa65Sig { bytes: lsig_b };
    group.bench_function("mldsa65_verify_single_3400", |bch| {
        bch.iter(|| black_box(pq_dsa::verify(black_box(&lpk), black_box(&lmsg), black_box(&lsig))))
    });

    // Full hybrid gate, depth 1, empty revocation set. iter_batched hands a fresh
    // gate per iteration (empty replay ledger => the nonce is always first-seen =>
    // the SUCCESS path; see the companion `replayed_frame_returns_nonce_rejected`).
    let (f1, roster1, chain1) = b::build_frame(1, b::PAYLOAD_BYTES, [11u8; 8]);
    let revs_empty = RevocationSet::new();
    group.bench_function("hybrid_gate_check/d1", |bch| {
        bch.iter_batched(
            || bebop_proto_cap::hybrid_gate::HybridGate::new(
                bebop_proto_cap::hybrid_gate::HybridPolicy::RequireBoth,
            ),
            |gate| black_box(gate.check(&f1, &roster1, &chain1, &revs_empty, 0).is_ok()),
            BatchSize::SmallInput,
        )
    });

    // Full hybrid gate, depth 3, 10k-entry revocation set.
    let (f3, roster3, chain3) = b::build_frame(3, b::PAYLOAD_BYTES, [13u8; 8]);
    let revs_10k = b::build_revocations(10_000);
    group.bench_function("hybrid_gate_check/d3_rev10k", |bch| {
        bch.iter_batched(
            || bebop_proto_cap::hybrid_gate::HybridGate::new(
                bebop_proto_cap::hybrid_gate::HybridPolicy::RequireBoth,
            ),
            |gate| black_box(gate.check(&f3, &roster3, &chain3, &revs_10k, 0).is_ok()),
            BatchSize::SmallInput,
        )
    });

    // SHA3-256 sanity anchor (~µs, per R4 §4).
    let sha_input = vec![0xC3u8; 1024];
    group.bench_function("sha3_256_1kib", |bch| {
        bch.iter(|| black_box(hash::sha3_256(black_box(&sha_input))))
    });

    // Batch verify (8 and 64) vs N singles.
    let batch = b::ed25519_batch_fixtures(64);
    let view = |n: usize| -> Vec<(&[u8; 32], &[u8], &[u8; 64])> {
        batch[..n].iter().map(|(pk, m, s)| (pk, m.as_slice(), s)).collect()
    };
    for &n in &[8usize, 64] {
        let v = view(n);
        group.bench_function(format!("ed25519_verify_batch/{n}"), |bch| {
            bch.iter(|| black_box(sign::verify_batch(black_box(&v))))
        });
    }

    group.finish();
}

criterion_group!(benches, crypto_benches);
criterion_main!(benches);
