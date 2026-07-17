//! verify_lane.rs — BLUEPRINT-P-E §3.2 DoD benchmark (zero-dep, std::time::Instant).
//!
//! Compares scalar single-verify vs the Mode-1 lane-parallel `verify_many`
//! throughput for ML-DSA-65 (AVX2 Keccak×4 ExpandA lane) and Ed25519, at
//! N ∈ {1, 4, 16, 64}. NO new dependencies (keeps `[dependencies] # none.`).
//! Run with: `cargo bench --features test_keygen`.
//!
//! This is a measured before/after number, NOT a correctness gate (correctness
//! lives in the parity/adversarial unit tests, §2.6/§3.3). Perf assertions in unit
//! CI are flaky; this binary is run explicitly.

use bebop2_core::pq_dsa;
use bebop2_core::sign;
use std::time::Instant;

const NS: [usize; 4] = [1, 4, 16, 64];

fn bench<F: FnMut()>(label: &str, iters: u32, mut f: F) -> f64 {
    // Warmup.
    for _ in 0..3 {
        f();
    }
    let start = Instant::now();
    for _ in 0..iters {
        f();
    }
    let elapsed = start.elapsed().as_nanos() as f64;
    let per = elapsed / iters as f64;
    println!("  {label:<40} {per:>12.1} ns/call");
    per
}

fn main() {
    println!("== BLUEPRINT-P-E Mode 1 verify-lane bench ==");
    let avx2 = std::is_x86_feature_detected!("avx2");
    println!("AVX2 available: {avx2}\n");

    // ── ML-DSA-65 corpus ────────────────────────────────────────────────────
    let mldsa: Vec<(Vec<u8>, Vec<u8>, Vec<u8>)> = (0..64u16)
        .map(|i| {
            let seed = [(i as u8).wrapping_mul(7).wrapping_add(1); 32];
            let (pk, sk) = pq_dsa::keygen_bytes(&seed);
            let msg = vec![i as u8; 32];
            let rnd = [i as u8 ^ 0x5a; 32];
            let sig = pq_dsa::sign_internal_bytes(&sk, &msg, &rnd);
            (pk, msg, sig)
        })
        .collect();

    println!("ML-DSA-65:");
    for &n in &NS {
        let reqs: Vec<(&[u8], &[u8], &[u8])> = mldsa[..n]
            .iter()
            .map(|(pk, m, s)| (pk.as_slice(), m.as_slice(), s.as_slice()))
            .collect();
        let iters = if n <= 4 { 200 } else { 40 };
        let scalar = bench(&format!("scalar loop  N={n}"), iters, || {
            for (pk, m, s) in &reqs {
                std::hint::black_box(pq_dsa::verify_internal_bytes(pk, m, s));
            }
        }) / n as f64;
        let lane = bench(&format!("verify_many  N={n}"), iters, || {
            std::hint::black_box(pq_dsa::verify_internal_bytes_many(&reqs));
        }) / n as f64;
        println!("    -> per-verify scalar {scalar:.0}ns  lane {lane:.0}ns  speedup {:.2}x\n", scalar / lane);
    }

    // ── Ed25519 corpus ──────────────────────────────────────────────────────
    #[cfg(feature = "test_keygen")]
    {
        let ed: Vec<([u8; 32], Vec<u8>, [u8; 64])> = (0..64u16)
            .map(|i| {
                let seed = [(i as u8).wrapping_mul(11).wrapping_add(3); 32];
                let (pk, _sk) = sign::keygen(&seed);
                let msg = vec![i as u8; 32];
                let sig = sign::sign(&seed, &msg);
                (pk, msg, sig)
            })
            .collect();

        println!("Ed25519:");
        for &n in &NS {
            let reqs: Vec<sign::VerifyReq> = ed[..n]
                .iter()
                .map(|(pk, m, s)| sign::VerifyReq { pubkey: pk, msg: m, sig: s })
                .collect();
            let iters = if n <= 4 { 200 } else { 40 };
            let scalar = bench(&format!("scalar loop  N={n}"), iters, || {
                for r in &reqs {
                    std::hint::black_box(sign::verify(r.pubkey, r.msg, r.sig));
                }
            }) / n as f64;
            let lane = bench(&format!("verify_many  N={n}"), iters, || {
                std::hint::black_box(sign::verify_many(&reqs));
            }) / n as f64;
            println!("    -> per-verify scalar {scalar:.0}ns  lane {lane:.0}ns  speedup {:.2}x\n", scalar / lane);
        }
    }
    #[cfg(not(feature = "test_keygen"))]
    println!("Ed25519 bench skipped (enable --features test_keygen for keygen/sign).");
}
