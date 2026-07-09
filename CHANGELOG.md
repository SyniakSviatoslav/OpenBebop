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
  `src/integration/**`). Authoritative runner confirmed at **305 tests, 0 fail**.
- README + README.uk: added the "Sovereign Node" integrations table and corrected the test count.

### Security / hardening
- **Attack-team (3 red-team subagents) ran after wiring — concrete findings fixed:**

  **F1 — consciousness self-evolution gate drift (red-team).** The kernel gate previously ran
  *after* the corpus mutation (a post-hoc audit append, not the admission authority). Fixed:
  `selfEvolve` now computes `applyCommandChecked` *before* `mem.remember` and aborts the mutation if
  `quarantined` — the kernel verdict is the single source of truth for self-evolution admission.
  - Tests: `consciousness.test.ts` (GREEN admitted → JOURNAL envelope + state advances; RED quarantined
    → state unchanged, DENIED envelope emitted).

  **F2 — optical recall poisoning (red-team).** `recallLocal` assigned every graph-hit a flat
  `score: 1`, so the optical tertiary signal became the de-facto primary ranker; a planted linked
  memory node could reach recall #1 above the genuine hit. Fixed: graph hits now carry their REAL
  spreading-activation energy as `score` (exact match = 1, one hop ≤ decay), so the graph ranks the
  set; optical only re-orders *within equal primary scores*. Also fixed a latent `hits.indexOf(a)`
  comparator bug (it read the live, reshuffling array) by keying on a stable original index.
  - Repro (RED→GREEN): attacker node linked into the corpus now ranks #2 (score 0.5) behind the
    genuine `kernel law` node (score 1.0); previously optical promoted it to #1.

  **F3 — `adviseLoop` belief not validated (red-team).** Degenerate/negative/un-normalized beliefs
  silently produced actionable output (e.g. `[1,1,1]` → `'done'`). Fixed: `adviseLoop` now requires
  a finite, non-negative, non-zero-sum belief and normalizes it; otherwise it throws (no silent
  directive). Confirmed the FEP advisor still cannot admit/deny a command (advisory-only).

  **F4 — money checker fail-open crash (red-team).** `BigInt()` on malformed input threw out of the
  checker, failing the whole command stream open (DoS). Fixed: parse is wrapped in try/catch returning
  `{ ok:false, reason }`, and non-string bigint fields are rejected as malformed. Conservation is
  enforced at shell apply-time via `applyMoneyTransfer`/`moneyConserved`.

  **F5 — journal keyless + no cumulative binding (honest framing).** The zkVM "tamper-evident" digest
  is a keyless FNV-style recomputation: it detects *accidental* bit-flips of stored digests but is
  forgeable by anyone who controls the stored `(state, digest)` (no secret/key/SNARK). Documented as
  **accidental-tamper detection, not cryptographic integrity**. Forward-chained binding + a real
  key/MAC/SNARK receipt remain the upgrade path (tracked, not yet wired — `rzup` unavailable).
  RED tests prove the detection still fires (tamper fails the chain).

### Known limitations (documented, not papered over)
- Stateless kernel → replay/double-spend is only prevented within a single evolving-state lineage
  threaded by the caller; a caller that resets state re-feeds commands. Money idempotency is not yet
  tied to a persisted ledger. (RED-test falsifiable: the detection + gate behavior is proven; the
  persistence gap is a known TODO, not a silent claim.)
- `verifyJournal` is NOT a substitute for a signature/MAC; treat digests as tamper-*detection*, not
  tamper-*proof* against an adversary who controls storage.

---

## [0.2.0] — prior release
- Deterministic Rust/WASM guard kernel, PSQ node identity, living (VSA) memory, L5 telemetry governor,
  freestyle bebop soul self-loop. See git history for detail.
