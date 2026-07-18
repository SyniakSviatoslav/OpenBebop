# Changelog

All notable changes to the bebop2 protocol/agent are documented here.
Format: [Keep a Changelog](https://keepachangelog.com/) + CalVer `YYYY.MM.PATCH`.

## [2026.07.0] — 2026-07-18

### Added
- `MESH_WIRE_VERSION` in-code wire version constant (bebop2/core/src/lib.rs).
- Lyapunov NaN/PSD fail-closed guard (V1 #2): non-finite / non-PSD operator state
  now surfaces as a fault instead of reading as healthy.
- Mesh hardening: TLS1.3-only rustls transport, replay-nonce admission, idle/DoS
  caps (MESH-10), agentic-mesh secret-leak (RefSigner) removed.
- `IssuanceBudget` Sybil cap (max_per_epoch) on node identity issuance (A5).

### Fixed
- Spectral Lyapunov primitive fail-open on NaN.

### Verification
- Full KAT 275 passed (ML-DSA-65 / ML-KEM-768 / Ed25519 / X25519 / SHA / KDF / VSA).
- C4b dudect Welch-t gate GREEN on signing path (cycle-accurate); sensitivity
  mutant proves the gate is real.
