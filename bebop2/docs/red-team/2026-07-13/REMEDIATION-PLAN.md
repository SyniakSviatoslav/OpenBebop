# Remediation Plan — bebop2 C1 / PQ (VERIFIED 2026-07-13)

Companion to `B1–B4` + dowiz `MASTER-SYNTHESIS.md`. Independently re-verified by
reading the live source and running `cargo test -p bebop-proto-cap` (31 passed).

## P0 · C1 — AnchorRoster dead code (CONFIRMED)
- `verify_chain` / `AnchorRoster` / `subject_in_roster` appear ONLY in
  `proto-cap/src/roster.rs` (test block + `lib.rs:43` re-export). NOT called from
  `proto-cap/src/hybrid_gate.rs:55` (`HybridGate::check`) or
  `proto-wire/src/wss_transport.rs:153` (`recv`, hardcodes `now=0`).
- Fix sequence:
  1. Add `delegation_chain: Vec<Delegation>` to `SignedFrame`.
  2. `HybridGate::check(&self, roster: &AnchorRoster, chain: &[Delegation], frame, now)`
     calls `verify_chain(roster, chain, &frame.capability, now)` and rejects
     `UnknownIssuer` / `ScopeViolation` BEFORE returning the frame.
  3. `WssTransport` owns a roster; `recv` passes a REAL clock, not `0`.
  4. Nonce store connection-independent + persistent (replay is per-conn today).
  5. RED test: self-signed frame over the REAL WSS carrier → rejected.
- Gate C1 CLOSED only when `verify_chain` is on the path + red WSS test passes.

## P0 · PQ not in force (CONFIRMED)
- `HybridGate::check` PQ leg = TODO (`HybridIncomplete`); `ClassicalUntilPqAudit`
  accepts classical-only. `ML-DSA-65` real (acvp_tests.rs 60/60). `ML-KEM-768`
  has NO vendored FIPS-203 KAT vectors (only dual-impl round-trip) → interop unproven.
- Fix:
  1. `sign_pq`/`verify_pq` pack/unpack; enable ML-DSA leg; require under `RequireBoth`.
  2. Add official FIPS-203 ML-KEM-768 KAT vectors; RED↔GREEN interop test
     (mirror `pq_dsa/acvp_tests.rs`).

## P2 · Wire spec (CONFIRMED)
- `wss_transport.rs:144` decodes via `serde_json`. Use the existing TLV codec
  (`tlv.rs`, already green: `signing_domain_is_tlv_not_serde`) and publish a spec.

## P2 · Process
- No security gate CLOSED on isolated unit-green alone; add an E2E adversarial
  CI lane against the real carrier.
