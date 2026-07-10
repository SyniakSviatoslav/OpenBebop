# Verification

Every claim in Bebop is **falsifiable** (Verified-by-Math): an assertion that goes RED on bad input,
shipped alongside the GREEN.

## Counts (1.0.0, 2026-07-09 — native Rust, no TypeScript runtime)
- **Rust kernel (`bebop-core` / `rust-core`):** 16 tests (`cargo test -p bebop-core`), wasm32 build clean.
- **Rust agent (`bebop`):** 63 tests (`cargo test -p bebop`), 0 fail.
- **Total:** **79 Rust tests** green — this is the number README/AGENTS claim, and it is real.
- **Doc-gate:** `node scripts/verify-doc-claims.mjs` → all doc claims backed by live proof.
- **Falsifiable-proof:** `node scripts/guardrail-falsifiable-proof.mjs` → 95/95 `#[test]` fn bodies have a non-tautological assertion (RED case exists).
- **Lint/format:** `cargo fmt --check` + `cargo clippy` gate the native path.

## Principles
- **Constant Doubt:** no verification, no statement.
- **Better less than sorry:** never state what isn't fact-checked.
- **Ground truth over proxy:** deterministic math truth may delete rotten processes.
- **Red-line globs** (auth / money / RLS / migrations / bulk-edit) need a human gate before change.

## Honest gaps (not silent losses)
Deleting the TypeScript layer retired ~30 analytic behaviors that were TS-only and not yet in Rust:
N1–N8 anomaly/cycle/liveness, T3MP3ST redteam, Portkey gateway, PDDL `logicalCot`, module registry,
audit, optical search. The **ML-KEM/ML-DSA post-quantum identity** is *no longer* a gap — it is
wired into the native Rust core as a hybrid vault (`src/vault.rs`: ML-KEM-768 ⊕ X25519, ML-DSA-65
⊕ Ed25519, Argon2id, XChaCha20-Poly1305) and covered by `vault::tests` + `doc_claims`. The remaining
items above are documented here and in the doc-gate rather than faked as "covered".
