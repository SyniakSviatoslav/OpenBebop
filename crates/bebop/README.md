# `bebop` — the agent (coding-agent CLI)

> **This is the AGENT**, `crates/bebop/`, run via `bin/bebop`. The from-scratch,
> zero-dependency **protocol** lives separately in `bebop2/` (ML-KEM-768 +
> ML-DSA-65 hand-rolled, ACVP-verified). Both are maintained in this repo;
> see `docs/design/BEBOP-CLAIM-AUDIT-2026-07-12.md` for the per-claim audit.
> This file documents only `crates/bebop/`. Claims here are grounded in the
> source files named inline (file:line); run `cargo test -p bebop` to verify.

`bebop` is a local-first coding-agent CLI with its **own deterministic Rust/WASM
guard kernel**, a **living (VSA) memory**, and a **post-quantum node identity**.
It drives any agent you already use (Claude Code, Codex, OpenCode, Aider, Goose)
behind one auditable, free-by-default, offline control plane, and self-evolves via
a "freestyle bebop soul" loop.

- **License:** AGPL-3.0 (`Cargo.toml`). Version `0.4.0`.
- **Native + WASM:** `native` feature = TUI binary (`src/main.rs`); `wasm` =
  bindgen core for the web build pipeline.
- **Deterministic math:** reuses `rust-core/` (spectral/Kalman/Lyapunov/FFT, VSA)
  for the guard kernel — see `rust-core/` and `src/mathx.rs`, `src/field.rs`.

## What's actually built (source-grounded)

The crate compiles to a real binary (`bin/bebop`). Module map (all under
`src/`, each with `#[cfg(test)]` RED+GREEN coverage):

| Layer | Modules | Ground |
|-------|---------|--------|
| Guard OS (deny-on-red) | `guard.rs`, `governor.rs`, `sandbox.rs`, `redteam.rs`, `audit.rs` | `guard.rs` `Gate::admit` |
| Memory (VSA / recall graph) | `memory.rs`, `recall_graph.rs`, `knowledge.rs`, `pod.rs` | `memory.rs` |
| Active-inference governor | `active_inference.rs`, `coherence.rs`, `stabilizer.rs`, `svc.rs` | `governor.rs` |
| Post-quantum vault | `vault.rs` (ML-KEM-768 + ML-DSA-65 via RustCrypto, x25519/ed25519 hybrid, argon2id KDF, chacha20poly1305 AEAD) | `Cargo.toml` `[dependencies]` |
| Mesh transport | `zenoh.rs` (RustCrypto-based PQ, operator-preferred mesh) | `zenoh.rs` |
| zkVM admission journal | `zkvm.rs` (`decide()` tamper-evident digest over `(state, commandHash, seq)`) | `zkvm.rs` |
| Dispatch / multi-pilot | `router.rs`, `multipilot.rs`, `copilot.rs`, `mcp.rs`, `execusion.rs` | `router.rs` |
| TUI / soul | `tui.rs`, `launch.rs`, `radio.rs`, `mission.rs`, `customize.rs`, `outfit.rs`, `narration/` | `tui.rs` |
| Doc-claim honesty | `doc_claims.rs` (the in-repo claim auditor) | `doc_claims.rs` |

> **Honesty note (per LOGIC-LAWS.md §8):** each of the above is *implemented and
> cargo-tested* in this repo. Side-channel resistance and "self-evolving soul"
> are documented as *operational behaviours backed by tests*, not as formally
> proven properties. Where a doc claim lacks a test/proof, it is escalated in
> `docs/design/ESCALATIONS.md` (see `scripts/logic-gate.mjs`).

## Crypto posture (verified, grounded)

- **KEM:** `ml-kem` 0.3 (FIPS 203, ML-KEM-768) — `Cargo.toml`.
- **Signatures:** `ml-dsa` 0.1 (FIPS 204, ML-DSA-65) — `Cargo.toml`.
- **Classical hybrid fallback:** `x25519-dalek` (DH) + `ed25519-dalek` (sig),
  composed with the PQ primitives for transition safety.
- **KDF / AEAD:** `argon2` (argon2id) + `chacha20poly1305`.
- **Secrets:** `zeroize` on drop (`vault.rs`).
- **Keygen entropy:** `getrandom` 0.3 (OS RNG) — `Cargo.toml`.

> The *protocol's* PQ layer (`bebop2/`) is the **from-scratch, KAT-gated**
> implementation. The *agent's* PQ layer (`crates/bebop/`) uses **audited
> RustCrypto crates** — a deliberate split: the agent ships now on vetted deps;
> the protocol is the hand-rolled, bit-exact reference. Both satisfy FIPS 203/204.

## Quick start

```bash
# build + run the TUI agent
cargo run -p bebop --bin bebop

# run the in-repo test suite (RED+GREEN, falsifiable)
cargo test -p bebop

# audit every documentation claim against live code
node scripts/verify-doc-claims.mjs
node scripts/logic-gate.mjs        # Global Logic Laws truth gate
```

## Governance

- **Truth gates:** `scripts/verify-doc-claims.mjs` (doc count vs `cargo test`)
  and `scripts/logic-gate.mjs` (LOGIC-LAWS.md — identity, non-contradiction,
  sufficient reason). Both run in `.git/hooks/pre-commit`.
- **Three-model review:** `scripts/three-model-review.sh` (builder ≠ reviewer ≠
  overlap). CI may set `CI_THREE_MODEL_REVIEW=allow` when it runs its own job.
- **Constitution (LOGIC-LAWS.md §6):** both `bebop2/` (protocol) and
  `crates/bebop/` (agent) MUST remain in the repo. Deleting either is a hard
  pre-commit refusal.

See `docs/README.md` for the full documentation map.
