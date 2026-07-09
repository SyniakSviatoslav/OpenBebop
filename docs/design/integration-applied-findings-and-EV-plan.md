# bebop integration — applied findings & max-EV usage plan

Date: 2026-07-08 · Author: Hermes agent · Branch: `backup-wip-2026-07-08`

This doc records (Part A) what was actually wired from the reverse-engineering pass, and (Part B)
where each integrated tool earns maximum expected value (EV) for bebop specifically — not in the
abstract, but against the real seams in `src/kernel.ts`, `src/loop.ts`, `src/field.ts`, `src/token.ts`.

---

## PART A — Findings applied (already landed, all RED+GREEN proven)

Six modules under `src/integration/`, each grounded in a real external ground truth:

| Module | Ground truth used | Wiring shim added | Tests |
|--------|-------------------|-------------------|-------|
| tigerbeetle | in-process double-entry + conservation (port of TB invariants) | `kernel-ledger.ts` → `moneyTransferChecker` (kernel `Checker`) + `applyMoneyTransfer` | 6 + 5 |
| zkvm | RISC Zero `decide()` (FNV digest, byte-parity with Rust guest) | `kernel-journal.ts` → `journalize`/`verifyJournal` over kernel state+commandHash+counter | 9 + 5 |
| active-inference | **real pymdp numbers** (posterior=[1,0], G=[-2.027,-0.227], chosen=1) | `loop-advisor.ts` → `adviseLoop(belief, preferDone)` FEP policy advisor | 10 + 5 |
| optical | **real SVETlANNa forward pass** (power conserved, thin-lens TF=exp(-i·k·r²/2f)) | `field-recall.ts` → `opticalRecall` field-search accelerator over VSA candidates | 6 + 3 |
| zenoh | port of Zenoh core semantics (@eclipse-zenoh/zenoh-ts@1.9.0) | `transport.ts` (no native dep) | 6 |
| wetware/finalspark | LIF stub + bio-safe 50 mV invariant | `finalspark.ts` (deterministic stub; remote adapter point) | 5 |

Full suite: **285 pass / 0 fail**, `tsc --noEmit` exit 0.

Bugs caught by grounding to external truth (Verified-by-Math doing its job):
- optical FFT sign + Parseval N² scaling (kernel was wrong vs numpy)
- zkVM `hash32` non-injective (collided distinct inputs) → FNV-spread fix
- FEP `inferStates` sign + C log-domain + G-maximize (3 bugs vs pymdp ground truth)
- loop-advisor `B` transition matrix was transposed (columns didn't sum to 1) → FEP degenerate

---

## PART B — Max-EV usage plan (where each tool pays for itself)

Principle applied: **a tool earns its keep only where it sits on a real seam and removes a real
weakness.** EV ranking from `docs/design/bleeding-edge-EV-REANALYSIS-2026-07-08.md`, re-derived from
the actual findings (determinism + physical correctness beat raw capability).

### 1. zkVM `decide` → KERNEL JOURNAL  (EV: HIGHEST — money/verify boundary)
Seam: `kernel.ts` `decide()` already emits `commandHash`; the kernel is content-addressed but does
not *journal* transitions. Wire `journalize(state, commandHash, counter)` into the loop's
`applyCommand` step (or a kernel `Checker` that records a digest per transition).
- WHY max EV: it is the ONE tamper-evident audit trail over sovereign decisions, with zero RNG,
  deterministic, and verifiable (RED: tampered/ mismatched input fails `verifyJournal`).
- HONEST LIMIT: runs the native TS port; real STARK *receipt* needs the risc0 prover (blocked in
  this env). Ship the digest now; gate the cryptographic receipt behind `cfg.zkReceipt` when the
  toolchain is available. Do NOT claim a proof we cannot produce.

### 2. TigerBeetle → MONEY BOUNDARY  (EV: HIGH — but only if bebop moves money)
Seam: `kernel.ts` "above" `Checker` + shell-side apply; `token.ts` `Ledger` is the current accounting.
Wire `moneyTransferChecker()` as a kernel `Checker` for money-tagged Commands and `applyMoneyTransfer`
at shell apply-time.
- WHY max EV *only if* the node actually transfers value: conservation `Σbalance==0` is a hard,
  falsifiable invariant the kernel's decide() is money-agnostic about. If bebop stays token-metering
  only (no transfers), TigerBeetle is LOWER EV — keep it as the unified ledger behind `token.ts`
  `record` (idempotent, conserved) rather than a separate boundary.
- DECISION NEEDED from operator: does bebop move money, or only meter it? This sets TB's EV tier.

### 3. Active Inference → LOOP POLICY ADVISOR  (EV: MEDIUM-HIGH — reasoning primitive)
Seam: `loop.ts` already imports `field.ts` (∇·F/∇×F directive). Add `adviseLoop(belief, preferDone)`
selectable via `cfg.activeInference` (same feature-flag pattern as `cfg.field`).
- WHY max EV: complements the field oracle — field = "where to look", FEP = "what to do" under
  belief. It replaces a heuristic with a principled expected-free-energy minimization, grounded in
  real pymdp numbers. Off by default; enables a genuine self-directed agent loop.
- RISK: FEP is O(actions^horizon); cap horizon at 1–2 for the loop. Keep it advisory, not authoritative
  — the guard gate still decides admission.

### 4. optical → FIELD-SEARCH ACCELERATOR  (EV: MEDIUM — off-chain co-processor)
Seam: `field.ts` recall (VSA dot-products). `opticalRecall` ranks candidates by optical propagation
correlation. Wire as an OPTIONAL ranking path behind `cfg.opticalRecall`, falling back to `field.ts`.
- WHY max EV: it is a *simulation* of an optical co-processor (zero-energy matrix transform). The EV
  is the conceptual primitive + a drop-in slot for real optical hardware later; as in-process compute
  it is NOT faster than `field.ts`. Be honest: ship it as a ranking accelerator + research slot, not a
  perf win.
- RED line: the final accept ALWAYS goes through the guard gate; optical output is advisory only.

### 5. Zenoh → INTER-NODE MESH  (EV: MEDIUM — only when multi-node)
Seam: `loop.ts` inter-node comms (currently single-node). `ZenohTransport` is a drop-in.
- WHY max EV *only when* the Sovereign Node runs multi-node: single-node it is dead weight. Ship the
  interface; swap `createLocalMesh` → real `createZenoh` when a peer mesh exists. Priority arbitration
  is the one feature worth keeping even single-node (control-plane precedence).

### 6. wetware/FinalSpark → EXPLORATION CO-PROCESSOR  (EV: LOW — research only)
Seam: none in the deterministic kernel path (bio is non-deterministic, seconds latency).
- WHY LOW EV: cannot sit on the money/verify boundary; high latency; bio-noise. Use only as an
  optional, feature-flagged anomaly-detection/exploration backend behind `WetwareBackend`, never in
  the guard gate. Bio-safe 50 mV invariant is non-negotiable.

---

## Sequencing (max EV first)
1. **zkVM journal** into `applyCommand` — ship now (no external dep, deterministic, high value).
2. **TigerBeetle**: confirm money-vs-meter decision → either money boundary or unified `token.ts` ledger.
3. **Active Inference** advisor behind `cfg.activeInference` in `loop.ts` (parallel to `field.ts`).
4. **optical** recall behind `cfg.opticalRecall` (advisory ranking).
5. **Zenoh** mesh — defer until multi-node.
6. **wetware** — research slot only.

## Guardrails (standing, from kernel.ts + Verified-by-Math)
- Every wiring module is feature-flagged (`cfg.*`); nothing is on by default except the kernel journal.
- Every wiring test is RED+GREEN; the authoritative runner is
  `node --test --import tsx 'src/**/*.test.ts'` (NOT `pnpm run test`, which misses `src/integration/**`).
- No falsified proofs: zkVM receipt and real TB cluster are HONESTLY marked pending, not claimed.
