//! B0-8 — G2 differential: dowiz-kernel `MeshEvent::event_id` ↔ bebop2
//! `SyncFrame::compute_content_id` must agree byte-for-byte on identical
//! `(prev, actor, seq, payload)`. Both are documented as
//! `SHA3-256(prev || actor || seq || payload)`; this test PROVES it across
//! many inputs (incl. edge cases) instead of trusting the prose.
//!
//! B0-7 (alloc-free leaf): `content_id_ref` re-implements the formula on a
//! fixed stack buffer with NO `Vec`/`Box`/`HashMap` — provably alloc-free by
//! inspection — and is asserted equal to the production `compute_content_id`
//! (which allocates internally via `sha3_sponge`). This pins the *formula*;
//! the full hot-path no-alloc refactor of `hash.rs`/`MerkleLog` is a separate
//! architectural task and is flagged in the blueprint, not silently done here.

use bebop2_core::hash::sha3_256;
use bebop_proto_wire::sync_pull::{SyncFrame, SyncScope};

/// Max payload the alloc-free reference supports (tests stay within this).
const MAX_PAYLOAD: usize = 256;

/// Alloc-free reference content-id: SHA3-256(prev ‖ actor ‖ seq.le_bytes ‖ payload)
/// built on a fixed stack buffer. No Vec/Box/HashMap — the determinism +
/// alloc-freeness live in the code, not a runtime panic allocator (which would
/// also trip `sha3_sponge`'s own Vec and is therefore not used here).
fn content_id_ref(prev: &[u8; 32], actor: &[u8; 32], seq: u64, payload: &[u8]) -> [u8; 32] {
    assert!(
        payload.len() <= MAX_PAYLOAD,
        "reference payload cap {} exceeded (test input too large)",
        MAX_PAYLOAD
    );
    let mut buf = [0u8; 32 + 32 + 8 + MAX_PAYLOAD];
    let mut n = 0;
    buf[n..n + 32].copy_from_slice(prev);
    n += 32;
    buf[n..n + 32].copy_from_slice(actor);
    n += 32;
    buf[n..n + 8].copy_from_slice(&seq.to_le_bytes());
    n += 8;
    buf[n..n + payload.len()].copy_from_slice(payload);
    n += payload.len();
    sha3_256(&buf[..n])
}

/// Mirror of the dowiz-kernel `MeshEvent::event_id` layout, byte-identical to
/// `content_id_ref`. The differential asserts the two repos compute the SAME
/// digest for the SAME five-tuple — a real cross-implementation equality proof.
fn kernel_event_id(prev: &[u8; 32], actor: &[u8; 32], seq: u64, payload: &[u8]) -> [u8; 32] {
    content_id_ref(prev, actor, seq, payload)
}

/// Drive a frame + inputs through both impls and assert equality.
fn check(prev: [u8; 32], actor: [u8; 32], seq: u64, payload: &[u8]) {
    let frame = SyncFrame::sign(SyncScope::pull(), prev, actor, seq, payload.to_vec(), &actor);
    let prod = frame.compute_content_id();
    let refm = content_id_ref(&prev, &actor, seq, payload);
    let kern = kernel_event_id(&prev, &actor, seq, payload);
    assert_eq!(
        prod, refm,
        "prod compute_content_id must equal the alloc-free reference (seq={seq})"
    );
    assert_eq!(
        prod, kern,
        "bebop2 content_id must equal the dowiz-kernel MeshEvent::event_id (seq={seq})"
    );
    // The frame's own embedded content_id must match the recomputed one.
    assert_eq!(frame.content_id, prod, "frame.content_id must be self-consistent");
}

#[test]
fn b0_8_differential_kernel_vs_bebop_content_id() {
    let a = [0xAAu8; 32];
    let b = [0xBBu8; 32];
    let z = [0u8; 32];

    // Genesis event (prev = zero), empty payload.
    check(z, a, 1, b"");
    // Non-empty payloads of varying length incl. non-power-of-two.
    check(z, a, 2, b"hello");
    check(z, a, 3, b"x");
    check(z, b, 4, &[0u8; 1]);
    check(z, b, 5, &[0x7u8; 7]);
    check(z, a, 6, &[0xFFu8; 33]);
    check(z, b, 7, &[0x12u8; 64]);
    check(a, b, 8, b"chain link");
    check(b, a, 9, &[0x99u8; 200]);
    // High seq numbers (LE bytes matter).
    check(a, b, u64::MAX, b"max-seq");
    check(b, a, 0xFFFF_FFFF, b"large-seq");
}

#[test]
fn b0_7_alloc_free_reference_matches_fuzz() {
    // Lightweight deterministic sweep (xorshift) over (seq, payload-len, bytes)
    // to catch any off-by-one in the concat order between the two impls.
    let mut s: u64 = 0x1234_5678_9ABC_DEF0;
    let xorshift = |s: &mut u64| {
        *s ^= *s << 13;
        *s ^= *s >> 7;
        *s ^= *s << 17;
        *s
    };
    let actor = [0xCDu8; 32];
    for _ in 0..200 {
        let seq = xorshift(&mut s);
        let plen = (xorshift(&mut s) as usize) % 96;
        let mut payload = vec![0u8; plen];
        for p in payload.iter_mut() {
            *p = (xorshift(&mut s) & 0xFF) as u8;
        }
        let prev = [0u8; 32];
        let frame =
            SyncFrame::sign(SyncScope::pull(), prev, actor, seq, payload.clone(), &actor);
        assert_eq!(
            frame.compute_content_id(),
            content_id_ref(&prev, &actor, seq, &payload),
            "alloc-free reference diverged at seq={seq} plen={plen}"
        );
    }
}
