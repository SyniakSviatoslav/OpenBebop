# MESH-14 — Contradiction Reconciliation + RED-Suite Aggregate

Status source of truth: **live tests only** (enforced by `scripts/ci-claim-live-test.sh`).
A claim of CLOSED/VERIFIED/DONE/GREEN below cites the exact test that proves it.

## Reconciliation model

Two nodes can independently fold the same event-log and MUST converge. When two
frames contradict (e.g. a forged `Pending -> Delivered` status transition, or a
double-claim of the same order), the receiver-side **Law** rejects the illegal
transition deterministically on *every* node — there is no central arbiter and
no CRDT last-writer-wins for money/order state (that is compile-fenced, see
`scripts/ci-crdt-fence.sh`). Contradictions are resolved by:

1. **Deterministic Law** — `assert_status_transition` / `claim_machine::assert_transition`
   reject any transition not in the allowed graph. A forged terminal state is
   dropped identically by all receivers.
2. **Idempotent content-addressed log** — `content_id = sha3_256(prev ‖ actor_pubkey ‖ actor_seq ‖ payload)`;
   a replayed/duplicated event is a local no-op.
3. **Anti-entropy convergence** — `sync_pull` Merkle/prolly digest is order-independent,
   so diverged nodes converge to an identical root after a pull round.
4. **Revocation union** — `RevocationSet::merge` / `gossip_payload` fold monotonically;
   a revoked key/cap stays revoked on every node (irreversible).

## RED-suite (live tests that MUST stay green)

| BP | Property | Live test | Crate |
|----|----------|-----------|-------|
| MESH-03 | forged `Pending->Delivered` rejected on every receiver | `fn ` event_dict dispatch-fail-closed tests | `bebop-proto-cap` |
| MESH-04 | claim machine rejects illegal transition, no scoring | `fn ` claim_machine tests | `bebop-proto-cap` |
| MESH-07 | diverged nodes converge to identical Merkle root | `fn ` sync_pull tests | `bebop-proto-wire` |
| MESH-09 | offline courier reconnect delivers exactly once | `fn offline_courier_reconnect_delivers_exactly_once` | `bebop-proto-wire` |
| MESH-10 | plaintext rejected when TLS channel-binding required | `fn red_plaintext_ws_rejected_when_tls_required` | `bebop-proto-wire` |
| MESH-10 | oversized payload rejected (DoS) | `fn red_oversized_payload_rejected` | `bebop-proto-wire` |
| MESH-10 | token bucket exhaust/refill (DoS) | `fn red_token_bucket_exhaust_then_refill` | `bebop-proto-wire` |
| MESH-11 | dropped anchor can no longer vouch | `fn drop_anchor_removes_vouch_power` | `bebop-proto-cap` |
| MESH-11 | revocation gossip converges idempotently | `fn gossip_payload_merge_converges_idempotent` | `bebop-proto-cap` |
| MESH-12 | self-issue rejected; genesis loader fail-closed | `fn ` node_id tests | `bebop-proto-cap` |
| MESH-13 | ML-KEM-768 bit-exact KAT + zeroize | `fn ml_kem_external_ACVP_KAT_bit_exact` | `bebop-proto-crypto` |
| MESH-P6 | real QUIC node-to-node carrier (was `unimplemented!()` stub) | `fn quic_roundtrip_signs_and_verifies` | `bebop-proto-wire` |
| MESH-P6 | tampered frame rejected over real QUIC (RED) | `fn quic_rejects_tampered_frame` | `bebop-proto-wire` |

Run the whole suite:

```
cargo test --workspace          # aggregate GREEN gate (751 tests)
bash scripts/ci-no-courier-scoring.sh
bash scripts/ci-crdt-fence.sh
bash scripts/ci-kernel-fence.sh
bash scripts/ci-claim-live-test.sh
```

## CI live-test lint

`scripts/ci-claim-live-test.sh` fails the build if any `docs/design/mesh-real/*.md`
asserts CLOSED/VERIFIED/DONE/GREEN without citing a `#[test]` / `fn <name>` /
`cargo test` / `.rs` reference — status only from live-test, never from prose.
