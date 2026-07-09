# Changelog

All notable changes to Bebop are documented here. Format: keep it falsifiable — every line is
backed by a RED+GREEN test in `src/**/*.test.ts` (authoritative runner:
`node --test --import tsx 'src/**/*.test.ts'`).

## [0.3.0] — 2026-07-09 — "Sovereign Node: integrations composed into the one gate"

### Added
- **zkVM `decide()` journal — on by default at the kernel gate.** Every admitted command now emits a
  `JOURNAL` envelope with a tamper-evident digest over `(state, commandHash, seq)`. The kernel's
  `applyCommandChecked` journals unconditionally (`journal=true` default). Replay-verifiable via
  `verifyJournal` / `compose.verifyJournalChain`; tampering any entry fails the chain.
  - Tests: `core.test.ts` (GREEN digest verifies; RED tampered state fails), `compose.test.ts`
    (GREEN chain replays; RED tamper breaks it).
- **TigerBeetle money boundary composed into the kernel gate.** `applyCommandChecked(.., money=true)`
  runs the `moneyTransferChecker` structural law (`amount>0`, `debit≠credit`, idempotent) *in addition
  to* the caller's policy checker — fail-closed. Mint/burn/replay are refused at the universal gate.
  - Tests: `core.test.ts` (GREEN legal transfer; RED mint `amount<=0`, RED replay).
- **Active Inference advisor in the dispatch loop.** `adviseLoop` (FEP policy selector over
  `{stuck, progressing, done}`) surfaces an advisory action when `cfg.activeInference` is set; the
  guard still decides admission. Advisory-only, never overrides the gate.
  - Tests: `loop.test.ts` (GREEN advisor surfaces when flag on; RED stays off when flag off),
    `loop-advisor.test.ts`.
- **Optical field recall in `knowledge.ts`.** `recall(query, { opticalRecall: true })` re-ranks
  candidates by SVETlANNa/Meep field correlation (placed behind a thin-lens mask) as a *third, advisory*
  signal — graph score and vector sim dominate; optical never filters and never promotes a weak hit
  above a strong one.
  - Tests: `knowledge.test.ts` (GREEN candidate id-set preserved; RED graph score dominates optical).
- **Tamper-evident self-evolution audit.** `bebop self evolve` now records each approved corpus
  mutation as a kernel `PUBLISH` command (journaled) and exposes `verifySelfEvolution()` — the agent
  can prove its own evolution history is unbroken (falsifiable: tamper breaks the replay).
  - Tests: `consciousness.test.ts` (GREEN clean chain verifies; RED tampered digest fails).
- **Sovereign Node composition layer** (`src/integration/compose.ts`) is now the canonical apply path:
  it delegates to the kernel's single gate (zkVM journal + optional TigerBeetle money), so there is one
  decision path, not two.

### Changed
- **`npm test` now covers the integration layer.** The script glob changed from `src/*.test.ts` to
  `src/**/*.test.ts`, so `self maintain` and CI exercise the full RED+GREEN suite (was silently missing
  `src/integration/**`). Authoritative runner confirmed at **303 tests, 0 fail**.
- README + README.uk: added the "Sovereign Node" integrations table and corrected the test count.

### Security / hardening
- Red-team (attack-team) probes ran against each new layer after wiring; findings and fixes are noted
  below as they land (see "Red-team findings" subsections). No live exploit was left open.

---

## [0.2.0] — prior release
- Deterministic Rust/WASM guard kernel, PSQ node identity, living (VSA) memory, L5 telemetry governor,
  freestyle bebop soul self-loop. See git history for detail.
