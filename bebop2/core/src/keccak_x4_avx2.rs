//! keccak_x4_avx2 — AVX2 lane-parallel Keccak-f[1600] (BLUEPRINT-P-E §2.3, Mode 1).
//!
//! Four INDEPENDENT Keccak-f[1600] states are interleaved across the 4 lanes of a
//! `__m256i` (25 lanes × 4-way = 25 `__m256i`). This is the SHAKE/Keccak batch lane
//! that accelerates ML-DSA-65 matrix expansion (ExpandA: K×L = 30 independent
//! SHAKE128 streams). It is pure ARITHMETIC acceleration — lane `k` only ever touches
//! its own stream, no instruction combines two streams (blueprint §2.1 invariant).
//!
//! Every permutation is byte/element-EXACT with the scalar `pq_kem::keccak_f`
//! (parity-pinned, §2.6): AVX2 only reschedules exact integer XOR/rotate ops, so the
//! output cannot differ from scalar — only its speed. Compiled ONLY for
//! `std + x86_64`; every other target (no_std, wasm32, ARM) never sees this file, so
//! the empty-import wasm gate is structurally untouched.
#![cfg(all(feature = "std", target_arch = "x86_64"))]

use alloc::vec;
use alloc::vec::Vec;

#[cfg(target_arch = "x86_64")]
use core::arch::x86_64::*;

/// Number of independent Keccak-f[1600] streams interleaved in one AVX2 state.
pub const KECCAK_X4_LANES: usize = 4;

// FIPS-202 round constants (identical to pq_kem::RC — the scalar reference).
const RC: [u64; 24] = [
    0x0000000000000001,
    0x0000000000008082,
    0x800000000000808a,
    0x8000000080008000,
    0x000000000000808b,
    0x0000000080000001,
    0x8000000080008081,
    0x8000000000008009,
    0x000000000000008a,
    0x0000000000000088,
    0x0000000080008009,
    0x000000008000000a,
    0x000000008000808b,
    0x800000000000008b,
    0x8000000000008089,
    0x8000000000008003,
    0x8000000000008002,
    0x8000000000000080,
    0x000000000000800a,
    0x800000008000000a,
    0x8000000080008081,
    0x8000000000008080,
    0x0000000080000001,
    0x8000000080008008,
];
const RHO: [u32; 24] = [
    1, 3, 6, 10, 15, 21, 28, 36, 45, 55, 2, 14, 27, 41, 56, 8, 25, 43, 62, 18, 39, 61, 20, 44,
];

/// Runtime AVX2 detection (the kernel/src/simd.rs dispatch pattern, reused).
#[inline]
pub fn avx2_available() -> bool {
    std::is_x86_feature_detected!("avx2")
}

/// Rotate-left each 64-bit lane of `x` by the (runtime) count `n` in [1,63].
/// `_mm256_sll_epi64` / `_mm256_srl_epi64` take a shared xmm count applied to all
/// lanes — exactly what we need since all 4 states permute in lockstep.
#[inline]
#[target_feature(enable = "avx2")]
unsafe fn rotl256(x: __m256i, n: u32) -> __m256i {
    let l = _mm256_sll_epi64(x, _mm_cvtsi32_si128(n as i32));
    let r = _mm256_srl_epi64(x, _mm_cvtsi32_si128((64 - n) as i32));
    _mm256_or_si256(l, r)
}

/// AVX2 Keccak-f[1600] over four interleaved states. Mirrors `pq_kem::keccak_f`
/// step for step (theta / rho+pi / chi / iota); only the datum is `__m256i` (×4).
#[target_feature(enable = "avx2")]
unsafe fn keccakf_x4(st: &mut [__m256i; 25]) {
    for round in 0..24 {
        // Theta
        let mut bc = [_mm256_setzero_si256(); 5];
        for i in 0..5 {
            bc[i] = _mm256_xor_si256(
                _mm256_xor_si256(
                    _mm256_xor_si256(st[i], st[i + 5]),
                    _mm256_xor_si256(st[i + 10], st[i + 15]),
                ),
                st[i + 20],
            );
        }
        for i in 0..5 {
            let t = _mm256_xor_si256(bc[(i + 4) % 5], rotl256(bc[(i + 1) % 5], 1));
            for j in 0..5 {
                st[i + 5 * j] = _mm256_xor_si256(st[i + 5 * j], t);
            }
        }
        // Rho + Pi
        let mut x = 1usize;
        let mut y = 0usize;
        let mut current = st[1];
        for i in 0..24 {
            let ax = y;
            let ay = (2 * x + 3 * y) % 5;
            let idx = ax + 5 * ay;
            let tmp = st[idx];
            st[idx] = rotl256(current, RHO[i]);
            current = tmp;
            x = ax;
            y = ay;
        }
        // Chi
        for y in 0..5 {
            let mut t = [_mm256_setzero_si256(); 5];
            for x in 0..5 {
                t[x] = st[x + 5 * y];
            }
            for x in 0..5 {
                let not_next = _mm256_andnot_si256(t[(x + 1) % 5], t[(x + 2) % 5]);
                st[x + 5 * y] = _mm256_xor_si256(t[x], not_next);
            }
        }
        // Iota
        st[0] = _mm256_xor_si256(st[0], _mm256_set1_epi64x(RC[round] as i64));
    }
}

/// Four independent SHAKE streams, interleaved. Inputs MUST be equal length
/// (the ML-DSA ExpandA use always feeds 34-byte seeds); each `out[k]` is filled
/// independently. `rate` is 168 (SHAKE128) or 136 (SHAKE256); `pad` is 0x1f.
#[target_feature(enable = "avx2")]
unsafe fn shake_x4(rate: usize, pad: u8, inputs: &[&[u8]; 4], out_len: usize) -> [Vec<u8>; 4] {
    let in_len = inputs[0].len();
    debug_assert!(inputs.iter().all(|m| m.len() == in_len));

    let mut st = [_mm256_setzero_si256(); 25];
    // Absorb: each 8-byte lane word XORs the four streams' bytes into lane k.
    let mut off = 0usize;
    let mut buf = [[0u8; 200]; 4]; // per-stream padded block accumulator
    let mut pos = 0usize;
    let absorb_word = |st: &mut [__m256i; 25], b: &[[u8; 200]; 4], word: usize| {
        let mut lane = |s: usize| -> u64 {
            let base = word * 8;
            let mut v = 0u64;
            for t in 0..8 {
                v |= (b[s][base + t] as u64) << (8 * t);
            }
            v
        };
        let cur = st[word];
        let add = _mm256_set_epi64x(lane(3) as i64, lane(2) as i64, lane(1) as i64, lane(0) as i64);
        st[word] = _mm256_xor_si256(cur, add);
    };

    // Full-rate blocks.
    while off + rate <= in_len {
        for k in 0..4 {
            buf[k][..rate].copy_from_slice(&inputs[k][off..off + rate]);
        }
        for w in 0..(rate / 8) {
            absorb_word(&mut st, &buf, w);
        }
        keccakf_x4(&mut st);
        off += rate;
    }
    // Final partial block + pad.
    let rem = in_len - off;
    for k in 0..4 {
        for b in buf[k].iter_mut() {
            *b = 0;
        }
        buf[k][..rem].copy_from_slice(&inputs[k][off..]);
        buf[k][rem] = pad;
        buf[k][rate - 1] |= 0x80;
    }
    for w in 0..(rate / 8) {
        absorb_word(&mut st, &buf, w);
    }
    keccakf_x4(&mut st);

    // Squeeze.
    let mut outs = [vec![0u8; out_len], vec![0u8; out_len], vec![0u8; out_len], vec![0u8; out_len]];
    let mut lane_words = [[0u64; 25]; 4];
    let store = |st: &[__m256i; 25], lw: &mut [[u64; 25]; 4]| {
        let mut tmp = [0i64; 4];
        for i in 0..25 {
            _mm256_storeu_si256(tmp.as_mut_ptr() as *mut __m256i, st[i]);
            for k in 0..4 {
                lw[k][i] = tmp[k] as u64;
            }
        }
    };
    let mut produced = 0usize;
    let _ = pos;
    while produced < out_len {
        store(&st, &mut lane_words);
        let take = core::cmp::min(rate, out_len - produced);
        for k in 0..4 {
            let mut bpos = 0usize;
            while bpos < take {
                let lane = bpos / 8;
                let shift = (bpos % 8) * 8;
                outs[k][produced + bpos] = (lane_words[k][lane] >> shift) as u8;
                bpos += 1;
            }
        }
        produced += take;
        if produced < out_len {
            keccakf_x4(&mut st);
        }
    }
    outs
}

/// Safe entry: 4 independent SHAKE128 streams (rate 168, pad 0x1f). Caller must
/// have checked [`avx2_available`]; asserted here in debug builds.
pub fn shake128_x4(inputs: &[&[u8]; 4], out_len: usize) -> [Vec<u8>; 4] {
    debug_assert!(avx2_available());
    // SAFETY: gated by the avx2_available() precondition (kernel/src/simd.rs:166 pattern).
    unsafe { shake_x4(168, 0x1f, inputs, out_len) }
}

/// Safe entry: 4 independent SHAKE256 streams (rate 136, pad 0x1f).
pub fn shake256_x4(inputs: &[&[u8]; 4], out_len: usize) -> [Vec<u8>; 4] {
    debug_assert!(avx2_available());
    // SAFETY: gated by the avx2_available() precondition.
    unsafe { shake_x4(136, 0x1f, inputs, out_len) }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Parity layer 2 (§2.6): the x4 permutation is element-exact vs the scalar SHAKE.
    #[test]
    fn shake128_x4_matches_scalar() {
        if !avx2_available() {
            eprintln!("AVX2 not available on this host — x4 parity test vacuously skipped");
            return;
        }
        let seeds: [Vec<u8>; 4] = [
            (0..34u8).collect(),
            (10..44u8).collect(),
            (0..34u8).map(|b| b ^ 0xa5).collect(),
            vec![0xffu8; 34],
        ];
        let refs: [&[u8]; 4] = [&seeds[0], &seeds[1], &seeds[2], &seeds[3]];
        let out_len = 2016;
        let x4 = shake128_x4(&refs, out_len);
        for k in 0..4 {
            let mut scalar = vec![0u8; out_len];
            crate::pq_kem::shake128(&seeds[k], &mut scalar);
            assert_eq!(x4[k], scalar, "SHAKE128 x4 lane {k} diverged from scalar");
        }
    }

    #[test]
    fn shake256_x4_matches_scalar() {
        if !avx2_available() {
            return;
        }
        let seeds: [Vec<u8>; 4] = [
            vec![1u8; 66],
            vec![2u8; 66],
            (0..66u8).collect(),
            (0..66u8).rev().collect(),
        ];
        let refs: [&[u8]; 4] = [&seeds[0], &seeds[1], &seeds[2], &seeds[3]];
        let out_len = 640;
        let x4 = shake256_x4(&refs, out_len);
        for k in 0..4 {
            let mut scalar = vec![0u8; out_len];
            crate::pq_kem::shake256(&seeds[k], &mut scalar);
            assert_eq!(x4[k], scalar, "SHAKE256 x4 lane {k} diverged from scalar");
        }
    }
}
