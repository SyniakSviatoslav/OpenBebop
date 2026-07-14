# Bleeding-edge EV re-analysis — on real reverse-engineering findings (2026-07-08)

Authoritative prior: `docs/design/bleeding-edge-EV-2026-07-08.md` (research-only, before integration).
This doc RE-RANKS EV based on what each tool *actually did* once reverse-engineered into the
bebop Sovereign Node and run under `node --test` / `cargo test`. Philosophy filter:
**offline · deterministic (Rust/WASM) · Verified-by-Math**. Findings below are evidence, not vibes.

## What actually integrated (and how cleanly)

| Tool | Reverse-engineered into | Determinism | Test status | EV verdict vs prior |
|------|--------------------------|-------------|-------------|---------------------|
| **Optical FFT (SVETlANNa+Meep)** | `optical/optic.ts` | deterministic (numpy REF) | 6/6 GREEN+RED | **UP** — cleanest physics primitive; REF matched `numpy.fft.fft2` exactly once FFT sign fixed |
| **RISC Zero zkVM** | `zkvm/{decide.ts,guest}` | deterministic digest | Rust guest `cargo test` pass; TS verifier pass | **SAME** — byte-parity hash works, but STARK proving still env-gated (honest gap) |
| **Active Inference (pymdp+RxInfer)** | `active-inference/ai.ts` | deterministic | 8/8 GREEN+RED | **UP** — FEP math slotted in 1:1; the `inferStates` sign bug proves value of RED tests |
| **TigerBeetle** | `tigerbeetle/ledger.ts` | deterministic | 6/6 GREEN+RED | **SAME** — ledger math trivial + correct; the win is the *invariant* (conservation), not novelty |
| **Eclipse Zenoh** | `zenoh/transport.ts` | deterministic twin | 6/6 GREEN+RED | **DOWN** — real client is a native binary; the value is transport semantics, easily a swap later |
| **FinalSpark (wetware)** | `wetware/finalspark.ts` | **NON-deterministic** | 5/5 (stub only) | **DOWN hard** — bio substrate cannot satisfy Verified-by-Math; only the LIF *stub* is testable |

## Key correction vs the research-only prior
The prior ranked FinalSpark as "excluded by red-line"; with red-lines lifted it was integrated,
and integration *proved* it is the lowest-EV item: a living organoid is definitionally
non-deterministic, so it can never be a falsifiable-proof surface. It stays an **out-of-band
signal source**, never the deterministic core. Red-lines were not the right reason to exclude it —
*determinism* is. That is a cleaner, philosophy-grounded exclusion.

## Re-ranked EV (Tier 1 → Tier 3), post-integration

**Tier 1 — integrate now, high EV, philosophy-fit:**
1. **Optical compute (SVETlANNa/Meep)** — a *new compute primitive* (matrix multiply via diffraction)
   that is both deterministic (FFT) and physically grounded. Highest surprise-EV: it is real math,
   not a wrapper. Wire as an optional `field.ts`/search accelerator.
2. **Active Inference (pymdp/RxInfer)** — the FEP policy selector is the missing *reasoning* primitive
   the agent loop needs; it is pure variational math and slots next to `governor.ts`.
3. **RISC Zero zkVM** — the only tool that gives a *provable money boundary*; keep `decide` as the
   authoritative event→journal, invest in the prover path when CI allows.

**Tier 2 — integrate, supporting role:**
4. **TigerBeetle** — operational ledger; correct and cheap, but it is a *substrate swap*, not a
   capability gain. Use behind the zkVM boundary.

**Tier 3 — defer / out-of-band:**
5. **Zenoh** — transport; valuable at multi-node scale, not at single-node. Swap the `zenoh/transport.ts`
   twin for the real client when a mesh is needed.
6. **FinalSpark (wetware)** — keep as a research signal adapter only; never in the deterministic path.

## Net
Reverse-engineering moved the needle for **optical** and **active-inference** (genuine new
primitives for the Sovereign Node), confirmed **zkVM + TigerBeetle** as the money backbone, and
demoted **wetware** to out-of-band on *determinism grounds* (not red-lines). Three RED-test
catches during integration (optical FFT sign, zkvm hash injectivity, FEP `inferStates` sign)
are the concrete evidence that this pass earned its keep.

## Verified
- `node --test --import tsx 'src/**/*.test.ts'` → **265 pass / 0 fail**
- `npx tsc --noEmit` → exit 0
- `cargo test` on `src/integration/zkvm/guest` → deterministic test pass
