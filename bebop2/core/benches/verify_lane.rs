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
//!
//! ── P82 expansion (bebop bench coverage, §3.3-C3) ──────────────────────────────
//! This file is extended (NOT replaced) to cover the remaining unbenched crypto
//! lanes of the sovereign stack. Every new lane follows the `<group>/<n>` bench-id
//! convention owned by dowiz P75 (P82 cites it) so the numbers land in the same
//! baseline schema as the rest of the P80/P81/P82 sweep. All new lanes are
//! ZERO-dep (core stays air-gapped) and BENCH-ONLY — no production crypto logic is
//! touched.
//!
//! Added lanes:
//!   verify_lane/sign_mldsa65        — the SIGN path (was setup-only before)
//!   verify_lane/sign_ed25519        — classical sign (cfg test_keygen)
//!   verify_lane/kem_encaps          — ML-KEM-768 encaps (gates the future NTT decision, D-9)
//!   verify_lane/kem_decaps          — ML-KEM-768 decaps
//!   verify_lane/x25519_encaps       — sovereign X25519 (M2 classical fallback)
//!   verify_lane/x25519_decaps       — sovereign X25519 decap
//!   verify_lane/aead/<n>            — size-swept XChaCha20-Poly1305
//!   verify_lane/sha3_256/<n>        — size-swept SHA3-256
//!   verify_lane/sha3_512/<n>        — size-swept SHA3-512
//!
//! NOTE: the KEM sign/decaps numbers are RECORDED here to feed the operator's
//! §4/D-3 NTT decision. This bench takes NO position on whether NTT should be
//! reintroduced — it only produces the data.

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

    // ── P82: SIGN timing (was setup-only before) ───────────────────────────────
    println!("SIGN paths (previously setup-only — now benched):");
    {
        // ML-DSA-65 sign (ungated: sign_internal_bytes is always available).
        let seed = [0x11u8; 32];
        let (_pk, sk) = pq_dsa::keygen_bytes(&seed);
        let msg = vec![0xabu8; 32];
        let rnd = [0u8; 32];
        bench("verify_lane/sign_mldsa65", 20, || {
            std::hint::black_box(pq_dsa::sign_internal_bytes(&sk, &msg, &rnd));
        });
    }
    #[cfg(feature = "test_keygen")]
    {
        // Ed25519 sign (gated like the verify path).
        let seed = [0x22u8; 32];
        let msg = vec![0xcd; 32];
        bench("verify_lane/sign_ed25519", 200, || {
            std::hint::black_box(sign::sign(&seed, &msg));
        });
    }

    // ── P82: KEM encaps/decaps (ML-KEM-768) — gates the future NTT decision (D-9) ─
    // Data only: this bench records encaps/decaps cost and takes NO position on
    // whether NTT should be reintroduced.
    println!("\nML-KEM-768 (encaps/decaps — NTT-decision data, no decision made):");
    {
        // Deterministic RNG (no real entropy; benches are seedable by contract).
        let mut s: u64 = 0x1234_5678_9abc_def0;
        let mut rng = |buf: &mut [u8]| {
            for b in buf.iter_mut() {
                s ^= s << 13;
                s ^= s >> 7;
                s ^= s << 17;
                *b = (s & 0xff) as u8;
            }
        };
        let (ek, dk) = bebop2_core::pq_kem::keygen(&mut rng);
        let m = [0x77u8; 32];
        let (ss_enc, ct) = bebop2_core::pq_kem::encaps_internal(&ek, &m);
        let ss_dec = bebop2_core::pq_kem::decaps(&dk, &ct);
        debug_assert_eq!(ss_enc, ss_dec, "KEM roundtrip must agree");
        bench("verify_lane/kem_encaps", 40, || {
            std::hint::black_box(bebop2_core::pq_kem::encaps_internal(&ek, &m));
        });
        bench("verify_lane/kem_decaps", 40, || {
            std::hint::black_box(bebop2_core::pq_kem::decaps(&dk, &ct));
        });
    }

    // ── P82: sovereign X25519 (M2 classical-fallback KEM leg) ─────────────────────
    println!("\nSovereign X25519 (M2 classical fallback):");
    {
        let (pk, sk) = bebop2_core::x25519::keygen(&[0x33u8; 32]);
        let eph = [0x44u8; 32];
        let (shared, ct) = bebop2_core::x25519::encaps(&pk, &eph);
        let shared2 = bebop2_core::x25519::decaps(&sk, &ct);
        debug_assert_eq!(shared, shared2, "X25519 roundtrip must agree");
        bench("verify_lane/x25519_encaps", 200, || {
            std::hint::black_box(bebop2_core::x25519::encaps(&pk, &eph));
        });
        bench("verify_lane/x25519_decaps", 200, || {
            std::hint::black_box(bebop2_core::x25519::decaps(&sk, &ct));
        });
    }

    // ── P82: size-swept AEAD (XChaCha20-Poly1305, RFC 8439) ───────────────────────
    println!("\nAEAD XChaCha20-Poly1305 (size-swept):");
    {
        let key = [0x55u8; 32];
        let nonce = [0x66u8; 24];
        let aad = b"bebop-frame";
        for &n in &[32usize, 256, 1024, 4096, 16384] {
            let pt = vec![0x42u8; n];
            bench(&format!("verify_lane/aead/{n}"), 100, || {
                std::hint::black_box(
                    bebop2_core::aead::aead_xchacha20_poly1305_encrypt(&key, &nonce, &pt, aad),
                );
            });
        }
    }

    // ── P82: size-swept SHA3 (256 / 512) ─────────────────────────────────────────
    println!("\nSHA3 (size-swept):");
    for &n in &[32usize, 256, 1024, 4096, 16384] {
        let data = vec![0x99u8; n];
        bench(&format!("verify_lane/sha3_256/{n}"), 200, || {
            std::hint::black_box(bebop2_core::hash::sha3_256(&data));
        });
        bench(&format!("verify_lane/sha3_512/{n}"), 200, || {
            std::hint::black_box(bebop2_core::hash::sha3_512(&data));
        });
    }

    println!("\n== done ==");
}
