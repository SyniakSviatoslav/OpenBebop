//! End-to-end Воля АНУ breach binding: the REAL `dowiz-kernel` `Hydra` organism
//! emits a `BreachAlert` (locked core, tamper-evident), the mesh node signs it
//! with a REAL Ed25519 + ML-DSA-65 hybrid, and a receiving node verifies the
//! signature AND that the signed bytes are exactly the kernel's own alert.
//!
//! Run with: `cargo test -p bebop-mesh-node --features kernel-rlib --test breach_integration`

use bebop2_core::pq_dsa;
use bebop_mesh_node::breach::{sign, verify};
use dowiz_kernel::hydra::BreachAlert;

#[test]
fn kernel_alert_signed_by_mesh_and_bound_back_to_kernel() {
    // 1. Kernel emits a breach alert (node_id + group_size), no code, tamper-evident.
    let mut node_id = [0u8; 32];
    node_id[0] = 0x42;
    let alert = BreachAlert {
        node_id,
        group_size: 7,
    };
    let alert_bytes = alert.to_bytes();
    assert_eq!(alert_bytes.len(), 40, "kernel alert wire layout is 40 bytes");

    // 2. Mesh node signs it with a real hybrid key (Ed25519 + ML-DSA-65).
    let ed = [0x10u8; 32];
    let pq_seed = [0x11u8; 32];
    let (pq_pk, pq_sk) = pq_dsa::keygen_derivable(&pq_seed);
    let frame = sign(
        &alert_bytes,
        &ed,
        &pq_sk.bytes.try_into().unwrap(),
        &pq_pk.bytes,
        [3u8; 8],
        9_999,
        &[0u8; 32],
    );

    // 3. Receiving node verifies the hybrid signature AND that the payload is a
    //    valid kernel BreachAlert (feature = kernel-rlib content-binding).
    let recovered = verify(&frame).expect("mesh-signed kernel breach verifies");
    assert_eq!(recovered, alert_bytes, "recovered payload is the kernel alert");

    // 4. The recovered alert is byte-for-byte what the kernel emitted — the
    //    transport signature is now bound to a real, locked-core alert, not a
    //    forged one.
    let back = BreachAlert::from_bytes(&recovered).expect("decodes as kernel alert");
    assert_eq!(back, alert, "round-trips through kernel BreachAlert");
    assert_eq!(back.witness_event_id(), alert.witness_event_id(), "kernel content-binding preserved");
}

#[test]
fn kernel_alert_tamper_after_sign_is_rejected() {
    let alert = BreachAlert {
        node_id: [0xABu8; 32],
        group_size: 3,
    };
    let alert_bytes = alert.to_bytes();

    let ed = [0x20u8; 32];
    let (pq_pk, pq_sk) = pq_dsa::keygen_derivable(&[0x21u8; 32]);
    let mut frame = sign(
        &alert_bytes,
        &ed,
        &pq_sk.bytes.try_into().unwrap(),
        &pq_pk.bytes,
        [4u8; 8],
        9_999,
        &[0u8; 32],
    );

    // Attacker flips a byte in the signed payload after signing.
    frame.payload[31] ^= 0xFF;

    assert!(
        verify(&frame).is_err(),
        "tampered kernel alert payload must fail hybrid verify"
    );
}
