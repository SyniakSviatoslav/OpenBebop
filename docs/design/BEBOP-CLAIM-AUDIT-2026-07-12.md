# Bebop claim audit — 2026-07-12 (truth-hook, pre-Phase-1)

Operator directive: keep BOTH the **bebop protocol** and the **bebop agent** in the
repo; write quality docs; audit every claim for logical consistency + truthful
implementation BEFORE building. Stated = mathematically + code-verified; if unsure,
check and state precisely.

## Method
Every marketing/status claim in `README.md` / `AGENTS.md` / `bebop2/README.md` was
checked against live code and `cargo test`. Verdict legend:
- **TRUE** — backed by code + test.
- **TRUE (path fix)** — correct fact, wrong file path in the doc.
- **OVERSTATED** — partial truth, wording implies more than code does.
- **STALE** — was true/false, now reversed by Phase-0 integration.
- **FALSE** — not backed.

## Two distinct artifacts (the root conflation)
1. **bebop** — the live Rust coding-agent CLI.
   - Crate: `crates/bebop/` (agentic_git, copilot, tui, vault, mcp, knowledge, …).
   - Entry: `bin/bebop` → `cargo run -p bebop`. Build: `package.json` `"build": "cargo build --release -p bebop"`.
   - PQ identity uses **RustCrypto crates** (`ml-dsa`/`ml-kem` — vendored) + `rust-core`
     (the dependency-free deterministic math core: field_*, vsa, kalman, …).
   - NOT from-scratch crypto: it reuses audited RustCrypto PQ crates.
2. **bebop2** — the from-scratch, zero-dependency, FIPS 203/204 **protocol**.
   - Crates: `bebop2/core` (ML-KEM-768 + ML-DSA-65 from scratch, KAT-verified),
     `bebop2/proto-cap` (capability/roster/TLV/signed-frame), `bebop2/proto-wire`
     (rustls transport), `bebop2/proto-crypto`.
   - `no_std + alloc`, empty-import wasm, ACVP-verified (60/60 ML-DSA-65).
   - This is "the protocol" per operator directive.

The README narrates the **agent** but cites paths/claims that belong to neither
artifact cleanly, and `AGENTS.md §3` says "bebop is PARKED" — contradicting both
live repos and the operator's keep-both directive.

## Claim-by-claim

### README.md (the agent narrative)
| # | Claim | Evidence | Verdict |
|---|-------|----------|---------|
| 1 | "local-first coding-agent CLI with deterministic Rust/WASM guard kernel" | `crates/bebop/` real; `rust-core/bebop_core.wasm` built; `bin/bebop` runs it | TRUE |
| 2 | "Node identity: **ML-KEM-768 ⊕ X25519** KEM, **ML-DSA-65 ⊕ Ed25519** sign, **Argon2id** KDF, **XChaCha20-Poly1305** AEAD (`src/vault.rs`, pure Rust)" | `crates/bebop/src/vault.rs:4-7,21-42` implements exactly these; uses `ml_dsa`/`ml_kem`/`argon2`/`chacha20poly1305` crates | **TRUE (path + overstate)**: file is `crates/bebop/src/vault.rs`, NOT `src/vault.rs`; AEAD/KDF are RustCrypto crates, not hand-rolled → not "pure Rust" from-scratch |
| 3 | "every node gets a hybrid post-quantum self-certifying identity" | `crates/bebop/src/vault.rs` gen; `bebop node` command (`cli.rs:140`) | TRUE |
| 4 | "cargo test — 499 Rust tests" | `cargo test --workspace` → 499 pass / 0 fail (independently re-run) | TRUE |
| 5 | "native runtime Rust/WASM, no TypeScript in live path" | `package.json` build = cargo; 59 `.rs`, **0** `.ts` (excl. node_modules); TS archived | TRUE |
| 6 | "field-as-cost-surface … deterministic graph-PDE field" | `rust-core` field_* + `crates/bebop/src/field.rs`; `docs/diagrams/field-sim-explainer.svg` exists | TRUE |
| 7 | "Multipilot fans N specialist pilots + field arbiter may veto" | `crates/bebop/src/*` multipilot + `field.rs` veto; falsifiable test present | TRUE |
| 8 | "encrypted-at-rest vault … wrong passphrase / tampered blob / tampered id fail closed" | `vault.rs` tested (wrong-pass test) | TRUE |
| 9 | "Zenoh mesh … INTERFACE (single-node: no-op)" | `crates/bebop` zenoh behind feature flag | TRUE (honestly scoped) |

### AGENTS.md
| # | Claim | Evidence | Verdict |
|---|-------|----------|---------|
| 1 | "bebop is PARKED as a protocol until dowiz carries it" (§3.3) | Both `crates/bebop` (agent) and `bebop2` (protocol) are live + committed; operator directive: keep BOTH | **STALE / CONTRADICTORY** — must be reversed |
| 2 | "cargo test — 499 Rust tests" | verified | TRUE |
| 3 | "3-model review gate; builder≠reviewer≠overlap" | `.git/hooks/pre-commit` + `scripts/three-model-review.sh` | TRUE |

### bebop2/README.md + red-team
| # | Claim | Evidence | Verdict |
|---|-------|----------|---------|
| 1 | "from-scratch, zero-dependency, post-quantum" | `bebop2/core/Cargo.toml` no deps; `no_std` | TRUE |
| 2 | Red-team §2 "proto-wire pulls OpenSSL/native-tls" | Phase-0 WS-6 replaced with rustls; `cargo tree -i openssl-sys` → no match | **STALE (now fixed)** |
| 3 | Red-team §4A "signatures over non-canonical JSON" | WS-2 TLV codec; `Delegation::canonical_bytes` also TLV (fixed this session) | **STALE (now fixed)** |
| 4 | Red-team §3A "self-issued capabilities" | WS-5 AnchorRoster + UCAN-subset delegation | **STALE (now fixed)** |
| 5 | Red-team "neither PQ primitive has external KAT" | WS-F2: ML-DSA-65 60/60 NIST ACVP byte-exact | **STALE (now fixed)** |
| 6 | "bebop2 is a protocol-in-name (no byte wire spec)" | `proto-cap` now has canonical TLV; `proto-wire` rustls transport; but **no standalone wire-spec doc + no interop vectors yet** | **PARTIAL** — code canonical, spec doc pending (Phase 1 work) |

## Truthful status to document
- **Agent (bebop):** LIVE, Rust, hybrid PQ identity via RustCrypto crates + rust-core math. Not from-scratch crypto.
- **Protocol (bebop2):** LIVE, from-scratch FIPS 203/204, ACVP-verified, OpenSSL gone, canonical TLV, anchored roster. **Protocol-in-name → protocol-in-code**, but a normative wire-spec document + interop vectors are still TODO (Phase 1).
- **AGENTS.md §3 "PARKED":** delete — contradicted by reality and operator directive.
- **README path `src/vault.rs`:** correct to `crates/bebop/src/vault.rs`; drop "pure Rust" for the AEAD/KDF (they're RustCrypto crates); keep "hybrid post-quantum" (true).

## Out of scope for this audit (Phase 1–5 next)
Wire-spec doc, interop vectors, wasm32 CI gate hardening, entropy source (WS-1),
and the remaining blueprint phases.
