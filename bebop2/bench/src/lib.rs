//! B4 — crypto ground-truth bench support.
//!
//! Shared fixture builders + a zero-dependency `std::time::Instant` percentile
//! sampler, used by:
//!   * `bin/record-ledger` — produces the durable `docs/ledger/crypto-bench.jsonl`
//!     rows (mean/median/ci95/p99 — the p99 field criterion does not expose);
//!   * `benches/crypto.rs`  — the criterion statistical harness (DECART choice);
//!   * the companion tests below (replayed-frame RED, envelope-size).
//!
//! It depends on `bebop2-core` (with `test_keygen` for deterministic fixtures) and
//! `bebop-proto-cap`. It is a NEVER-SHIPPED bench crate: `core`/`proto-cap` stay
//! dep-free (M6 trust boundary untouched).

use bebop2_core::{pq_dsa, sign};
use bebop_proto_cap::capability::Capability;
use bebop_proto_cap::hybrid_gate::{HybridGate, HybridPolicy};
use bebop_proto_cap::revocation::RevocationSet;
use bebop_proto_cap::roster::{AnchorRoster, Delegation, Effect};
use bebop_proto_cap::scope::{Action, Resource, Scope};
use bebop_proto_cap::signed_frame::SignedFrame;

/// WorkReceipt-class payload → ~400 B frame signing domain (blueprint §2.1).
pub const PAYLOAD_BYTES: usize = 250;

/// Deterministic Ed25519 keypair from a single seed byte.
fn ed_key(seed_byte: u8) -> ([u8; 32], [u8; 32]) {
    let seed = [seed_byte; 32];
    let (pk, _) = sign::keygen(&seed);
    (seed, pk)
}

/// The deterministic ML-DSA-65 keypair used as the PQ half of the hybrid identity
/// (same `0xAB` seed the proto-cap tests use).
pub fn pq_keypair() -> (pq_dsa::MlDsa65Pk, pq_dsa::MlDsa65Sk) {
    pq_dsa::keygen_derivable(&[0xABu8; 32])
}

/// A fully-valid HYBRID frame with an anchor-rooted delegation chain of depth
/// `depth` (>= 1) and a distinct `nonce`. Classical + real ML-DSA-65 legs signed.
pub fn build_frame(
    depth: usize,
    payload_len: usize,
    nonce: [u8; 8],
) -> (SignedFrame, AnchorRoster, Vec<Delegation>) {
    assert!(depth >= 1, "chain depth must be >= 1");
    // Key ladder: index 0 = anchor, index `depth` = leaf (the frame subject).
    let mut seeds: Vec<[u8; 32]> = Vec::with_capacity(depth + 1);
    let mut pks: Vec<[u8; 32]> = Vec::with_capacity(depth + 1);
    for i in 0..=depth {
        let (s, p) = ed_key(0xE0u8.wrapping_add(i as u8));
        seeds.push(s);
        pks.push(p);
    }
    let leaf_seed = seeds[depth];
    let leaf_pk = pks[depth];

    let (pq_pk, pq_sk) = pq_keypair();
    let cap = Capability::new_hybrid(
        leaf_pk,
        pq_pk.bytes.clone(),
        Resource::Route,
        Action::Send,
        nonce,
        9999,
    );
    let mut f = SignedFrame::new(cap, vec![0x5Au8; payload_len]);
    f.sign_classical(&leaf_seed).expect("classical sign");
    let sk_arr: [u8; 4032] = pq_sk.bytes.clone().try_into().expect("ml-dsa sk length");
    f.sign_pq(&sk_arr, &[0u8; 32]).expect("pq sign");

    // Chain: link i is issued_by pks[i] -> subject pks[i+1], all Route::Send.
    let scope = Scope::single(Resource::Route, Action::Send);
    let mut chain: Vec<Delegation> = Vec::with_capacity(depth);
    for i in 0..depth {
        let link = Delegation::sign(
            pks[i],
            pks[i + 1],
            scope.clone(),
            Effect::single(Resource::Route, Action::Send),
            9999,
            nonce,
            &seeds[i],
        )
        .expect("delegation sign");
        chain.push(link);
    }
    f.delegation_chain = chain.clone();

    let mut roster = AnchorRoster::new();
    roster.enroll(&pks[0]);
    (f, roster, chain)
}

/// A revocation set of `n` distinct entries that do NOT collide with any real
/// fixture key (derived from a counter hash), so a valid frame still passes.
pub fn build_revocations(n: usize) -> RevocationSet {
    let mut revs = RevocationSet::new();
    for i in 0..n {
        revs.revoke_key(bebop2_core::hash::sha3_256(&(i as u64).to_le_bytes()));
    }
    revs
}

/// Ed25519-leg fixture over the REAL frame signing domain (~400 B): the classical
/// public key, the message, and the genuine Ed25519 signature.
pub fn ed25519_single_fixture() -> ([u8; 32], Vec<u8>, [u8; 64]) {
    let (f, _r, _c) = build_frame(1, PAYLOAD_BYTES, [1u8; 8]);
    let msg = f.binding_signing_domain().expect("binding domain");
    let pk = f.capability.subject_key;
    let sig: [u8; 64] = f
        .classical_sig
        .clone()
        .expect("classical sig")
        .try_into()
        .expect("ed25519 sig length");
    (pk, msg, sig)
}

/// ML-DSA-65-leg fixture over the same ~400 B frame domain: `(pk_bytes, msg, sig_bytes)`.
pub fn mldsa_frame_fixture() -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let (f, _r, _c) = build_frame(1, PAYLOAD_BYTES, [2u8; 8]);
    let msg = f.binding_signing_domain().expect("binding domain");
    let pk = f.capability.subject_key_pq.clone().expect("pq key");
    let sig = f.pq_sig.clone().expect("pq sig");
    (pk, msg, sig)
}

/// ML-DSA-65 fixture over a `msg_len`-byte message (the SHAKE `mu` cost scales
/// with message size — the batched-envelope class at ~3.4 KB).
pub fn mldsa_large_fixture(msg_len: usize) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let (pq_pk, pq_sk) = pq_keypair();
    let msg = vec![0xABu8; msg_len];
    let sig = pq_dsa::sign(&pq_sk, &msg, &[0u8; 32]);
    (pq_pk.bytes.clone(), msg, sig.bytes)
}

/// `n` independent, genuine Ed25519 `(pubkey, message, signature)` triples for the
/// batch-verify benches / consistency checks.
pub fn ed25519_batch_fixtures(n: usize) -> Vec<([u8; 32], Vec<u8>, [u8; 64])> {
    (0..n)
        .map(|i| {
            let mut seed = [0u8; 32];
            seed[0] = (i & 0xff) as u8;
            seed[1] = ((i >> 8) & 0xff) as u8;
            seed[2] = 0xB4;
            let (pk, _) = sign::keygen(&seed);
            let msg = format!("bebop2 batch work-receipt frame #{i}").into_bytes();
            let sig = sign::sign(&seed, &msg);
            (pk, msg, sig)
        })
        .collect()
}

/// Convenience: run the hybrid gate against a fresh gate (empty replay ledger) so
/// the nonce is always first-seen — the SUCCESS path. Panics if the fixture is not
/// on the success path (a guard against silently benching an error path).
pub fn gate_check_once(
    frame: &SignedFrame,
    roster: &AnchorRoster,
    chain: &[Delegation],
    revs: &RevocationSet,
) {
    let gate = HybridGate::new(HybridPolicy::RequireBoth);
    gate.check(frame, roster, chain, revs, 0)
        .expect("hybrid gate bench must run on the success path");
}

// ─────────────────────────────────────────────────────────────────────────────
// Zero-dep Instant percentile sampler + stats + run_key
// ─────────────────────────────────────────────────────────────────────────────

/// Time-bounded warm-up, then collect per-call latencies (ns) until `measure_s`
/// elapses or `cap` samples are gathered. `f` should `black_box` its result.
pub fn sample<F: FnMut()>(mut f: F, warmup_s: f64, measure_s: f64, cap: usize) -> Vec<u64> {
    use std::time::{Duration, Instant};
    let warm_dur = Duration::from_secs_f64(warmup_s);
    let warm = Instant::now();
    while warm.elapsed() < warm_dur {
        f();
    }
    let measure_dur = Duration::from_secs_f64(measure_s);
    let mut out: Vec<u64> = Vec::with_capacity(cap.min(1 << 16));
    let start = Instant::now();
    while out.len() < cap && start.elapsed() < measure_dur {
        let t = Instant::now();
        f();
        out.push(t.elapsed().as_nanos() as u64);
    }
    out
}

/// Percentile / dispersion summary of a raw ns sample vector.
#[derive(Debug, Clone)]
pub struct Stats {
    pub mean_ns: f64,
    pub median_ns: u64,
    pub ci95_low_ns: f64,
    pub ci95_high_ns: f64,
    pub p99_ns: u64,
    pub min_ns: u64,
    pub samples: usize,
}

/// Compute `Stats` (CI95 of the mean via the normal-approx standard error).
pub fn stats_from(raw: &[u64]) -> Stats {
    assert!(!raw.is_empty(), "no samples collected");
    let n = raw.len();
    let mut v = raw.to_vec();
    v.sort_unstable();
    let sum: u128 = v.iter().map(|&x| x as u128).sum();
    let mean = sum as f64 / n as f64;
    let median = v[n / 2];
    let p99_idx = (((n as f64) * 0.99) as usize).min(n - 1);
    let p99 = v[p99_idx];
    let var = v.iter().map(|&x| (x as f64 - mean).powi(2)).sum::<f64>() / n as f64;
    let se = var.sqrt() / (n as f64).sqrt();
    Stats {
        mean_ns: mean,
        median_ns: median,
        ci95_low_ns: mean - 1.96 * se,
        ci95_high_ns: mean + 1.96 * se,
        p99_ns: p99,
        min_ns: v[0],
        samples: n,
    }
}

/// The blueprint's `run_key` uniqueness scheme:
/// `sha3_256(bench_id ‖ commit_bebop ‖ commit_dowiz ‖ host ‖ cpu ‖ msg_bytes ‖
///  chain_depth ‖ samples ‖ warmup_s ‖ measure_s)` truncated to 16 hex chars.
/// `samples` here is the CONFIGURED cap (stable across runs), not the actual count.
#[allow(clippy::too_many_arguments)]
pub fn run_key(
    bench_id: &str,
    commit_bebop: &str,
    commit_dowiz: &str,
    host: &str,
    cpu: &str,
    msg_bytes: usize,
    chain_depth: i64,
    samples_cfg: usize,
    warmup_s: f64,
    measure_s: f64,
) -> String {
    let mut buf: Vec<u8> = Vec::new();
    for part in [bench_id, commit_bebop, commit_dowiz, host, cpu] {
        buf.extend_from_slice(part.as_bytes());
        buf.push(0x1f); // unit separator — unambiguous field boundary
    }
    for n in [msg_bytes as i64, chain_depth, samples_cfg as i64] {
        buf.extend_from_slice(n.to_string().as_bytes());
        buf.push(0x1f);
    }
    for x in [warmup_s, measure_s] {
        buf.extend_from_slice(format!("{x:.3}").as_bytes());
        buf.push(0x1f);
    }
    let h = bebop2_core::hash::sha3_256(&buf);
    let mut s = String::with_capacity(16);
    for b in &h[..8] {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use bebop_proto_cap::error::CapError;

    // Acceptance criterion 2 (companion RED test): the gate bench provably runs on
    // the SUCCESS path — a REPLAYED frame (same gate, same nonce) returns
    // NonceRejected. This is exactly why the bench uses a fresh gate per call (or
    // distinct-nonce frames): a naive `iter()` over one gate would bench the error
    // path from the second iteration on.
    #[test]
    fn replayed_frame_returns_nonce_rejected() {
        let (f, roster, chain) = build_frame(1, PAYLOAD_BYTES, [7u8; 8]);
        let revs = RevocationSet::new();
        let gate = HybridGate::new(HybridPolicy::RequireBoth);
        assert!(
            gate.check(&f, &roster, &chain, &revs, 0).is_ok(),
            "first sight of the nonce must succeed"
        );
        assert!(
            matches!(
                gate.check(&f, &roster, &chain, &revs, 0),
                Err(CapError::NonceRejected)
            ),
            "a replayed frame on the same gate must be NonceRejected"
        );
    }

    // A fresh gate per call keeps the nonce first-seen => success path.
    #[test]
    fn fresh_gate_per_call_is_success_path() {
        let (f, roster, chain) = build_frame(3, PAYLOAD_BYTES, [8u8; 8]);
        let revs = build_revocations(10_000);
        for _ in 0..5 {
            gate_check_once(&f, &roster, &chain, &revs); // panics if not Ok
        }
    }

    // Acceptance criterion 4 / task item 5: envelope-size arithmetic, recomputed
    // from source constants (not quoted). SIGNATUREBYTES = 3309 (pq_dsa.rs:64),
    // Ed25519 = 64 -> raw = 3373; + TLV field framing (FID(1)+u32_le len(4) = 5 B
    // per field x 2) = 3383 B ≈ 3.3 KiB. The PQ public key is 1952 bytes; shipping
    // it per frame would add 1952 B — the pin is key-by-reference via the 32-byte
    // pq_key_id, never re-shipping the key.
    #[test]
    fn envelope_tax_matches_recomputation() {
        assert_eq!(pq_dsa::SIGNATUREBYTES, 3309, "ML-DSA-65 sig bytes");
        assert_eq!(pq_dsa::PUBLICKEYBYTES, 1952, "ML-DSA-65 pubkey bytes");
        let ed25519_sig = 64usize;
        let raw = pq_dsa::SIGNATUREBYTES + ed25519_sig;
        assert_eq!(raw, 3373, "raw hybrid signature bytes");
        let tlv_framing = 2 * (1 + 4); // two fields, FID(1) + u32_le len(4)
        let framed = raw + tlv_framing;
        assert_eq!(framed, 3383, "framed hybrid signature tax ≈ 3.3 KiB");
        // If the PQ pubkey shipped per frame, the delta would balloon:
        let with_key_reshipped = framed + pq_dsa::PUBLICKEYBYTES;
        assert_eq!(with_key_reshipped, 5335, "per-frame key re-ship (rejected)");
        // Key-by-reference id is 32 bytes — this is what a frame carries instead.
        let id = bebop_proto_cap::revocation::pq_key_id(&vec![0u8; pq_dsa::PUBLICKEYBYTES]);
        assert_eq!(id.len(), 32, "pq_key_id is a 32-byte reference");
    }

    #[test]
    fn stats_and_run_key_are_sane() {
        let raw = vec![100u64, 200, 300, 400, 500];
        let s = stats_from(&raw);
        assert_eq!(s.median_ns, 300);
        assert_eq!(s.min_ns, 100);
        assert!((s.mean_ns - 300.0).abs() < 1e-9);
        let a = run_key("x", "c1", "c2", "h", "cpu", 400, 1, 100, 3.0, 5.0);
        let b = run_key("x", "c1", "c2", "h", "cpu", 400, 1, 100, 3.0, 5.0);
        assert_eq!(a, b, "run_key deterministic");
        assert_eq!(a.len(), 16, "run_key is 16 hex chars");
        let c = run_key("y", "c1", "c2", "h", "cpu", 400, 1, 100, 3.0, 5.0);
        assert_ne!(a, c, "different bench_id => different run_key");
    }
}
