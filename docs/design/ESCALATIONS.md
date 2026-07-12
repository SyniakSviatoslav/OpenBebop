# Escalations — human-arbitrated truth resolutions

When `scripts/logic-gate.mjs` (Enforcement model in `LOGIC-LAWS.md`) cannot
establish that a claim is true — because it is **unbacked**, **self-referential
(paradox)**, or a **suspected logical contradiction** — it writes an entry here
and returns exit code `2` (commit allowed, but tracked). A human arbiter (the
operator or a designated user) fills the `Resolution` field.

**Rules**
- `OPEN` escalations may ship, but must be resolved before a release cut.
- Resolution values: `TRUE — <ref>`, `FALSE`, `DEFER — <reason>`.
- Machine state lives in `.bebop/escalations.jsonl` (deduped, regenerated).
  This file is a rendered summary — do not hand-edit below the marker.
- Never delete an `OPEN` entry to make the gate green.

<!-- LOGIC-GATE:OPEN-ITEMS (regenerated each run; do not edit by hand) -->
## Open escalations (7) — human arbiter required

- **ESC-b259452574aa** [unbacked] `README.md:38` — - **Narration + looks** — `bebop init` picks a voice (bebop / plain / sarcastic / corporate-killer)
  - Arbiter: operator · Status: OPEN
- **ESC-aa8900135b97** [unbacked] `README.md:123` — | **zkVM `decide()` journal** | Every admitted command gets a tamper-evident digest over `(state, commandHash, seq)`. On by default at the kernel gate. Replay-verifiable. **Scope:** detects *accidenta
  - Arbiter: operator · Status: OPEN
- **ESC-6146ded2191c** [unbacked] `AGENTS.md:65` —    decisions, and ground-truth facts to the canonical corpus. Source of truth = the corpus, not chat.
  - Arbiter: operator · Status: OPEN
- **ESC-47b70acd788f** [unbacked] `docs/ARCHITECTURE.md:84` — A servo: PID authority, ICIR factor health, resonance risk **before** any gain change, and >3σ
  - Arbiter: operator · Status: OPEN
- **ESC-16bdc71ebfec** [unbacked] `docs/ARCHITECTURE.md:85` — anomaly signals. Fed quality streams; emits math-proven authority. Applied live to any
  - Arbiter: operator · Status: OPEN
- **ESC-cf12cf999033** [unbacked] `docs/design/LOGIC-LAWS.md:147` — > The gate only guarantees the *claim* "this code is secure/correct" is grounded,
  - Arbiter: operator · Status: OPEN
- **ESC-9f594d9df384** [unbacked] `bebop2/README.md:3` — > Greenfield rebuild of bebop. NOT a refactor of `crates/bebop` — a parallel implementation
  - Arbiter: operator · Status: OPEN

## Resolved (0)
_none_
