//! proto_cap_benches.rs — P82 bebop bench expansion (§3.3-C3), `proto-cap` group.
//!
//! Criterion harness (`harness = false`), wired via `[[bench]]` in
//! `bebop2/proto-cap/Cargo.toml`. Bench ids follow the `<group>/<n>` convention
//! owned by dowiz P75 (P82 cites it) so the numbers land in the same baseline
//! schema as the rest of the P80/P81/P82 sweep.
//!
//! Groups:
//!   hybrid_gate_check/<c>   — THE per-frame auth gate, chain-swept {0,1,4,16}
//!                              (headline bench; feeds the §4/D-3 NTT decision data)
//!   hybrid_verify_pq        — SignedFrame::verify_pq (real ML-DSA-65)
//!   hybrid_verify_classical — SignedFrame::verify_classical (real Ed25519)
//!   tlv_signing_input       — building the canonical TLV signing domain
//!   roster_verify_chain/<c> — roster::verify_chain, chain-swept {0,1,4,16}
//!   matcher_assign/<n>      — matcher::assign over n candidates (HRW)
//!
//! BENCH-ONLY: no production crypto / verify logic is changed. All inputs are
//! deterministic (seeded). The KEM data lives in core/verify_lane.rs; this file
//! covers the authorization-line hot paths.

use bebop_proto_cap::matcher::{assign, Courier, Order};
use bebop_proto_cap::roster::{AnchorRoster, Delegation, Effect};
use bebop_proto_cap::scope::{Action, Resource, Scope};
use bebop_proto_cap::signed_frame::SignedFrame;
use bebop_proto_cap::{Capability, HybridGate, HybridPolicy, RevocationSet};
use bebop2_core::pq_dsa;
use bebop2_core::sign;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

const EXPIRY: u64 = 1_000_000;
const NOW: u64 = 0;

/// Deterministic seed -> (seed, Ed25519 pk). `sign::keygen` is reachable because
/// proto-cap enables `test_keygen` (=> ceremony), which is the sanctioned
/// constant-seed path for tests/benches.
fn ed_key(seed_byte: u8) -> ([u8; 32], [u8; 32]) {
    let seed = [seed_byte; 32];
    let (pk, _) = sign::keygen(&seed);
    (seed, pk)
}

/// Build a fully-valid hybrid `SignedFrame` carrying an anchor-rooted delegation
/// chain of `links` links, plus the enrolled roster.
///
/// `links` is the number of delegation links:
///   - `links == 1` with `self_issue == true` => the anchor issues the capability
///     to ITSELF (depth-0 / no intermediate delegation — the most common prod shape).
///     This is the `hybrid_gate_check/0` data point.
///   - `links > 0` => anchor -> ... -> leaf, with `links` links and `links`
///     intermediate+leaf keys; the leaf signs the frame (and carries a real
///     ML-DSA-65 `subject_key_pq` so `RequireBoth` verifies the PQ leg for real).
fn make_frame(links: usize, self_issue: bool) -> (SignedFrame, AnchorRoster) {
    let n = if self_issue { 1 } else { links + 1 };
    let keys: Vec<([u8; 32], [u8; 32])> = (0..n as u8).map(ed_key).collect();

    // Leaf (frame signer) + its real PQ keypair.
    let leaf_idx = n - 1;
    let leaf_seed = keys[leaf_idx].0;
    let leaf_pk = keys[leaf_idx].1;
    let pq_seed = [0xA3u8; 32];
    let (pq_pk, pq_sk) = pq_dsa::keygen(&pq_seed);

    let cap = Capability::new_hybrid(
        leaf_pk,
        pq_pk.bytes.clone(),
        Resource::Route,
        Action::Send,
        [links as u8; 8],
        EXPIRY,
    );
    let mut frame = SignedFrame::new(cap, b"bebop-per-frame-auth".to_vec());
    frame.sign_classical(&leaf_seed).expect("classical sign");
    frame
        .sign_pq(&pq_sk.bytes.clone().try_into().unwrap(), &[0u8; 32])
        .expect("pq sign");

    // Build the chain: link i issued_by keys[i].pk -> subject keys[i+1].pk.
    // self_issue (depth-0 anchor-issued frame) carries NO delegation links, so the
    // chain is empty and the anchor is both issuer and subject of the capability.
    let chain_len = if self_issue { 0 } else { links };
    let mut chain: Vec<Delegation> = Vec::with_capacity(chain_len);
    for i in 0..chain_len {
        let issued_by = keys[i].1;
        let subject = keys[i + 1].1;
        let link = Delegation::sign(
            issued_by,
            subject,
            Scope::single(Resource::Route, Action::Send),
            Effect::single(Resource::Route, Action::Send),
            EXPIRY,
            [i as u8; 8],
            &keys[i].0,
        )
        .expect("sign delegation link");
        chain.push(link);
    }
    frame.delegation_chain = chain;

    // Roster: enroll the anchor (chain root).
    let mut roster = AnchorRoster::new();
    roster.enroll(&keys[0].1);
    (frame, roster)
}

fn bench_hybrid_gate(c: &mut Criterion) {
    let mut group = c.benchmark_group("hybrid_gate_check");
    // chain-swept {0,1,4,16}. 0 = direct anchor-issued (depth-0, no intermediate);
    // 1/4/16 = anchor->...->leaf with that many delegation links. Explicit mapping
    // so the bench ids are exactly {0,1,4,16}.
    let cases: &[(usize, bool)] = &[(1, true), (1, false), (4, false), (16, false)];
    for &(links, self_issue) in cases {
        let (frame, roster) = make_frame(links, self_issue);
        let id = if self_issue { 0 } else { links };
        group.bench_with_input(BenchmarkId::from_parameter(id), &id, |b, _| {
            // Fresh gate per iter so the replay ledger (seen) never short-circuits
            // the measured path into a NonceRejected fast-path. Gate construction
            // is one empty Mutex<HashSet> alloc — negligible vs the PQ verify.
            b.iter(|| {
                let gate = HybridGate::new(HybridPolicy::RequireBoth);
                black_box(gate.check(
                    &frame,
                    &roster,
                    &frame.delegation_chain,
                    &RevocationSet::new(),
                    NOW,
                ))
            })
        });
    }
    group.finish();
}

fn bench_verify_legs(c: &mut Criterion) {
    let (frame, _roster) = make_frame(1, false);
    let mut group = c.benchmark_group("hybrid");
    group.bench_function("verify_pq", |b| {
        b.iter(|| black_box(frame.verify_pq()))
    });
    group.bench_function("verify_classical", |b| {
        b.iter(|| black_box(frame.verify_classical()))
    });
    group.finish();
}

fn bench_tlv(c: &mut Criterion) {
    let (frame, _roster) = make_frame(1, false);
    let mut group = c.benchmark_group("tlv");
    // The canonical signing input (calls tlv::tlv_signing_input).
    group.bench_function("signing_input", |b| {
        b.iter(|| black_box(frame.binding_signing_domain().unwrap()))
    });
    group.finish();
}

fn bench_roster_verify_chain(c: &mut Criterion) {
    let mut group = c.benchmark_group("roster_verify_chain");
    let cases: &[(usize, bool)] = &[(1, true), (1, false), (4, false), (16, false)];
    for &(links, self_issue) in cases {
        let (frame, roster) = make_frame(links, self_issue);
        let id = if self_issue { 0 } else { links };
        group.bench_with_input(BenchmarkId::from_parameter(id), &id, |b, _| {
            b.iter(|| {
                black_box(bebop_proto_cap::roster::verify_chain(
                    &roster,
                    &frame.delegation_chain,
                    &frame.capability,
                    NOW,
                ))
            })
        });
    }
    group.finish();
}

fn bench_matcher_assign(c: &mut Criterion) {
    let mut group = c.benchmark_group("matcher_assign");
    let order = Order {
        id: 4242,
        src: "R".into(),
        dst: "C".into(),
    };
    for &n in &[1usize, 4, 16, 64, 256] {
        let candidates: Vec<Courier> = (0..n as u8).map(|b| Courier { pubkey: [b; 32] }).collect();
        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, _| {
            b.iter(|| black_box(assign(&order, &candidates, candidates.len())))
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_hybrid_gate,
    bench_verify_legs,
    bench_tlv,
    bench_roster_verify_chain,
    bench_matcher_assign
);
criterion_main!(benches);
