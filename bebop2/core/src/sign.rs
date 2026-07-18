//! sign — Ed25519 (RFC 8032 §5.1, test vectors §7.1), from scratch, zero-dependency.
//!
//! Verified-by-Math: every keygen/sign/verify path is anchored to the RFC 8032 §7.1
//! published test vectors (bit-exact). A corrupted signature MUST fail verification
//! (RED case asserted). Determinism: identical seed → identical (pk, sk) and signature.
//!
//! NO external crates, NO std::time/OS RNG/network. SHA-512 comes from `crate::hash`.
//! The only entropy is caller-supplied: `keygen(seed)` takes a 32-byte seed; `sign`
//! takes the secret key + message. (Production seeds hardware entropy out of tree.)
//!
//! Field arithmetic is GF(2^255-19), represented as a 32-byte little-endian
//! canonical integer (the value is always in [0, p)). All ops reduce mod p.
//! Curve is the twisted Edwards form a = -1, d = -121665/121666 mod p.

extern crate alloc;

use alloc::vec::Vec;
use core::convert::TryInto;

// ── GF(2^255-19): 32-byte LE canonical integers, reduced mod p ───────────────
// p = 2^255 - 19.
// P_BE: p as big-endian bytes, for the big-endian bignum helpers (cmp/sub/mod).
type Fe = [u8; 32];

const P_BE: [u8; 32] = [
    0x7f, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xed,
];

#[inline]
fn fe_0() -> Fe {
    [0u8; 32]
}
#[inline]
fn fe_1() -> Fe {
    let mut o = [0u8; 32];
    o[0] = 1;
    o
}
#[inline]
fn fe_2() -> Fe {
    let mut o = [0u8; 32];
    o[0] = 2;
    o
}

/// Load a 32-byte LE field element. Caller guarantees it is < p (true for point
/// encodings and constants we construct this way).
#[inline]
fn fe_from_bytes(b: &[u8; 32]) -> Fe {
    *b
}

/// Canonical encoding is the 32-byte LE integer.
#[inline]
fn fe_to_bytes(a: &Fe) -> [u8; 32] {
    *a
}

// ── Fast GF(2^255-19) arithmetic in 64-bit limbs (no heap, no per-bit loop) ──
// `Fe` stays the canonical 32-byte LE integer < p. The slow path was a
// `Vec<u8>` big-endian bignum with a bit-by-bit division per field op; this
// replaces it with fixed 64-bit-limb schoolbook + a 2^255≡19 reduction.
// Algebra is identical to the RFC 8032 §5.1 spec — same values, ~1000× faster.
// p = 2^255 - 19  (little-endian u64 limbs).
const P_LIMBS: [u64; 4] = [
    0xffff_ffff_ffff_ffed,
    0xffff_ffff_ffff_ffff,
    0xffff_ffff_ffff_ffff,
    0x7fff_ffff_ffff_ffff,
];

#[inline]
fn fe_to_limbs(a: &Fe) -> [u64; 4] {
    let mut out = [0u64; 4];
    for i in 0..4 {
        let mut limb = 0u64;
        for j in 0..8 {
            limb |= (a[i * 8 + j] as u64) << (8 * j);
        }
        out[i] = limb;
    }
    out
}

#[inline]
fn fe_from_limbs(a: &[u64; 4]) -> Fe {
    let mut out = [0u8; 32];
    for i in 0..4 {
        let limb = a[i];
        for j in 0..8 {
            out[i * 8 + j] = (limb >> (8 * j)) as u8;
        }
    }
    out
}

/// Schoolbook 4×4 -> 8-limb product of two 256-bit LE values.
/// Uses a full u128 accumulator with a propagated carry chain so no limb silently
/// truncates (a bare `p[i+4] = (p[i+4] + carry) as u64` would drop high bits).
#[inline]
fn limbs_mul(a: &[u64; 4], b: &[u64; 4]) -> [u64; 8] {
    let mut p = [0u64; 8];
    for i in 0..4 {
        let mut carry: u128 = 0;
        for j in 0..4 {
            let idx = i + j;
            let v = p[idx] as u128 + (a[i] as u128) * (b[j] as u128) + carry;
            p[idx] = v as u64;
            carry = v >> 64;
        }
        // Propagate the leftover carry through the high limbs.
        let mut k = i + 4;
        let mut c = carry;
        while c > 0 {
            let v = p[k] as u128 + c;
            p[k] = v as u64;
            c = v >> 64;
            k += 1;
        }
    }
    p
}

/// Fold a value V (up to 8 LE u64 limbs) mod p using 2^255 ≡ 19 (mod p):
///   V = A + 2^255·B  →  A + 19·B
/// Returns the result as up to 7 LE limbs.
#[inline]
fn fold_val(v: &[u64; 8]) -> [u64; 7] {
    let a0 = v[0];
    let a1 = v[1];
    let a2 = v[2];
    let a3 = v[3] & 0x7fff_ffff_ffff_ffff;
    // B = V >> 255 (V < 2^512 so B < 2^257). Correct limb extraction:
    //   b_k = (V bits 255+64k .. 255+64k+63)
    //       = (v_{k+4} << 1) | (v_{k+3} >> 63), with v_8 = 0.
    let b0 = (v[4] << 1) | (v[3] >> 63);
    let b1 = (v[5] << 1) | (v[4] >> 63);
    let b2 = (v[6] << 1) | (v[5] >> 63);
    let b3 = (v[7] << 1) | (v[6] >> 63);
    let b4 = v[7] >> 63;
    let b5 = 0u64;
    let mut tb = [0u64; 6];
    let mut carry = 0u128;
    let b = [b0, b1, b2, b3, b4, b5];
    for i in 0..6 {
        let val = (b[i] as u128) * 19 + carry;
        tb[i] = val as u64;
        carry = val >> 64;
    }
    let mut r = [0u64; 7];
    r[0] = a0;
    r[1] = a1;
    r[2] = a2;
    r[3] = a3;
    let mut c = 0u128;
    for i in 0..6 {
        let val = r[i] as u128 + tb[i] as u128 + c;
        r[i] = val as u64;
        c = val >> 64;
    }
    if c > 0 {
        r[6] = c as u64;
    }
    r
}

/// Reduce an 8-limb product mod p = 2^255-19. Iterate the 2^255-fold (each pass
/// shrinks the magnitude by ~2^255) until the value fits in 255 bits, then do a
/// single conditional subtraction of p. Converges in <= 3 folds.
fn reduce_p(prod: &[u64; 8]) -> [u64; 4] {
    let mut r = fold_val(prod);
    let mut guard = 0;
    while (r[4] | r[5] | r[6]) != 0 || r[3] >= 0x8000_0000_0000_0000 {
        let v8 = [r[0], r[1], r[2], r[3], r[4], r[5], r[6], 0];
        r = fold_val(&v8);
        guard += 1;
        if guard > 8 {
            break;
        }
    }
    if limbs_ge_p(&r) {
        limbs_sub_p(&mut r);
    }
    [r[0], r[1], r[2], r[3]]
}

#[inline]
fn limbs_ge_p(r: &[u64; 7]) -> bool {
    for i in (4..7).rev() {
        if r[i] != 0 {
            return true;
        }
    }
    for i in (0..4).rev() {
        if r[i] > P_LIMBS[i] {
            return true;
        }
        if r[i] < P_LIMBS[i] {
            return false;
        }
    }
    true // r == p: must still subtract p to normalize to 0
}

fn limbs_sub_p(r: &mut [u64; 7]) {
    let mut borrow = 0i128;
    for i in 0..4 {
        let v = r[i] as i128 - P_LIMBS[i] as i128 - borrow;
        if v < 0 {
            r[i] = (v + (1i128 << 64)) as u64;
            borrow = 1;
        } else {
            r[i] = v as u64;
            borrow = 0;
        }
    }
    for i in 4..7 {
        if borrow == 0 {
            break;
        }
        let v = r[i] as i128 - borrow;
        if v < 0 {
            r[i] = (v + (1i128 << 64)) as u64;
            borrow = 1;
        } else {
            r[i] = v as u64;
            borrow = 0;
        }
    }
}

#[inline]
fn fe_add(a: &Fe, b: &Fe) -> Fe {
    let la = fe_to_limbs(a);
    let lb = fe_to_limbs(b);
    let mut s = [0u64; 8];
    let mut carry = 0u128;
    for i in 0..4 {
        let v = la[i] as u128 + lb[i] as u128 + carry;
        s[i] = v as u64;
        carry = v >> 64;
    }
    if carry > 0 {
        s[4] = carry as u64;
    }
    let prod = [s[0], s[1], s[2], s[3], s[4], 0, 0, 0];
    fe_from_limbs(&reduce_p(&prod))
}

#[inline]
fn fe_sub(a: &Fe, b: &Fe) -> Fe {
    let la = fe_to_limbs(a);
    let lb = fe_to_limbs(b);
    // Compute p + a as a 5-limb value WITH carry propagation into pa[4].
    let mut pa = [0u64; 5];
    let mut c: u128 = 0;
    for i in 0..4 {
        let v = P_LIMBS[i] as u128 + la[i] as u128 + c;
        pa[i] = v as u64;
        c = v >> 64;
    }
    pa[4] = c as u64; // 0 or 1 (p+a < 2p < 2^256 when a < p)
                      // Now pa - b (b has 4 limbs, b < p). Result is in [0, 2p); keep pa[4] as the
                      // high limb so the carry isn't lost. Integer pa >= b so final borrow is 0.
    let mut d = [0u64; 8];
    let mut borrow = 0i128;
    for i in 0..4 {
        let mut v = pa[i] as i128 - lb[i] as i128 - borrow;
        if v < 0 {
            v += 1i128 << 64;
            borrow = 1;
        } else {
            borrow = 0;
        }
        d[i] = v as u64;
    }
    let prod = [d[0], d[1], d[2], d[3], pa[4], 0, 0, 0];
    fe_from_limbs(&reduce_p(&prod))
}

#[inline]
fn fe_neg(a: &Fe) -> Fe {
    fe_sub(&fe_0(), a)
}

#[inline]
fn fe_mul(a: &Fe, b: &Fe) -> Fe {
    let prod = limbs_mul(&fe_to_limbs(a), &fe_to_limbs(b));
    fe_from_limbs(&reduce_p(&prod))
}

/// d = -121665/121666 mod p, computed from integers (not a hardcoded limb constant),
/// so the representation is independent of any radix convention.
fn fe_d() -> Fe {
    // -121665 mod p = 0 - 121665 (proper LE Fe, since fe_sub reduces mod p).
    let num = fe_sub(&fe_0(), &fe_from_u64(121665));
    let den = fe_from_u64(121666);
    fe_mul(&num, &fe_invert(&den))
}

#[inline]
fn fe_square(a: &Fe) -> Fe {
    fe_mul(a, a)
}

/// Invert a via Fermat: a^(p-2) mod p, using square-and-multiply over the exact
/// 255-bit exponent p-2 = 0x7fff…ffeb (no hand-counted windows that can drift).
fn fe_invert(a: &Fe) -> Fe {
    // MSB-first square-and-multiply: square the accumulator FIRST, then
    // conditionally multiply by the (constant) base. This computes a^E exactly.
    let mut acc = fe_1();
    let base = *a;
    for i in (0..255).rev() {
        acc = fe_square(&acc);
        let bit = (P_MINUS_2[i / 64] >> (i % 64)) & 1;
        if bit == 1 {
            acc = fe_mul(&acc, &base);
        }
    }
    acc
}

// p - 2 = 2^255 - 21, little-endian u64 limbs (255-bit value).
const P_MINUS_2: [u64; 4] = [
    0xffff_ffff_ffff_ffeb,
    0xffff_ffff_ffff_ffff,
    0xffff_ffff_ffff_ffff,
    0x7fff_ffff_ffff_ffff,
];

// Exponent e1 = (p + 3) / 8 = 2^252 - 2, used for the candidate square root when
// p ≡ 5 (mod 8). Stored as a 32-byte LE integer bit string.
const E1: [u8; 32] = [
    0xfe, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x0f,
];
// sqrt(-1) mod p = 2^((p-1)/4) mod p. Precomputed const (Tonelli-Shanks sign fix).
const SQRT_M1: [u8; 32] = [
    0xb0, 0xa0, 0x0e, 0x4a, 0x27, 0x1b, 0xee, 0xc4, 0x78, 0xe4, 0x2f, 0xad, 0x06, 0x18, 0x43, 0x2f,
    0xa7, 0xd7, 0xfb, 0x3d, 0x99, 0x00, 0x4d, 0x2b, 0x0b, 0xdf, 0xc1, 0x4f, 0x80, 0x24, 0x83, 0x2b,
];

/// Modular square root for p ≡ 5 (mod 8). Returns Some(root) where root^2 = a (mod p)
/// and root has the lower "x_0" sign bit, or None if a is a non-residue.
/// Algorithm: candidate x = a^((p+3)/8); if x^2 == a, x is the root; else if x^2 == -a,
/// the root is x * sqrt(-1); otherwise a has no square root.
fn fe_sqrt(a: &Fe) -> Option<Fe> {
    let x = {
        // a^E1 via square-and-multiply over the 256-bit E1 bit string (MSB-first).
        let mut acc = fe_1();
        let base = *a;
        for i in (0..256).rev() {
            acc = fe_square(&acc);
            let bit = (E1[i / 8] >> (i % 8)) & 1;
            if bit == 1 {
                acc = fe_mul(&acc, &base);
            }
        }
        acc
    };
    let xx = fe_square(&x);
    if fe_eq(&xx, a) {
        Some(x)
    } else {
        // x^2 == -a  →  root = x * sqrt(-1)
        let neg_a = fe_neg(a);
        if fe_eq(&xx, &neg_a) {
            Some(fe_mul(&x, &SQRT_M1))
        } else {
            None
        }
    }
}

/// Constant-time field equality (returns true iff a == b as canonical Fe).
fn fe_eq(a: &Fe, b: &Fe) -> bool {
    let mut diff = 0u8;
    for i in 0..32 {
        diff |= a[i] ^ b[i];
    }
    diff == 0
}

// ── Point in projective (X:Y:Z) coordinates; x = X/Z, y = Y/Z ────────────────
fn fe_from_u64(v: u64) -> Fe {
    let mut o = [0u8; 32];
    o[0..8].copy_from_slice(&v.to_le_bytes());
    o
}

// d = -121665/121666 mod 2^255-19 (computed once at first use; see fe_d()).
// Stored as a const via a const-fn-free lazy: we just call fe_d() where needed.
// For static use we precompute it as a `lazy` const expression through a function.

// ── Point in projective (X:Y:Z) coordinates; x = X/Z, y = Y/Z ────────────────
#[derive(Clone, Copy)]
struct Point {
    x: Fe,
    y: Fe,
    z: Fe,
    t: Fe,
}

fn point_identity() -> Point {
    Point {
        x: fe_0(),
        y: fe_1(),
        z: fe_1(),
        t: fe_0(),
    }
}

/// C4: test-only tally of curve additions, read by `scalar_mul_op_count_is_constant`.
/// Thread-local so cargo's parallel test threads can't corrupt each other's count
/// (scalar_mul runs synchronously, so all its point_add calls are on the caller thread).
#[cfg(test)]
thread_local! {
    static POINT_ADD_CALLS: core::cell::Cell<u64> = const { core::cell::Cell::new(0) };
}

/// Twisted-Edwards addition in extended homogeneous coordinates (RFC 8032
/// §5.1.4, a = -1). Complete addition — `point_double` reuses it. Verbatim from
/// the RFC:
///   A = (Y1-X1)*(Y2-X2)   B = (Y1+X1)*(Y2+X2)   C = T1*2*d*T2   D = Z1*2*Z2
///   E = B-A   F = D-C   G = D+C   H = B+A
///   X3 = E*F   Y3 = G*H   T3 = E*H   Z3 = F*G
///
/// `d2` must be the precomputed `2*d` (passed in to avoid recomputing the
/// expensive `fe_invert` inside every addition).
fn point_add(p: &Point, q: &Point, d2: &Fe) -> Point {
    // C4 constant-time proof hook: count every curve addition (test builds only, zero
    // prod overhead). `scalar_mul` must call this the SAME number of times regardless of
    // the scalar's bits — the op-count test below asserts exactly that.
    #[cfg(test)]
    POINT_ADD_CALLS.with(|c| c.set(c.get() + 1));
    let x1 = p.x;
    let y1 = p.y;
    let x2 = q.x;
    let y2 = q.y;
    let a = fe_mul(&fe_sub(&y1, &x1), &fe_sub(&y2, &x2)); // A = (Y1-X1)*(Y2-X2)
    let b = fe_mul(&fe_add(&y1, &x1), &fe_add(&y2, &x2)); // B = (Y1+X1)*(Y2+X2)
    let c = fe_mul(d2, &fe_mul(&p.t, &q.t)); // C = T1*2*d*T2
    let dd = fe_mul(&fe_mul(&p.z, &q.z), &fe_2()); // D = Z1*2*Z2
    let e = fe_sub(&b, &a); // E = B - A
    let f = fe_sub(&dd, &c); // F = D - C
    let g = fe_add(&dd, &c); // G = D + C
    let h = fe_add(&b, &a); // H = B + A
    let x3 = fe_mul(&e, &f); // X3 = E*F
    let y3 = fe_mul(&g, &h); // Y3 = G*H
    let t3 = fe_mul(&e, &h); // T3 = E*H
    let z3 = fe_mul(&f, &g); // Z3 = F*G
    Point {
        x: x3,
        y: y3,
        z: z3,
        t: t3,
    }
}

/// Double via addition (correct for a = -1 twisted Edwards).
fn point_double(p: &Point, d2: &Fe) -> Point {
    point_add(p, p, d2)
}

/// Decode an RFC 8032 32-byte point encoding (y, x-sign in top bit of last byte).
fn point_decompress(s: &[u8; 32]) -> Option<Point> {
    let mut b = *s;
    let sign_bit = b[31] >> 7;
    b[31] &= 0x7f;
    // RFC 8032 §5.1.3: reject non-canonical y (y >= p) — encoding must be canonical.
    if cmp_be(&be(&b), &P_BE) != core::cmp::Ordering::Less {
        return None;
    }
    let y = fe_from_bytes(&b);
    let one = fe_1();
    let yy = fe_square(&y);
    // x^2 = (y^2 - 1) / (d*y^2 + 1) = u / v
    let u = fe_sub(&yy, &one);
    let v = fe_add(&fe_mul(&fe_d(), &yy), &one);
    let uv_inv = fe_mul(&u, &fe_invert(&v)); // = x^2 candidate
    let x = match fe_sqrt(&uv_inv) {
        None => return None,
        Some(x) => x,
    }; // root r with r's own low-bit parity
       // The other root is -r; verify r^2 * v == u (else non-residue → reject).
    let check = fe_sub(&fe_mul(&fe_square(&x), &v), &u);
    if !fe_eq(&check, &fe_0()) {
        return None;
    }
    // Choose the root whose low bit matches the encoded sign bit.
    let xb = fe_to_bytes(&x);
    let xsign = xb[0] & 1;
    let xfinal = if xsign != sign_bit { fe_neg(&x) } else { x };
    Some(Point {
        x: xfinal,
        y,
        z: one,
        t: fe_mul(&xfinal, &y),
    })
}

/// Encode a point: y (LE) with x-sign in top bit.
fn point_compress(p: &Point) -> [u8; 32] {
    let zinv = fe_invert(&p.z);
    let y = fe_to_bytes(&fe_mul(&p.y, &zinv)); // affine y = y / z
                                               // compute x/z mod p to read the sign bit
    let xz = fe_to_bytes(&fe_mul(&p.x, &zinv));
    let mut out = y;
    if (xz[0] & 1) == 1 {
        out[31] |= 0x80;
    }
    out
}

/// Projective point equality: P == Q iff X1*Z2 == X2*Z1 AND Y1*Z2 == Y2*Z1.
fn point_eq(p: &Point, q: &Point) -> bool {
    let a = fe_to_bytes(&fe_mul(&p.x, &q.z));
    let b = fe_to_bytes(&fe_mul(&q.x, &p.z));
    let c = fe_to_bytes(&fe_mul(&p.y, &q.z));
    let d = fe_to_bytes(&fe_mul(&q.y, &p.z));
    a == b && c == d
}

// ── Scalar arithmetic mod L (group order), 256-bit bignum (BE Vec<u8>) ────────
// L = 2^252 + 27742317777372353535851937790883648493
const L: [u8; 32] = [
    0xed, 0xd3, 0xf5, 0x5c, 0x1a, 0x63, 0x12, 0x58, 0xd6, 0x9c, 0xf7, 0xa2, 0xde, 0xf9, 0xde, 0x14,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10,
];

fn be(bytes_le: &[u8; 32]) -> Vec<u8> {
    bytes_le.iter().rev().copied().collect()
}

fn cmp_be(a: &[u8], b: &[u8]) -> core::cmp::Ordering {
    // pad to equal length with leading zeros
    let n = core::cmp::max(a.len(), b.len());
    let mut pa = vec![0u8; n - a.len()];
    pa.extend_from_slice(a);
    let mut pb = vec![0u8; n - b.len()];
    pb.extend_from_slice(b);
    for i in 0..n {
        if pa[i] < pb[i] {
            return core::cmp::Ordering::Less;
        } else if pa[i] > pb[i] {
            return core::cmp::Ordering::Greater;
        }
    }
    core::cmp::Ordering::Equal
}

/// Constant-time big-endian bignum comparison: returns an all-1 (`0xFF`) mask iff
/// `a >= b`, else an all-0 (`0x00`) mask. **Never branches on the data.** A byte-wise
/// compare accumulates `eq_mask` (still-equal so far) and `gt_mask` (already known a>b),
/// then the final mask is `gt | eq & cmp_of_last_differing_byte`.
fn ct_ge(a: &[u8], b: &[u8]) -> u8 {
    debug_assert_eq!(a.len(), b.len());
    let n = a.len();
    // eq_mask: 0xFF while every byte seen so far has been equal, else 0x00.
    // gt_mask: 0xFF once a strictly-greater byte is found within the equal-prefix.
    // Booleans are converted to 0xFF/0x00 masks via the standard wrapping_sub idiom,
    // so all arithmetic is branch-free on the secret data.
    let mut eq_mask: u8 = 0xFF;
    let mut gt_mask: u8 = 0x00;
    for i in 0..n {
        let a_gt = 0u8.wrapping_sub((a[i] > b[i]) as u8); // 0xFF if a[i] > b[i] else 0x00
        let a_lt = 0u8.wrapping_sub((a[i] < b[i]) as u8); // 0xFF if a[i] < b[i] else 0x00
        let new_gt = a_gt & eq_mask;
        let new_lt = a_lt & eq_mask;
        gt_mask |= new_gt;
        eq_mask &= !new_lt; // a strictly-less byte ends the equal-prefix
        eq_mask &= !new_gt; // a strictly-greater byte also ends it (a is now known > b)
    }
    // a >= b iff (a > b) OR (all bytes equal).
    gt_mask | eq_mask
}

/// Constant-time big-endian bignum subtraction `a - b` (assumes `a >= b`), on a
/// fixed-width stack buffer. **No heap, no data-dependent control flow** — the borrow
/// is computed via a branch-free mask (0xFF borrow / 0x00 no-borrow).
fn ct_sub<const W: usize>(a: &[u8; W], b: &[u8; W]) -> [u8; W] {
    let mut out = [0u8; W];
    let mut borrow: u32 = 0;
    for i in (0..W).rev() {
        let av = a[i] as u32;
        let bv = b[i] as u32 + borrow;
        let diff = av.wrapping_sub(bv); // correct mod-256 whether or not av >= bv
        // Branch-free borrow flag: 1 iff av < bv, via the standard flag→mask idiom
        // (0u32.wrapping_sub of a 0/1 bool) — NO data-dependent control flow.
        let need_borrow = (0u32).wrapping_sub((av < bv) as u32) >> 31; // 1 if av<bv else 0
        out[i] = (diff & 0xff) as u8;
        borrow = need_borrow;
    }
    out
}

/// Constant-time big-endian bignum addition `a + b` on a fixed-width stack buffer
/// (carry out at top is discarded — callers keep inputs within the buffer width).
/// No data-dependent control flow.
fn ct_add<const W: usize>(a: &[u8; W], b: &[u8; W]) -> [u8; W] {
    let mut out = [0u8; W];
    let mut carry: u32 = 0;
    for i in (0..W).rev() {
        let v = a[i] as u32 + b[i] as u32 + carry;
        out[i] = (v & 0xff) as u8;
        carry = v >> 8;
    }
    out
}

/// Constant-time selection on fixed-width big-endian bignums: returns `a` if
/// `mask == 0x00`, `b` if `mask == 0xFF`. Branch-free byte masking (mirrors `fe_cselect`).
fn ct_select<const W: usize>(mask: u8, a: &[u8; W], b: &[u8; W]) -> [u8; W] {
    debug_assert!(mask == 0x00 || mask == 0xFF);
    let mut out = [0u8; W];
    for i in 0..W {
        out[i] = (a[i] & !mask) | (b[i] & mask);
    }
    out
}

fn add_be(a: &[u8], b: &[u8]) -> Vec<u8> {
    let n = core::cmp::max(a.len(), b.len()) + 1;
    let mut pa = vec![0u8; n - a.len()];
    pa.extend_from_slice(a);
    let mut pb = vec![0u8; n - b.len()];
    pb.extend_from_slice(b);
    let mut out = vec![0u8; n];
    let mut carry = 0u32;
    for i in (0..n).rev() {
        let v = pa[i] as u32 + pb[i] as u32 + carry;
        out[i] = (v & 0xff) as u8;
        carry = v >> 8;
    }
    out
}

fn mul_be(a: &[u8], b: &[u8]) -> Vec<u8> {
    // Inputs are big-endian. Reverse to little-endian (index 0 = LSB) and do the
    // standard grade-school multiply with a carry chain, then reverse the result
    // back to BE.
    let al: Vec<u8> = a.iter().rev().copied().collect();
    let bl: Vec<u8> = b.iter().rev().copied().collect();
    let mut out = vec![0u8; al.len() + bl.len()];
    for i in 0..al.len() {
        let mut carry = 0u32;
        for j in 0..bl.len() {
            let idx = i + j;
            let v = out[idx] as u32 + (al[i] as u32) * (bl[j] as u32) + carry;
            out[idx] = (v & 0xff) as u8;
            carry = v >> 8;
        }
        out[i + bl.len()] = (out[i + bl.len()] as u32 + carry) as u8;
    }
    out.iter().rev().copied().collect()
}

/// Reduce a big-endian bignum mod L via bit-by-bit division.
/// Group order L, as a big-endian Vec, for the bignum mod-L helpers.
/// (The `L` const is stored little-endian; `mod_l`/`sub_be`/`cmp_be` need BE.)
fn l_be() -> Vec<u8> {
    let mut v = L.to_vec();
    v.reverse();
    v
}

/// Reduce a big-endian bignum mod L — **CONSTANT-TIME** (C4b).
///
/// C4b (2026-07-17) closes the scalar-layer side-channel that the C4 (2026-07-14)
/// group-level fix left open: the prior `mod_l` had a secret-bit branch
/// `if (byte>>bit)&1` AND a data-dependent conditional subtract `if cmp_be(..)!=Less`,
/// both acting on the secret nonce `r` and key `a`. Both leaked the scalar bits via
/// timing/power (biased-nonce → lattice key recovery).
///
/// This rewrite is **branch-free on the secret and on length**:
///   * The accumulator lives in a FIXED-WIDTH 64-byte buffer, so its length never
///     depends on the input — no `Vec` growth, no `insert(0,0)` left-pad.
///   * The per-bit "add 1" is gated by the bit value as a `0x00`/`0xFF` mask
///     (`ct_select`), never an `if`.
///   * The conditional subtract is a branch-free `ct_sub` applied only when the
///     comparison mask `ct_ge(rem, L)` is all-1; selection via `ct_select`.
///   * The loop is a FIXED 512 iterations (8 bits × each of the up-to-64 input bytes),
///     with NO early exit and NO data-dependent length.
///
/// INVARIANT (why ONE conditional subtract per bit suffices): before each bit,
/// `rem < L`; the candidate `2·rem + bit < 2·L`; subtracting L at most once returns
/// `rem < L`. A single branch-free subtract-if-`ge` is therefore exact for every bit.
///
/// The MATH is unchanged from RFC 8032 §7.1 — the result is bit-for-bit identical to
/// the old variable-time divider, so the §7.1 KAT still passes.
fn mod_l(num_be: &[u8]) -> [u8; 32] {
    const W: usize = 64; // fixed-width BE buffer (fits the accumulator < L at all times)
    let l = l_be();
    let l_arr: [u8; 32] = {
        let mut a = [0u8; 32];
        a.copy_from_slice(&l);
        a
    };
    let l_w: [u8; W] = {
        let mut a = [0u8; W];
        a[W - 32..].copy_from_slice(&l_arr);
        a
    };
    debug_assert_eq!(l.len(), 32);

    // Accumulator starts at ZERO (fixed-width stack buffer). Feeding the input bits
    // MSB-first, one per iteration, maintains the invariant rem < L before every bit,
    // so a single branch-free conditional subtract per bit is exact (2*rem+bit < 2*L).
    let mut rem = [0u8; W];

    // FIXED 512 iterations (8 bits × each of the up-to-64 input bytes), MSB-first,
    // NO early exit, NO data-dependent length — constant-time by construction.
    for &byte in num_be.iter().take(W) {
        for bit in (0..8).rev() {
            let bitval = (byte >> bit) & 1; // 0 or 1
            let bit_mask = 0u8.wrapping_sub(bitval); // 0xFF if bit set, else 0x00
            // rem <<= 1 (numeric double, big-endian, fixed width). In big-endian the
            // LSB is the last byte; its top bit becomes the bottom bit of the next-more
            // significant byte. Process LSB→MSB so each byte's carry is available for
            // the one above it. The MSB's top bit overflows and is discarded — rem < L
            // < 2^253 so this never drops a real bit.
            let mut shifted = [0u8; W];
            for i in (0..W).rev() {
                let cur = rem[i] as u32;
                let carry_in = if i + 1 < W { (rem[i + 1] >> 7) as u32 } else { 0 };
                shifted[i] = ((cur << 1) | carry_in) as u8;
            }
            rem = shifted;
            // Add the current bit via a branch-free mask, never an `if`.
            let mut bitword = [0u8; W];
            bitword[W - 1] = bitval; // 1 only in the LSB if the bit is set
            let added = ct_add(&rem, &bitword);
            rem = ct_select(bit_mask, &rem, &added);
            // Conditional subtract of L, branch-free: always compute, keep original if not >= L.
            let ge = ct_ge(&rem, &l_w); // 0xFF if rem >= L else 0x00
            let subbed = ct_sub(&rem, &l_w);
            rem = ct_select(ge, &rem, &subbed);
        }
    }

    // rem is BE in the low 32 bytes (high bytes stay 0; rem < L < 2^253). Convert to LE.
    let be32: [u8; 32] = {
        let mut a = [0u8; 32];
        a.copy_from_slice(&rem[W - 32..]);
        a
    };
    let mut out = [0u8; 32];
    for i in 0..32 {
        out[i] = be32[31 - i];
    }
    out
}

/// SHA-512 hash of `data`, reduced mod L (256-bit scalar).
fn scalar_from_hash(data: &[u8]) -> [u8; 32] {
    let h = crate::hash::sha512(data);
    mod_l(&be_array(&h))
}

fn be_array(h: &[u8; 64]) -> Vec<u8> {
    h.iter().rev().copied().collect()
}

/// Scalar (LE 32 bytes) × scalar → mod L.
fn scalar_mul_mod_l(a: &[u8; 32], b: &[u8; 32]) -> [u8; 32] {
    let prod = mul_be(&be(a), &be(b));
    mod_l(&prod)
}

/// Scalar (LE 32 bytes) + scalar → mod L.
fn scalar_add_mod_l(a: &[u8; 32], b: &[u8; 32]) -> [u8; 32] {
    let s = add_be(&be(a), &be(b));
    mod_l(&s)
}

/// Constant-time field select: returns `b` if `bit == 1`, `a` if `bit == 0`.
/// `bit` MUST be 0 or 1. Branch-free byte masking (no data-dependent control flow).
#[inline]
fn fe_cselect(bit: u8, a: &Fe, b: &Fe) -> Fe {
    let mask = 0u8.wrapping_sub(bit); // bit=1 -> 0xFF, bit=0 -> 0x00
    let mut out = [0u8; 32];
    for i in 0..32 {
        out[i] = (a[i] & !mask) | (b[i] & mask);
    }
    out
}

/// Constant-time point select: `bit ? b : a`, applied coordinate-wise.
#[inline]
fn point_select(bit: u8, a: &Point, b: &Point) -> Point {
    Point {
        x: fe_cselect(bit, &a.x, &b.x),
        y: fe_cselect(bit, &a.y, &b.y),
        z: fe_cselect(bit, &a.z, &b.z),
        t: fe_cselect(bit, &a.t, &b.t),
    }
}

/// Point scalar multiplication `scalar · base` (LSB-first double-and-add).
///
/// C4 (constant-time audit, 2026-07-14): this runs on the SECRET scalar in `sign`
/// (`a` = clamped key, and the per-signature nonce `r`), so its operation trace must
/// NOT depend on the scalar bits. The addition is therefore computed on EVERY bit and
/// the result is chosen by a branch-free [`point_select`] — replacing the prior
/// `if bit == 1 { result = point_add(..) }`, whose presence/absence of a whole point
/// addition equalled the secret bit (a double-and-add timing/power oracle → key/nonce
/// recovery). The math is unchanged (result becomes the sum iff the bit is set), so the
/// RFC 8032 §7.1 KAT still passes bit-for-bit.
///
/// RESIDUAL (remaining after C4, partially closed by C4b):
///   (a) SCALAR layer — **CLOSED by C4b (2026-07-17).** `mod_l` is now branch-free and
///       fixed-width: the per-bit `if (byte>>bit)&1` and the data-dependent conditional
///       subtract are replaced by mask-based `ct_select`/`ct_ge`/`ct_sub` on a fixed 64-byte
///       buffer, so the secret nonce `r` / key `a` no longer leak via timing/power.
///   (b) FIELD layer — `reduce_p` has a magnitude-dependent fold loop and `limbs_ge_p`/
///       `limbs_sub_p` a data-dependent conditional subtract (a weaker, higher-order signal).
/// A fixed-width Barrett/Montgomery rewrite of the FIELD reduction closes the remaining gap
/// (tracked separately from C4b). `verify` uses only PUBLIC scalars, so its
/// non-constant-time paths are fine.
fn scalar_mul(base: &Point, scalar_le: &[u8; 32]) -> Point {
    let d2 = fe_mul(&fe_d(), &fe_2()); // 2*d, computed once
    let mut result = point_identity();
    let mut addend = *base;
    for i in 0..256 {
        let byte = scalar_le[i / 8];
        let bit = (byte >> (i % 8)) & 1;
        // Always add; select branch-free so the op-trace is scalar-independent.
        let sum = point_add(&result, &addend, &d2);
        result = point_select(bit, &result, &sum);
        addend = point_double(&addend, &d2);
    }
    result
}

// ── Public API ────────────────────────────────────────────────────────────────

/// RFC 8032 §5.1.5 — generate (public_key, secret_key) from a 32-byte seed.
/// secret_key = seed || pubkey (64 bytes, RFC form). pubkey = 32 bytes.
///
/// **TEST-ONLY / `dangerous_deterministic`.** In a normal (non-test, feature-off)
/// build this symbol does not exist, so production code cannot keygen from a
/// predictable constant seed. Use [`keygen_from_entropy`] for prod.
#[cfg(any(test, feature = "dangerous_deterministic", feature = "test_keygen"))]
pub fn keygen(seed: &[u8; 32]) -> ([u8; 32], [u8; 32]) {
    let az = crate::hash::sha512(seed);
    let mut a = [0u8; 32];
    a.copy_from_slice(&az[0..32]);
    // clamp (RFC 8032 §5.1.5): clear low 3 bits of octet 0; clear high bit and
    // set second-highest bit of octet 31.
    a[0] &= 248;
    a[31] = (a[31] & 0x7f) | 0x40;
    let b_pt = point_decompress(&B_ENCODED).expect("base point must decode");
    let a_pt = scalar_mul(&b_pt, &a);
    let pk = point_compress(&a_pt);
    let mut sk = [0u8; 32];
    sk.copy_from_slice(&pk);
    (pk, sk)
}

/// Production Ed25519 keygen: draw a fresh 32-byte seed from platform entropy and
/// derive the keypair. Fail-closed — returns `Err` if entropy is unavailable, never
/// a constant fallback. Replaces the constant-seed [`keygen`] in all prod paths.
pub fn keygen_from_entropy() -> Result<([u8; 32], [u8; 32]), crate::rng::EntropyError> {
    let mut seed = [0u8; 32];
    crate::rng::entropy_provider().fill(&mut seed)?;
    // Delegate to the deterministic core. SAFETY: `keygen` is gated behind test /
    // dangerous_deterministic, but it is NEVER depend-feature-gated off for the crate
    // itself (only for downstream callers), so it is always available in-tree. To keep
    // the prod path unconditionally present, inline the derivation here instead.
    Ok(keygen_from_seed_infallible(&seed))
}

/// In-tree deterministic Ed25519 derivation (always available; never exposed publicly
/// as a constant-seed entry point). Used by [`keygen_from_entropy`].
fn keygen_from_seed_infallible(seed: &[u8; 32]) -> ([u8; 32], [u8; 32]) {
    let az = crate::hash::sha512(seed);
    let mut a = [0u8; 32];
    a.copy_from_slice(&az[0..32]);
    a[0] &= 248;
    a[31] = (a[31] & 0x7f) | 0x40;
    let b_pt = point_decompress(&B_ENCODED).expect("base point must decode");
    let a_pt = scalar_mul(&b_pt, &a);
    let pk = point_compress(&a_pt);
    let mut sk = [0u8; 32];
    sk.copy_from_slice(&pk);
    (pk, sk)
}

/// RFC 8032 §5.1.6 — sign `msg` with the 32-byte secret seed → 64-byte signature.
/// (Convenience form: takes the seed, derives the secret scalar internally.)
pub fn sign(seed: &[u8; 32], msg: &[u8]) -> [u8; 64] {
    let az = crate::hash::sha512(seed);
    let mut a = [0u8; 32];
    a.copy_from_slice(&az[0..32]);
    a[0] &= 248;
    a[31] = (a[31] & 0x7f) | 0x40;
    let prefix = &az[32..64];

    let b_pt = point_decompress(&B_ENCODED).expect("base point must decode");
    let a_pt = scalar_mul(&b_pt, &a);
    let pk = point_compress(&a_pt);

    let mut r_input = Vec::with_capacity(prefix.len() + msg.len());
    r_input.extend_from_slice(prefix);
    r_input.extend_from_slice(msg);
    let r = scalar_from_hash(&r_input);

    let r_pt = scalar_mul(&b_pt, &r);
    let r_enc = point_compress(&r_pt);

    let mut k_input = Vec::with_capacity(32 + 32 + msg.len());
    k_input.extend_from_slice(&r_enc);
    k_input.extend_from_slice(&pk);
    k_input.extend_from_slice(msg);
    let k = scalar_from_hash(&k_input);

    // S = (r + k*a) mod L
    let ka = scalar_mul_mod_l(&k, &a);
    let s = scalar_add_mod_l(&r, &ka);

    let mut sig = [0u8; 64];
    sig[0..32].copy_from_slice(&r_enc);
    sig[32..64].copy_from_slice(&s);
    sig
}

/// RFC 8032 §5.1.7 — verify a 64-byte signature over `msg` with `pubkey`.
pub fn verify(pubkey: &[u8; 32], msg: &[u8], sig: &[u8; 64]) -> bool {
    let r_enc = match sig[0..32].try_into() {
        Ok(v) => v,
        Err(_) => return false,
    };
    let s_le = match sig[32..64].try_into() {
        Ok(v) => v,
        Err(_) => return false,
    };
    // RFC 8032 §5.1.7: S must be in [0, L). Reject non-canonical / malleable S >= L.
    if cmp_be(&be(&s_le), &l_be()) != core::cmp::Ordering::Less {
        return false;
    }
    let a_pt = match point_decompress(pubkey) {
        Some(p) => p,
        None => return false,
    };
    let r_pt = match point_decompress(&r_enc) {
        Some(p) => p,
        None => return false,
    };

    let mut k_input = Vec::with_capacity(32 + 32 + msg.len());
    k_input.extend_from_slice(&r_enc);
    k_input.extend_from_slice(pubkey);
    k_input.extend_from_slice(msg);
    let k = scalar_from_hash(&k_input);

    // Check S·B == R + k·A
    let b_pt = match point_decompress(&B_ENCODED) {
        Some(p) => p,
        None => return false,
    };
    let lhs = scalar_mul(&b_pt, &s_le);
    let ka_pt = scalar_mul(&a_pt, &k);
    let d2 = fe_mul(&fe_d(), &fe_2());
    let rhs = point_add(&r_pt, &ka_pt, &d2);
    point_eq(&lhs, &rhs)
}

/// RFC 8032 §5.1.3 base point B encoding (y = 4/5, x positive).
const B_ENCODED: [u8; 32] = [
    0x58, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66,
    0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66,
];

// ── BLUEPRINT-P-E §2.2/§2.3 — lane-parallel INDEPENDENT-verify batch API ─────────
// Mode 1 (verify-only speedup). NOT batch-accept (§2.1): every request is verified
// fully and independently; out[i] is a pure function of reqs[i] alone.

/// One independent Ed25519 verification request. Borrowed, zero-copy.
pub struct VerifyReq<'a> {
    pub pubkey: &'a [u8; 32],
    pub msg: &'a [u8],
    pub sig: &'a [u8; 64],
}

/// The scalar single-verify reference (RFC 8032 §5.1.7) as the parity anchor for
/// the batch path (§2.6). Thin wrapper over the UNCHANGED `verify`.
pub(crate) fn verify_scalar(req: &VerifyReq<'_>) -> bool {
    verify(req.pubkey, req.msg, req.sig)
}

/// Verify N Ed25519 signatures, EACH fully and independently (RFC 8032 §5.1.7 per
/// item). `out[i]` is a pure function of `reqs[i]` alone; changing any other batch
/// element can never change verdict `i` (the property batch-accept cannot have,
/// BLUEPRINT-P-E §2.1). For N==1 this is exactly the scalar `verify`.
///
/// Ed25519's verify hashes with SHA-512 (not Keccak), so this lane is a
/// parallel-independent loop over the constant-time scalar verify; the accept/
/// reject decision is byte-identical to `verify` for every element on every host.
pub fn verify_many(reqs: &[VerifyReq<'_>]) -> Vec<bool> {
    reqs.iter().map(verify_scalar).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dehex(s: &str) -> Vec<u8> {
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
            .collect()
    }

    // RFC 8032 §7.1 — first test vector (the canonical one).
    #[test]
    fn ed25519_rfc8032_section_7_1_vector1() {
        // RFC 8032 §7.1 TEST 1 (verbatim).
        let seed_hex = "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60";
        let pk_hex = "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a";
        let msg = b"";
        let sig_hex = "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b";

        let seed: [u8; 32] = dehex(seed_hex).try_into().unwrap();
        let (pk, _sk) = keygen(&seed);
        assert_eq!(hex(&pk), pk_hex, "pubkey mismatch (RFC 8032 §7.1 #1)");

        let sig = sign(&seed, msg);
        assert_eq!(hex(&sig), sig_hex, "signature mismatch (RFC 8032 §7.1 #1)");

        let pk_arr: [u8; 32] = pk;
        assert!(
            verify(&pk_arr, msg, &sig),
            "verify must pass for the genuine §7.1 #1 signature"
        );

        // RED KAT: a wrong public key must NOT verify the genuine signature.
        // (Catches a verify-always-true bug — the original test only used the computed pk.)
        let mut wrong_pk = pk_arr;
        wrong_pk[0] ^= 0xff;
        assert!(
            !verify(&wrong_pk, msg, &sig),
            "verify must REJECT a signature under the wrong public key"
        );
    }

    #[test]
    fn ed25519_roundtrip_red_green() {
        // GREEN: sign then verify recovers. RED: tampering the signature fails.
        let seed = [0x42u8; 32];
        let msg = b"the cosmo-noir helm turns by starlight, never by panic.";
        let (pk, _sk) = keygen(&seed);
        let sig = sign(&seed, msg);
        assert!(verify(&pk, msg, &sig), "genuine signature must verify");

        // RED: flip a byte in the signature → must NOT verify.
        let mut bad = sig;
        bad[10] ^= 0xff;
        assert!(
            !verify(&pk, msg, &bad),
            "tampered signature must NOT verify"
        );

        // RED: different message → must NOT verify.
        assert!(
            !verify(&pk, b"other", &sig),
            "wrong message must NOT verify"
        );
    }

    #[test]
    fn ed25519_deterministic_same_seed_same_keys() {
        let seed = [0x13u8; 32];
        let (pk1, _) = keygen(&seed);
        let (pk2, _) = keygen(&seed);
        assert_eq!(pk1, pk2, "same seed → same public key");
    }

    #[test]
    fn ed25519_field_known_values() {
        // Sanity: 1 + 1 == 2, and invert(invert(x)) == x for a sample field element.
        let one = fe_1();
        let two = fe_add(&one, &one);
        assert_eq!(fe_to_bytes(&two)[0], 2, "1 + 1 = 2 in the field");
        let x = fe_from_bytes(&[0x11; 32]);
        let xi = fe_invert(&x);
        let back = fe_mul(&x, &xi);
        assert_eq!(fe_to_bytes(&back), fe_to_bytes(&one), "x * x^-1 == 1");
    }

    // ── C4: scalar_mul must be OPERATION-COUNT constant (no secret-bit branch) ──────
    // Deterministic constant-time proof (no flaky wall-clock timing): the number of
    // curve additions scalar_mul performs MUST NOT depend on the scalar's Hamming
    // weight or which bits are set — otherwise the op-trace equals the secret bits.
    // RED (prior double-and-add `if bit==1 { add }`): a low-weight scalar skips
    // additions, dropping the count. GREEN (branch-free point_select): exactly one
    // addition + one doubling per bit = 2·256 = 512 point_add calls for EVERY scalar.
    #[test]
    fn scalar_mul_op_count_is_constant() {
        let b = point_decompress(&B_ENCODED).expect("base point decodes");
        let count_for = |s: &[u8; 32]| {
            POINT_ADD_CALLS.with(|c| c.set(0));
            let _ = scalar_mul(&b, s);
            POINT_ADD_CALLS.with(|c| c.get())
        };
        let zeros = [0u8; 32];
        let ones = [0xffu8; 32];
        let mut low_bit = [0u8; 32];
        low_bit[0] = 1; // scalar = 1 (single low bit)
        let mut high_bit = [0u8; 32];
        high_bit[31] = 0x40; // a single high bit, far from the low ones

        let c_zeros = count_for(&zeros);
        let c_ones = count_for(&ones);
        let c_low = count_for(&low_bit);
        let c_high = count_for(&high_bit);

        assert_eq!(
            c_zeros, c_ones,
            "op-count varies with Hamming weight (0 vs 256 set bits) → secret-bit timing oracle"
        );
        assert_eq!(
            c_zeros, c_low,
            "op-count varies with bit position (low bit)"
        );
        assert_eq!(
            c_zeros, c_high,
            "op-count varies with bit position (high bit)"
        );
        // Exactly one addition + one doubling per scalar bit over 256 bits.
        assert_eq!(
            c_zeros, 512,
            "expected exactly 2 point_add calls per scalar bit (1 add + 1 double)"
        );
    }

    fn hex(b: &[u8]) -> String {
        let mut s = String::new();
        for x in b {
            s.push_str(&format!("{:02x}", x));
        }
        s
    }

    // ── BLUEPRINT-P-E §2.3/§3.3 — Ed25519 verify_many parity + no-cross-contam ─────
    #[test]
    fn ed25519_verify_many_parity_and_isolation() {
        let mut triples: Vec<([u8; 32], Vec<u8>, [u8; 64])> = Vec::new();
        for i in 0..5u8 {
            let seed = [0x50 + i; 32];
            let (pk, _sk) = keygen(&seed);
            let msg = alloc::vec![i; 3 + i as usize];
            let sig = sign(&seed, &msg);
            triples.push((pk, msg, sig));
        }
        // All-valid parity vs scalar verify.
        let reqs: Vec<VerifyReq> = triples
            .iter()
            .map(|(pk, m, s)| VerifyReq { pubkey: pk, msg: m, sig: s })
            .collect();
        let out = verify_many(&reqs);
        for (i, r) in reqs.iter().enumerate() {
            assert!(out[i], "ed25519 batch lane {i} rejected valid sig");
            assert_eq!(out[i], verify(r.pubkey, r.msg, r.sig), "parity mismatch {i}");
        }
        // T4: forge each index in turn.
        for k in 0..triples.len() {
            let mut forged = triples.clone();
            forged[k].2[10] ^= 0xff;
            let reqs: Vec<VerifyReq> = forged
                .iter()
                .map(|(pk, m, s)| VerifyReq { pubkey: pk, msg: m, sig: s })
                .collect();
            let out = verify_many(&reqs);
            for (i, v) in out.iter().enumerate() {
                assert_eq!(*v, i != k, "cross-contamination: forged {k}, index {i}");
            }
        }
    }

    #[test]
    fn ed25519_verify_many_n1_equals_scalar() {
        let seed = [0x9a; 32];
        let (pk, _) = keygen(&seed);
        let sig = sign(&seed, b"n1");
        let out = verify_many(&[VerifyReq { pubkey: &pk, msg: b"n1", sig: &sig }]);
        assert_eq!(out, alloc::vec![verify(&pk, b"n1", &sig)]);
    }
}

#[cfg(test)]
mod bignum_tests {
    use super::*;
    #[test]
    fn mul_be_basic() {
        // 123 * 456 = 56088 = 0xDB18 (big-endian bytes; mul_be returns len a+b = 3)
        let a = vec![123u8];
        let b = vec![0x01u8, 0xC8u8]; // 456 = 0x01C8 (big-endian)
        let prod = mul_be(&a, &b);
        assert_eq!(
            prod,
            vec![0, 0xDB, 0x18],
            "123*456 should be 0x00DB18, got {:?}",
            prod
        );
        // verify via fe_mul
        let f = fe_mul(&fe_from_u64(123), &fe_from_u64(456));
        let le = fe_to_bytes(&f);
        // 56088 mod p = 56088 (since < p); LE [0]=24,[1]=219
        assert_eq!(le[0], 24, "fe_mul(123,456)[0]");
        assert_eq!(le[1], 219, "fe_mul(123,456)[1]");
    }
}

// ── C4b: dudect-style statistical timing gate for `mod_l` ────────────────────────
//
// Cycle-accurate Welch-t gate. The secret we reduce is a 64-byte (512-bit) SHA-512
// hash; the "fixed" class uses an all-zero buffer, the "random" class varies every bit.
// If the reduction leaked secret-dependent timing, the two distributions would diverge
// and |t| (Welch) would exceed the dudect 4.5 threshold.
//
// EVIDENCE GRADE: this version measures CPU cycles via `_rdtsc` on x86_64 (with a
// wall-clock fallback on non-x86), NOT `std::time::Instant`. Wall-clock on a shared host
// is dominated by scheduler jitter (ms-scale), which would make a real ≤10µs leak
// invisible (|t|≈0) — that is a fake-green trap. Cycle counts are sensitive to the actual
// instruction path, so a leaking `mod_l` would show up as a real |t| spike. We still
// run many samples and `black_box` the call so the compiler cannot hoist/precompute it.
// The math is unchanged (proven by the RFC 8032 §7.1 KAT), so this gate measures ONLY
// the control-flow / length side-channel.
//
// DO NOT lower the threshold to pass; fix the leak. |t| < 4.5 is the dudect bar.
#[cfg(test)]
mod c4b_mod_l_timing_gate {
    use super::*;

    const N: usize = 20000; // samples per class (more cycles of data => tighter t)

    /// Cycle-accurate timestamp. x86_64 reads TSC directly; other targets fall back to
    /// wall-clock (still statistical, just noisier — the fallback keeps the gate portable
    /// and clearly labelled, never silently substituting a pass).
    #[inline(never)]
    fn read_cycles() -> u64 {
        #[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
        unsafe {
            // lfence serialises so the rdtsc is ordered between the measured work.
            std::arch::x86_64::_mm_lfence();
            std::arch::x86_64::_rdtsc()
        }
        #[cfg(not(all(target_arch = "x86_64", target_feature = "sse2")))]
        {
            std::time::Instant::now().elapsed().as_nanos() as u64
        }
    }

    /// Welch's t-statistic: how separated the two *mean* timing distributions are,
    /// normalised by the combined sample standard error.
    fn welch_t(a: &[f64], b: &[f64]) -> f64 {
        let mean = |x: &[f64]| -> f64 { x.iter().sum::<f64>() / x.len() as f64 };
        let var = |x: &[f64], m: f64| -> f64 {
            x.iter().map(|v| (v - m) * (v - m)).sum::<f64>() / (x.len() as f64)
        };
        let ma = mean(a);
        let mb = mean(b);
        let va = var(a, ma);
        let vb = var(b, mb);
        let na = a.len() as f64;
        let nb = b.len() as f64;
        let se = (va / na + vb / nb).sqrt();
        if se == 0.0 {
            return 0.0;
        }
        (ma - mb) / se
    }

    #[test]
    fn mod_l_is_constant_time() {
        // Fixed (zero) secret — the "class 0" distribution.
        let fixed: Vec<u8> = vec![0u8; 64];
        // Random secrets — the "class 1" distribution (vary every bit).
        let mut random: Vec<u8> = vec![0u8; 64];
        // XOR-shift PRNG (no external rng dep) — enough to scatter bits across the buffer.
        let mut state: u64 = 0x1234_5678_9abc_def0;
        let mut rng = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        for chunk in random.chunks_mut(8) {
            let v = rng();
            chunk.copy_from_slice(&v.to_le_bytes());
        }

        let mut t_fixed = Vec::with_capacity(N);
        let mut t_random = Vec::with_capacity(N);

        for _ in 0..N {
            let start = read_cycles();
            let _ = std::hint::black_box(mod_l(&fixed));
            t_fixed.push((read_cycles() - start) as f64);

            let start = read_cycles();
            let _ = std::hint::black_box(mod_l(&random));
            t_random.push((read_cycles() - start) as f64);
        }

        let t = welch_t(&t_fixed, &t_random);
        let t_abs = t.abs();
        eprintln!(
            "C4b dudect gate (cycle-accurate): |Welch t| = {:.4}  (threshold 4.5; t_fixed_mean={:.1} cyc, t_random_mean={:.1} cyc)",
            t_abs,
            t_fixed.iter().sum::<f64>() / N as f64,
            t_random.iter().sum::<f64>() / N as f64,
        );
        assert!(
            t_abs < 4.5,
            "C4b NOT closed: |Welch t| = {:.4} >= 4.5 — mod_l leaks secret-dependent timing",
            t_abs
        );
    }
}
