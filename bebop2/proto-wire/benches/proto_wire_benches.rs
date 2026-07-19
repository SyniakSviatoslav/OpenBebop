//! proto_wire_benches.rs — P82 bebop bench expansion (§3.3-C3), `proto-wire` group.
//!
//! Criterion harness (`harness = false`), wired via `[[bench]]` in
//! `bebop2/proto-wire/Cargo.toml`. Bench ids follow the `<group>/<n>` convention
//! owned by dowiz P75 (P82 cites it) so the numbers land in the same baseline
//! schema as the rest of the P80/P81/P82 sweep.
//!
//! Groups:
//!   wire_encode_frame/<c>      — wire_codec::encode_frame, chain-swept
//!   wire_decode_frame/<c>      — wire_codec::decode_frame, chain-swept
//!   wire_framing_encode        — framing::encode (length-prefixed envelope)
//!   wire_framing_decode        — framing::decode (cursor-based)
//!   wire_envelope_serde_json   — Envelope serde_json cost (the outer envelope)
//!
//! BENCH-ONLY: no production codec / verify logic is changed. All inputs are
//! deterministic (seeded). The `chain-swept` dimension mirrors the proto-cap
//! `hybrid_gate_check` sweep: the frame carries a delegation chain of depth
//! {0,1,4,16} so the wire cost scales with chain length.

use bebop2_core::pq_dsa;
use bebop2_core::sign;
use bebop_proto_cap::roster::{AnchorRoster, Delegation, Effect};
use bebop_proto_cap::scope::{Action, Resource, Scope};
use bebop_proto_cap::signed_frame::SignedFrame;
use bebop_proto_cap::Capability;
use bebop_proto_wire::envelope::Envelope;
use bebop_proto_wire::framing;
use bebop_proto_wire::wire_codec;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

const EXPIRY: u64 = 1_000_000;
const NOW: u64 = 0;

fn ed_key(seed_byte: u8) -> ([u8; 32], [u8; 32]) {
    let seed = [seed_byte; 32];
    let (pk, _) = sign::keygen(&seed);
    (seed, pk)
}

/// Build a fully-signed hybrid frame carrying a delegation chain of depth `links`
/// (0 => anchor issues directly to itself; i.e. depth-0). Used by both the
/// encode/decode sweeps and the proto-cap gate sweep.
fn make_frame(links: usize, self_issue: bool) -> (SignedFrame, AnchorRoster) {
    let n = if self_issue { 1 } else { links + 1 };
    let keys: Vec<([u8; 32], [u8; 32])> = (0..n as u8).map(ed_key).collect();
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

    let mut chain: Vec<Delegation> = Vec::with_capacity(links);
    for i in 0..links {
        let link = Delegation::sign(
            keys[i].1,
            keys[i + 1].1,
            Scope::single(Resource::Route, Action::Send),
            Effect::single(Resource::Route, Action::Send),
            EXPIRY,
            [i as u8; 8],
            &keys[i].0,
        )
        .expect("sign link");
        chain.push(link);
    }
    frame.delegation_chain = chain;

    let mut roster = AnchorRoster::new();
    roster.enroll(&keys[0].1);
    (frame, roster)
}

fn bench_encode_frame(c: &mut Criterion) {
    let mut group = c.benchmark_group("wire_encode_frame");
    for &(links, self_issue) in &[(1usize, true), (1, false), (4, false), (16, false)] {
        let (frame, _roster) = make_frame(links, self_issue);
        let id = if self_issue { 0 } else { links };
        group.bench_with_input(BenchmarkId::from_parameter(id), &id, |b, _| {
            b.iter(|| black_box(wire_codec::encode_frame(&frame).unwrap()))
        });
    }
    group.finish();
}

fn bench_decode_frame(c: &mut Criterion) {
    let mut group = c.benchmark_group("wire_decode_frame");
    for &(links, self_issue) in &[(1usize, true), (1, false), (4, false), (16, false)] {
        let (frame, _roster) = make_frame(links, self_issue);
        let bytes = wire_codec::encode_frame(&frame).expect("encode");
        let id = if self_issue { 0 } else { links };
        group.bench_with_input(BenchmarkId::from_parameter(id), &id, |b, _| {
            b.iter(|| black_box(wire_codec::decode_frame(&bytes).unwrap()))
        });
    }
    group.finish();
}

fn bench_framing(c: &mut Criterion) {
    let (frame, _roster) = make_frame(1, false);
    let envelope = Envelope::new([0xABu8; 16], wire_codec::encode_frame(&frame).unwrap());
    let mut group = c.benchmark_group("wire_framing");
    group.bench_function("encode", |b| {
        b.iter(|| black_box(framing::encode(&envelope).unwrap()))
    });
    let buf = framing::encode(&envelope).unwrap();
    group.bench_function("decode", |b| {
        b.iter(|| {
            let mut b2 = buf.clone();
            black_box(framing::decode(&mut b2).unwrap().unwrap())
        })
    });
    group.finish();
}

fn bench_envelope_serde_json(c: &mut Criterion) {
    let (frame, _roster) = make_frame(1, false);
    let envelope = Envelope::new([0xABu8; 16], wire_codec::encode_frame(&frame).unwrap());
    let mut group = c.benchmark_group("wire_envelope");
    group.bench_function("serde_json_to_bytes", |b| {
        b.iter(|| black_box(envelope.to_bytes().unwrap()))
    });
    let bytes = envelope.to_bytes().unwrap();
    group.bench_function("serde_json_from_bytes", |b| {
        b.iter(|| black_box(Envelope::from_bytes(&bytes).unwrap()))
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_encode_frame,
    bench_decode_frame,
    bench_framing,
    bench_envelope_serde_json
);
criterion_main!(benches);
