# RISC Zero zkVM Integration — `decide()` as a deterministic guest

## What was built

A RISC Zero guest crate that computes a **deterministic** `decide(state, cmd, ctx, counter) -> [u8;32]`,
mirrored 1:1 in TypeScript, with a verifier stub and RED+GREEN tests.

Files (all under `src/integration/zkvm/`):

| File | Role |
|------|------|
| `guest/Cargo.toml` | risc0 guest crate (dep: `risc0-zkvm` 1.2.6) |
| `guest/src/lib.rs` | `decide()` + `hash32()` — pure arithmetic (wrapping add / rotate / multiply), no IO/time/random. Committed `counter` nonce. |
| `guest/src/main.rs` | guest entrypoint: reads stdin-encoded inputs, commits `digest‖counter‖lens‖payload` to the **journal** via `env::commit_slice`. |
| `decide.ts` | Byte-for-byte TS port of `lib.rs` (`decide`, `hash32`, `buildJournal`). |
| `verify.ts` | Verifier stub: parses journal, recomputes digest, rejects tampering / input mismatch / counter mismatch. |
| `zkvm.test.ts` | RED+GREEN tests. |

## Determinism contract

`decide()` = Davies–Mayer-style 4-pass mixing over a 32-byte accumulator seeded by `counter`.
Identical inputs ⇒ identical 32-byte digest; any input/counter change ⇒ different digest.
This holds in BOTH the Rust guest (`cargo test`) and the TS port (tested).

## Prover status — HONEST

- The RISC Zero toolchain installer (`curl https://risczero.com/install | bash`) was **blocked**
  in this environment, so `rzup` was never installed.
- `rustup target add riscv32im-risc0-zkvm-elf` fails: that target is shipped by rzup, not upstream
  rustup. Therefore the guest **cannot be built for the risc0 riscv target** here, and **no receipt
  can be generated** (no dev-mode prover, no Bonsai creds).
- **No receipt hash is fabricated.** The proof of correctness rests on:
  1. `cargo test` of the guest crate (host target) ⇒ `decide()` deterministic ✅
  2. Native TS `decide()` determinism + tamper/input-mismatch rejection ✅

## To actually prove (elsewhere, with rzup installed)

```bash
rzup install                                    # installs risc0 toolchain + riscv target
cd src/integration/zkvm/guest
cargo build --target riscv32im-risc0-zkvm-elf   # compile guest ELF
# Host methods crate would then: Prover::env(&mut env).write(...).run().unwrap()
#   -> Receipt { journal, seal }; receipt.verify(IMAGE_ID)
```
The committed journal format here (`digest‖counter‖lens‖payload`) is exactly what a host
verifier binds against, so the native `verifyJournal` stub is a faithful stand-in for the
real `Receipt::verify` binding check (only the STARK proof itself is delegated).

## Test results
- `node --test --import tsx src/integration/zkvm/zkvm.test.ts` → **9/9 pass**
  (8 GREEN binding/determinism + 1 GREEN cargo-test; 3 RED tamper/input/counter checks all fail as required)
- `npx tsc --noEmit` → clean. Full `src/*.test.ts` → **225/225 pass**, no regression.
