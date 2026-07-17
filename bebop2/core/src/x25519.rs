//! x25519 — X25519 (RFC 7748) elliptic-curve Diffie–Hellman, implemented from scratch,
//! zero external crates, pure `core` (no `std`/`alloc` on the crypto path → empty-import
//! wasm32 build). This is the **classical KEM leg** of M2's "ML-KEM-768 + classical-fallback"
//! hybrid (BLUEPRINT-P03 §3.6). `core::pq_kem` is the PQ leg; this module is the classical one.
//!
//! Why from-scratch (not `x25519-dalek`/`curve25519-dalek`): M6 forbids external crates at the
//! wire boundary, so the `dowiz-pq`/`vault.rs` dalek impls are references for the *math only*.
//! We mirror the zero-dep posture of `core/src/sign.rs` (Ed25519) and `core/src/pq_kem.rs`.
//!
//! Curve: Montgomery Curve25519, `y² = x³ + 486662·x² + x` over the field GF(p), p = 2²⁵⁵−19.
//! The shared secret is the u-coordinate of `k·u` (Montgomery ladder), RFC 7748 §5 exactly.
//!
//! CONSTANT-TIME DISCIPLINE (mirrors the C4b pattern in `sign.rs`):
//!   * The Montgomery ladder runs a FIXED 255 iterations — the operation trace does NOT depend
//!     on the secret scalar bits (no early-out, no secret-bit branch).
//!   * The per-iteration conditional swap is branch-free: `cswap_fe` xors limbs under a mask
//!     `0xFF`/`0x00` derived by `0u8.wrapping_sub(bit)` (bit ∈ {0,1}), exactly like
//!     `sign.rs::fe_cselect`. No data-dependent control flow anywhere on the secret path.
//!   * Field add/sub are implemented as full-width carry/borrow with a final constant-time
//!     conditional subtract of p; field multiply reduces via the 2²⁵⁵ ≡ 19 mod p fold — neither
//!     branch on secret data.
//!
//! INTENTIONAL CEILINGS (documented, not defects):
//!   * Schoolbook 256×256 multiplication (O(n²) limb products) is chosen over NTT/Karatsuba.
//!     X25519 is fixed 255-bit; the constant-time simplicity ceiling beats micro-optimization,
//!     and there is no secret-data leak from a fixed multiply schedule.
//!   * The ladder ceiling is the RFC-mandated 254→0 bit range; bit 255 is dropped by clamping
//!     (`k[31] &= 127`), so the scalar is always a multiple of 8 with the high bit cleared —
//!     this is the RFC 7748 §5 clamp, not an optimization we invented.
//!   * `fe_invert` uses square-and-multiply over the fixed 255-bit exponent p−2 (clearly correct,
//!     not the hand-rolled 254-multiply ref10 window). Performance is irrelevant for a KEM key
//!     agreement that runs a handful of times per session.

// ─────────────────────────────────────────────────────────────────────────────
// Field GF(p), p = 2²⁵⁵ − 19, represented as a 256-bit little-endian integer in
// eight u32 limbs. All values are kept in [0, p) (canonical residue) by every op.
// ─────────────────────────────────────────────────────────────────────────────

/// p = 2²⁵⁵ − 19, little-endian as eight u32 limbs.
/// p mod 2³² = −19 mod 2³² = 0xFFFFFFED; p >> 224 = 2³¹ − 1 = 0x7FFFFFFF (top limb).
const P: [u32; 8] = [
    0xFFFFFFED, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0x7FFFFFFF,
];

/// Field element: eight little-endian u32 limbs, value in [0, p).
type Fe = [u32; 8];

const FE_ZERO: Fe = [0; 8];
const FE_ONE: Fe = [1, 0, 0, 0, 0, 0, 0, 0];
/// 121665 = (486662 − 2)/4 = a24, the Montgomery-ladder constant (RFC 7748 §5).
const FE_A24: Fe = [121665, 0, 0, 0, 0, 0, 0, 0];

/// Exponent for inversion: p − 2 = 2²⁵⁵ − 21, little-endian as eight u32 limbs.
/// 2²⁵⁵ is below 2²⁵⁶, so bit 255 is CLEAR: limb7 = 0x7FFFFFFF. Only bit 2 of the
/// low limb is clear (21 = 16 + 4 + 1).
const EXP_P_MINUS_2: [u32; 8] = [
    0xFFFFFFEB, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0x7FFFFFFF,
];

#[inline]
fn load_le32(b: &[u8]) -> u32 {
    (b[0] as u32) | ((b[1] as u32) << 8) | ((b[2] as u32) << 16) | ((b[3] as u32) << 24)
}

#[inline]
fn store_le32(b: &mut [u8], v: u32) {
    b[0] = v as u8;
    b[1] = (v >> 8) as u8;
    b[2] = (v >> 16) as u8;
    b[3] = (v >> 24) as u8;
}

/// Constant-time `r = r - P` iff `r >= P`, else leave `r` unchanged.
/// `sub` is computed unconditionally into a temp, then each limb is selected
/// between `r` and `sub` via the full-width mask `m = ct_ge_mask(r, P)`.
#[inline]
fn ct_sub_if_ge(r: &mut Fe) {
    let m = ct_ge_mask(r, &P);
    let mut sub = *r;
    sub_inplace(&mut sub, &P);
    let nm = !m;
    for i in 0..8 {
        r[i] = (sub[i] & m) | (r[i] & nm);
    }
}

/// Constant-time `a >= b` as a 256-bit big integer: returns `0xFFFFFFFF` if true,
/// `0` if false. No early return — accumulates a mask across all limbs so timing
/// carries no information about where (or whether) the values first differ.
#[inline]
fn ct_ge_mask(a: &Fe, b: &Fe) -> u32 {
    let mut eq = 0xFFFFFFFFu32; // still-equal so far
    let mut gt = 0u32;          // some limb of a already exceeded b
    for i in (0..8).rev() {
        let a_gt = ((a[i] > b[i]) as u32).wrapping_neg(); // 0xFFFFFFFF if a[i]>b[i], else 0
        let a_lt = (a[i] < b[i]) as u32;
        // If we haven't already diverged, this limb decides: set gt mask if a[i]>b[i].
        gt |= eq & a_gt;
        // Once any limb differs, eq collapses to 0 (no later limb can re-open equality).
        eq &= (a_gt | a_lt).wrapping_sub(1);
    }
    // a >= b iff (some limb exceeded) OR (all limbs equal).
    gt | eq
}

/// `a -= b`, assuming `a >= b` (no borrow past the top limb). Branch-free:
/// the borrow is computed as a full-width mask, not a data-dependent `if`.
#[inline]
fn sub_inplace(a: &mut Fe, b: &Fe) {
    let mut borrow: i64 = 0;
    for i in 0..8 {
        let s = (a[i] as i64) - (b[i] as i64) - borrow;
        let mask = (s >> 63) as u32; // 0xFFFFFFFF if s < 0, else 0
        a[i] = (s + (mask as i64 & 0x1_0000_0000)) as u32;
        borrow = (mask & 1) as i64;
    }
}

/// Field addition, reduced mod p. Inputs in [0, p) ⇒ sum < 2p < 2²⁵⁶, fits 8 limbs.
#[inline]
pub(crate) fn fe_add(a: &Fe, b: &Fe) -> Fe {
    let mut r = [0u32; 8];
    let mut carry: u64 = 0;
    for i in 0..8 {
        let s = (a[i] as u64) + (b[i] as u64) + carry;
        r[i] = s as u32;
        carry = s >> 32;
    }
    // carry is 0 here (2p < 2²⁵⁶); kept for safety.
    let _ = carry;
    ct_sub_if_ge(&mut r);
    r
}

/// Field subtraction, reduced mod p. Returns `(a − b) mod p` for `a, b ∈ [0, p)`.
#[inline]
pub(crate) fn fe_sub(a: &Fe, b: &Fe) -> Fe {
    // (a − b) mod p = (a + p − b) mod p; a + p ∈ [p, 2p) so the difference is in [0, 2p).
    let mut t = [0u32; 8];
    let mut carry: u64 = 0;
    for i in 0..8 {
        let s = (a[i] as u64) + (P[i] as u64) + carry;
        t[i] = s as u32;
        carry = s >> 32;
    }
    let _ = carry;
    let mut r = [0u32; 8];
    let mut borrow: i64 = 0;
    for i in 0..8 {
        let s = (t[i] as i64) - (b[i] as i64) - borrow;
        if s < 0 {
            r[i] = (s + 0x1_0000_0000) as u32;
            borrow = 1;
        } else {
            r[i] = s as u32;
            borrow = 0;
        }
    }
    ct_sub_if_ge(&mut r);
    r
}

/// Constant-time field select: returns `b` if `bit == 1`, else `a`. `bit ∈ {0,1}`.
/// Branch-free limb masking — mirrors `sign.rs::fe_cselect`.
#[inline]
pub(crate) fn fe_cselect(bit: u8, a: &Fe, b: &Fe) -> Fe {
    // Full-width mask (NOT u8→u32, which would only gate the low 8 bits):
    // bit=1 → 0xFFFFFFFF, bit=0 → 0x00000000.
    let m = 0u32.wrapping_sub(bit as u32);
    let mut out = [0u32; 8];
    for i in 0..8 {
        out[i] = (a[i] & !m) | (b[i] & m);
    }
    out
}

// ── Wide (512-bit) helpers for multiplication + reduction ─────────────────────

/// Schoolbook 256×256 → 512-bit product, as sixteen u32 limbs.
/// Accumulates into a u128 so a limb product (≤ 2⁶⁴−2³³+1) + prior carry + existing limb
/// (each ≤ 2³²−1) can never overflow (max ≈ 2⁶⁴ + 2⁶⁵+2³² < 2¹²⁸). No data-dependent control
/// flow, so the multiply schedule is secret-independent.
fn mul_wide(a: &Fe, b: &Fe) -> [u32; 16] {
    let mut p = [0u128; 16];
    for i in 0..8 {
        for j in 0..8 {
            p[i + j] += (a[i] as u128) * (b[j] as u128);
        }
    }
    let mut out = [0u32; 16];
    let mut carry: u128 = 0;
    for i in 0..16 {
        let cur = p[i] + carry;
        out[i] = (cur & 0xFFFF_FFFF) as u32;
        carry = cur >> 32;
    }
    out
}

/// `x >> s` for a 512-bit little-endian value (s = 255 here). Obvious word+bit shift.
fn shr_bits(x: &[u32; 16], s: u32) -> [u32; 16] {
    let mut out = [0u32; 16];
    let word = (s / 32) as usize;
    let bit = s % 32;
    for i in 0..16 {
        if i + word >= 16 {
            break;
        }
        let mut v = x[i + word];
        if bit > 0 {
            v >>= bit;
            if i + word + 1 < 16 {
                v |= x[i + word + 1] << (32 - bit);
            }
        }
        out[i] = v;
    }
    out
}

/// `x mod 2^s` for a 512-bit little-endian value (s = 255 here).
fn low_bits(x: &[u32; 16], s: u32) -> [u32; 16] {
    let mut out = [0u32; 16];
    let word = (s / 32) as usize;
    let bit = s % 32;
    for i in 0..word {
        out[i] = x[i];
    }
    if word < 16 {
        if bit == 0 {
            out[word] = x[word];
        } else {
            out[word] = x[word] & ((1u32 << bit) - 1);
        }
    }
    out
}

/// `x * m` (m small) for a 512-bit little-endian value.
fn mul_small(x: &[u32; 16], m: u32) -> [u32; 16] {
    let mut out = [0u32; 16];
    let mut carry: u64 = 0;
    for i in 0..16 {
        let cur = (x[i] as u64) * (m as u64) + carry;
        out[i] = cur as u32;
        carry = cur >> 32;
    }
    out
}

/// 512-bit addition.
fn add_wide16(a: &[u32; 16], b: &[u32; 16]) -> [u32; 16] {
    let mut out = [0u32; 16];
    let mut carry: u64 = 0;
    for i in 0..16 {
        let cur = (a[i] as u64) + (b[i] as u64) + carry;
        out[i] = cur as u32;
        carry = cur >> 32;
    }
    out
}

/// Reduce a value < 2⁵¹² to its canonical residue in [0, p) using 2²⁵⁵ ≡ 19 (mod p).
///
/// Fold: `x = (x mod 2²⁵⁵) + 19·(x >> 2²⁵⁵)`; after two folds `x < 2²⁵⁶`, then at most two
/// subtractions of p land it in [0, p). The fold is the standard constant-time reduction; no
/// secret-dependent branch.
fn mod_p(x: &[u32; 16]) -> Fe {
    let mut r = *x;
    for _ in 0..4 {
        // 4 iterations is overkill (2 suffice for <2⁵¹²); harmless and clearly correct.
        let hi = shr_bits(&r, 255);
        let lo = low_bits(&r, 255);
        r = add_wide16(&lo, &mul_small(&hi, 19));
    }
    let mut out = [0u32; 8];
    out.copy_from_slice(&r[0..8]);
    ct_sub_if_ge(&mut out);
    ct_sub_if_ge(&mut out);
    out
}

/// Field multiplication, reduced mod p.
#[inline]
pub(crate) fn fe_mul(a: &Fe, b: &Fe) -> Fe {
    mod_p(&mul_wide(a, b))
}

/// Field squaring.
#[inline]
pub(crate) fn fe_sq(a: &Fe) -> Fe {
    fe_mul(a, a)
}

/// Decode a 32-byte little-endian string into a field element (RFC 7748 §5: the most
/// significant bit is ignored, i.e. forced to 0, then reduced mod p).
#[inline]
pub(crate) fn fe_from_bytes(s: &[u8; 32]) -> Fe {
    let mut w = [0u32; 16];
    for i in 0..8 {
        w[i] = load_le32(&s[i * 4..]);
    }
    w[7] &= 0x7FFF_FFFF; // clear bit 255 per RFC 7748 §5
    mod_p(&w)
}

/// Encode a field element to 32-byte little-endian (value is in [0, p) by invariant).
#[inline]
pub(crate) fn fe_to_bytes(f: &Fe) -> [u8; 32] {
    let mut out = [0u8; 32];
    for i in 0..8 {
        store_le32(&mut out[i * 4..], f[i]);
    }
    out
}

/// `z^(p−2)` in GF(p) via fixed-schedule square-and-multiply (the field inverse).
fn fe_invert(z: &Fe) -> Fe {
    let mut base = *z;
    let mut acc = FE_ONE;
    for bitpos in 0..255 {
        let w = bitpos / 32;
        let b = bitpos % 32;
        let bit = ((EXP_P_MINUS_2[w] >> b) & 1) as u8;
        let prod = fe_mul(&acc, &base);
        acc = fe_cselect(bit, &acc, &prod);
        base = fe_sq(&base);
    }
    acc
}

// ─────────────────────────────────────────────────────────────────────────────
// X25519 scalar multiplication (Montgomery ladder) — RFC 7748 §5 / §5.2.
// ─────────────────────────────────────────────────────────────────────────────

/// Branch-free conditional swap of two field elements. `swap ∈ {0,1}`.
/// xors limbs under `0xFF`/`0x00`; no data-dependent control flow.
#[inline]
fn cswap_fe(swap: u8, p: Fe, q: Fe) -> (Fe, Fe) {
    // Full-width mask (NOT u8→u32, which would only gate the low 8 bits).
    let m = 0u32.wrapping_sub(swap as u32); // swap=1 → 0xFFFFFFFF, swap=0 → 0x00000000
    let mut pp = p;
    let mut qq = q;
    for i in 0..8 {
        let t = (pp[i] ^ qq[i]) & m;
        pp[i] ^= t;
        qq[i] ^= t;
    }
    (pp, qq)
}

/// X25519(k, u): the u-coordinate of `k·u` on Curve25519, RFC 7748 §5 exactly.
///
/// `k` is clamped internally (RFC 7748 §5 clamp: `k[0] &= 248; k[31] &= 127; k[31] |= 64`),
/// so callers may pass any 32 bytes. Returns the 32-byte little-endian shared u-coordinate.
pub fn x25519(k_in: &[u8; 32], u_in: &[u8; 32]) -> [u8; 32] {
    let mut k = *k_in;
    k[0] &= 248;
    k[31] &= 127;
    k[31] |= 64;

    let x1 = fe_from_bytes(u_in);
    let mut x2 = FE_ONE;
    let mut z2 = FE_ZERO;
    let mut x3 = x1;
    let mut z3 = FE_ONE;
    let mut swap: u8 = 0;

    // FIXED 255 iterations (bit 254 → 0). The high bit 255 was cleared by clamping, so the
    // scalar is always a multiple of 8; the ladder never branches on a secret bit.
    for t in (0..=254).rev() {
        let bit = ((k[t / 8] >> (t % 8)) & 1) as u8;
        swap ^= bit;
        // Constant-time swap of (x2,x3) and (z2,z3) under `swap`.
        let (x2n, x3n) = cswap_fe(swap, x2, x3);
        x2 = x2n;
        x3 = x3n;
        let (z2n, z3n) = cswap_fe(swap, z2, z3);
        z2 = z2n;
        z3 = z3n;
        swap = bit;

        // Montgomery ladder step.
        let a = fe_add(&x2, &z2);
        let aa = fe_sq(&a);
        let b = fe_sub(&x2, &z2);
        let bb = fe_sq(&b);
        let e = fe_sub(&aa, &bb);
        let c = fe_add(&x3, &z3);
        let d = fe_sub(&x3, &z3);
        let da = fe_mul(&d, &a);
        let cb = fe_mul(&c, &b);
        let da_plus = fe_add(&da, &cb);
        let x3_new = fe_sq(&da_plus);
        let da_sub = fe_sub(&da, &cb);
        let z3_new = fe_mul(&x1, &fe_sq(&da_sub));
        let x2_new = fe_mul(&aa, &bb);
        let z2_new = fe_mul(&e, &fe_add(&aa, &fe_mul(&e, &FE_A24)));
        x2 = x2_new;
        z2 = z2_new;
        x3 = x3_new;
        z3 = z3_new;
    }

    // Final conditional swap.
    let (x2n, x3n) = cswap_fe(swap, x2, x3);
    x2 = x2n;
    x3 = x3n;
    let (z2n, z3n) = cswap_fe(swap, z2, z3);
    z2 = z2n;
    z3 = z3n;

    // Shared u-coordinate = x2 / z2 = x2 · z2^(p−2).
    let inv = fe_invert(&z2);
    let r = fe_mul(&x2, &inv);
    fe_to_bytes(&r)
}

// ─────────────────────────────────────────────────────────────────────────────
// Classical KEM leg (BLUEPRINT-P03 §3.6): ephemeral-X25519 DH.
//   encaps = ephemeral X25519 keypair + DH(ephemeral_secret, recipient_pub)
//   decaps = DH(recipient_secret, ephemeral_pub)
// The hybrid "X25519 ⊕ ML-KEM-768, shared secret = KDF over both" composition is Phase 9
// (transport use); this phase lands the primitive + its KAT. `basepoint_u` = 9 (RFC 7748 §5).
// ─────────────────────────────────────────────────────────────────────────────

pub const X25519_PUBLICKEYBYTES: usize = 32;
pub const X25519_SECRETKEYBYTES: usize = 32;
pub const X25519_CIPHERTEXTBYTES: usize = 32;
pub const X25519_SHAREDBYTES: usize = 32;

/// The Montgomery basepoint u-coordinate = 9 (little-endian).
const BASEPOINT_U: [u8; 32] = [
    9, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];

/// Generate an X25519 keypair from a 32-byte seed (the scalar). The public key is
/// `X25519(seed, 9)`. Clamping is applied inside `x25519`, so `seed` may be any 32 bytes.
pub fn keygen(seed: &[u8; 32]) -> ([u8; 32], [u8; 32]) {
    let sk = *seed;
    let pk = x25519(&sk, &BASEPOINT_U);
    (pk, sk)
}

/// Encap. `ct` = ephemeral public key; `shared` = DH(ephemeral_secret, recipient_pk).
pub fn encaps(pk: &[u8; 32], eph_seed: &[u8; 32]) -> ([u8; 32], [u8; 32]) {
    let ct = x25519(eph_seed, &BASEPOINT_U);
    let shared = x25519(eph_seed, pk);
    (shared, ct)
}

/// Decap. `shared` = DH(recipient_sk, ephemeral_ct).
pub fn decaps(sk: &[u8; 32], ct: &[u8; 32]) -> [u8; 32] {
    x25519(sk, ct)
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests — KAT-gated (RFC 7748 §5.2 canonical vectors) + roundtrip + negative gate.
// ─────────────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    // Parse a big-endian hex string and reverse to little-endian (RFC 7748 gives BE).
    fn hex_be_rev<const L: usize>(s: &str) -> [u8; L] {
        let s = s.trim();
        assert_eq!(s.len(), L * 2, "hex length mismatch");
        let mut out = [0u8; L];
        let bytes = s.as_bytes();
        for i in 0..L {
            let hi = (bytes[2 * i] as char).to_digit(16).unwrap();
            let lo = (bytes[2 * i + 1] as char).to_digit(16).unwrap();
            out[L - 1 - i] = ((hi << 4) | lo) as u8;
        }
        out
    }

    fn hex_le<const L: usize>(s: &str) -> [u8; L] {
        let s = s.trim();
        assert_eq!(s.len(), L * 2, "hex length mismatch");
        let mut out = [0u8; L];
        let bytes = s.as_bytes();
        for i in 0..L {
            let hi = (bytes[2 * i] as char).to_digit(16).unwrap();
            let lo = (bytes[2 * i + 1] as char).to_digit(16).unwrap();
            out[i] = ((hi << 4) | lo) as u8;
        }
        out
    }

    // RFC 7748 §5.2 canonical test vectors (given big-endian; reversed to LE for our API).
    const ALICE_PRIV: &str = "77076d0a7318a57d3c16c17251b26645df4c2f87ebc0992ab177fba51db92c2a";
    const BOB_PRIV: &str = "5dab087e624a8a4b79e17f8b83800ee66f3bb1292618b6fd1c2f8b27ff88e0eb";
    const ALICE_PUB: &str = "8520f0098930a754748b7ddcb43ef75a0dbf3a0d26381af4eba4a98eaa9b4e6a";
    const BOB_PUB: &str = "de9edb7d7b7dc1b4d35b61c2ece435373f8343c85b78674dadfc7e146f882b4f";
    const SHARED: &str = "4a5d9d5ba4ce2de1728e3bf480350f25e07e21c947d19e3376f09b3c1e161742";

    /// RFC 7748 §5.2 KAT — the falsifiable gate. MUST pass bit-for-bit.
    /// Covers: the two single-iteration vectors (X25519(a,9), X25519(b,9)) AND the
    /// k=1-iteration Alice/Bob DH shared secret, both directions (symmetry).
    #[test]
    fn rfc7748_x25519_kat() {
        // RFC 7748 §5.2 vectors are LITTLE-ENDIAN byte strings (first byte = LSB).
        let a = hex_le::<32>(ALICE_PRIV);
        let b = hex_le::<32>(BOB_PRIV);
        let alice_pub = hex_le::<32>(ALICE_PUB);
        let bob_pub = hex_le::<32>(BOB_PUB);
        let shared = hex_le::<32>(SHARED);
        let base9 = BASEPOINT_U;

        // Single-iteration vector 1: X25519(a, 9) = Alice's public key.
        assert_eq!(x25519(&a, &base9), alice_pub, "X25519(a,9) mismatch");
        // Single-iteration vector 2: X25519(b, 9) = Bob's public key.
        assert_eq!(x25519(&b, &base9), bob_pub, "X25519(b,9) mismatch");
        // k=1-iteration (DH) shared secret: X25519(a, X25519(b,9)) = K.
        let k_ab = x25519(&a, &bob_pub);
        assert_eq!(k_ab, shared, "X25519(a, BobPub) shared-secret mismatch");
        // Symmetry: Bob must derive the same secret from Alice's public key.
        let k_ba = x25519(&b, &alice_pub);
        assert_eq!(k_ba, shared, "X25519(b, AlicePub) shared-secret mismatch");
    }

    /// KEM API reaches the same RFC 7748 public key as the primitive (ties keygen to the KAT).
    #[test]
    fn x25519_keygen_matches_kat() {
        let a = hex_le::<32>(ALICE_PRIV);
        let alice_pub = hex_le::<32>(ALICE_PUB);
        let (pk, _sk) = keygen(&a);
        assert_eq!(pk, alice_pub);
    }

    /// Roundtrip: encaps/decaps agree on the shared secret.
    #[test]
    fn x25519_kem_roundtrip() {
        let recipient_seed = [0x42u8; 32];
        let (pk, sk) = keygen(&recipient_seed);
        let eph_seed = [0x07u8; 32];
        let (ss_enc, ct) = encaps(&pk, &eph_seed);
        let ss_dec = decaps(&sk, &ct);
        assert_eq!(ss_enc, ss_dec, "encaps/decaps shared secret mismatch");
    }

    /// Negative gate (corrupted-ciphertext MUST change the output): a tampered ephemeral
    /// public key yields a different shared secret — no silent reuse / implicit-accept.
    #[test]
    fn x25519_tamper_changes_secret() {
        let recipient_seed = [0x42u8; 32];
        let (pk, sk) = keygen(&recipient_seed);
        let eph_seed = [0x07u8; 32];
        let (ss_enc, mut ct) = encaps(&pk, &eph_seed);
        // Flip a byte in the ciphertext (ephemeral public key).
        ct[0] ^= 0xFF;
        let ss_dec = decaps(&sk, &ct);
        assert_ne!(
            ss_enc, ss_dec,
            "tampered ciphertext must NOT reproduce the secret"
        );
    }

    /// Clamping is applied: two scalars differing ONLY in clamped bits (low 3 bits + high bit)
    /// derive the SAME public key (RFC 7748 §5 clamp). Here k1 and k2 share all non-clamped
    /// bits (set to 1) and differ only in the clamped positions, so after clamping both become
    /// identical → identical public key.
    #[test]
    fn x25519_clamping_is_deterministic() {
        // k1 has all bits set (including the clamped positions); k2 is k1 with the
        // clamped bits already forced to their canonical form. After clamping both
        // become byte-identical, so they must derive the same public key.
        let mut k1 = [0xFFu8; 32];
        let mut k2 = [0xFFu8; 32];
        k2[0] &= 0xF8;                 // clear low 3 bits
        k2[31] = (k2[31] & 0x7F) | 0x40; // clear high bit, set bit 6
        let p1 = x25519(&k1, &BASEPOINT_U);
        let p2 = x25519(&k2, &BASEPOINT_U);
        assert_eq!(p1, p2, "clamped scalars must map to the same public key");
    }

    /// RFC 7748 §5.2 (final vector): `X25519(a546e36bf0527c9d3b16154b82465edd62144c0ac1fc5a18506a2244ba449ac4, 0)`
    /// = 0. The all-zero *input* u-coordinate maps to 0; this exercises the z2 == 0 →
    /// fe_invert == 0 → shared = 0 path without panic, and pins the RFC's exact scalar.
    #[test]
    fn x25519_zero_u_maps_to_zero() {
        let k = hex_le::<32>("a546e36bf0527c9d3b16154b82465edd62144c0ac1fc5a18506a2244ba449ac4");
        let zero = [0u8; 32];
        assert_eq!(
            x25519(&k, &zero),
            zero,
            "X25519(k, 0) must be 0 (RFC 7748 §5.2)"
        );
    }
}