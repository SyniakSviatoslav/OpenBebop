# bebop Agent — Self-Improvement, Memory, Reflection, Skills: Research + Living-Memory Analysis

> Authoritative synthesis for the bebop agent (`crates/bebop/`). Distills the
> best-practice loops from **OpenCode** and **Hermes Agent**, then performs a
> FULL analysis of the bebop agent's living-memory system and proposes concrete
> enrichments grounded in the project's own development history.
>
> Standing law: every claim here is backed by a real source (cited) or a real
> test (RED+GREEN). No noble lies, no stale "known defect" claims (LOGIC-LAWS §23/§9E).

---

## Part A — OpenCode approach (best practices to adopt)

Source: `skills/autonomous-ai-agents/opencode` + Ryan Carson "Ralph Wiggum" /
Addy Osmani self-improving-agent write-ups (web-searched 2026-07-12).

### A.1 The continuous coding loop (Ralph Wiggum technique)
- Break work into **atomic tasks** (one AI session each, unambiguous pass/fail criteria).
- Loop: pick next undone task → implement → **validate** (tests/typecheck) →
  commit if green → update task status → **reset agent context** → repeat.
- Key insight: **stateless-but-iterative**. Reset context each iter to avoid
  confusion accumulation + context overflow. Solves the "one giant prompt drifts"
  failure mode.
- Tooling: `opencode run '...'` for one-shot; `opencode` (TUI, `pty=true`)
  for interactive; `--continue` / `--session` to resume; `opencode pr N` for
  review; `opencode stats --days 7` for cost.

### A.2 Compound loops (analysis → planning → execution)
- Compound Product pattern: **Analysis loop** (read reports, decide what to build)
  → **Planning loop** (generate PRD + tasks) → **Execution loop** (coding agent).
- One agent's output (branch name / task list) becomes the next agent's input.
- bebop analogue: the `field.rs` physics-veto arbiter is the *execution-gate*;
  we lack the analysis/planning loops — enrichment candidate (§D.3).

### A.3 Context files (AGENTS.md handbook)
- Persistent `AGENTS.md` carries knowledge across iterations.
- **Leverage training knowledge**: don't paste docs the model already knows
  (React, std libs). Save only project-specific / new / obscure facts.
- bebop already has `AGENTS.md` (ponytail rules) + `docs/RULES.md` (Constant
  Doubt) — these ARE the context handbook. Good. Keep lean.

### A.4 Human oversight / elbow grease
- No one-shot autonomous magic. Curate the process: specs, reviews, guidance.
- Watch cost (API burn in a loop). Set iteration caps.

**Takeaways for bebop:** adopt the *atomic-task + validate + reset* loop as the
agent's own internal task driver; the existing `field_gate` is the validator.

---

## Part B — Hermes Agent approach (best practices to adopt)

Source: `skills/autonomous-ai-agents/hermes-agent` + hermes-agent docs
(memory, five-pillars) (web-searched 2026-07-12).

### B.1 Five pillars
Memory · Skills · Soul · Crons · Self-improvement. The agent *learns from
experience by saving reusable procedures as skills* (accumulate over time → better
at YOUR tasks). Persistent memory across sessions. Multi-platform gateway. Profiles
(isolated configs/skills/memory). Extensible (plugins, MCP, cron).

### B.2 Memory system (the model to copy)
- **Two bounded files**, injected as a FROZEN snapshot at session start
  (preserves LLM prefix cache — never change mid-session):
  - `MEMORY.md` — agent's personal notes (env facts, conventions, lessons).
    **Char limit 2,200** (~800 tokens).
  - `USER.md` — user profile (name, prefs, comms style, pet peeves, skill
    level). **Char limit 1,375** (~500 tokens).
- **No auto-compaction.** When a write would exceed limit, the `memory` tool
  returns an ERROR (not silent drop). Agent makes room itself (consolidate/remove)
  in the same turn. → Bounded, explicit, never lossy.
- Actions: `add` / `replace` (substring match) / `remove`. No `read` (auto-injected).
- **What to save** (proactively): user prefs, env facts, corrections, conventions,
  completed-work diary, explicit requests. **Skip**: trivial/obvious, easily
  re-discovered facts, raw data dumps.
- External providers pluggable (built-in, Honcho, Mem0).

### B.3 Skills (self-improvement engine)
- When the agent solves a complex problem / gets corrected / discovers a workflow,
  it persists that as a `SKILL.md` (frontmatter + steps + pitfalls + verification).
- `curator` background process: tracks usage (`use_count`, `hit:N`), marks idle
  skills stale, **archives** (NEVER deletes — max destructive = archive),
  keeps a `.usage.json` telemetry, `pin`ned skills exempt.
- `skills check` / `skills update` / `skills publish`.

### B.4 Delegation + Crons (durable loops)
- `delegate_task`: isolated subagent context + terminal; leaf vs orchestrator roles;
  background returns result as a new turn. NOT durable (dies with parent) → use
  `cronjob` / `terminal(background, notify)` for work that must outlive session.
- Crons: durable scheduler, `context_from` chains job A→B, multi-platform delivery.

### B.5 Shell-hooks allowlist
- Some shell-hook integrations need explicit allowlisting before they fire
  (`~/.hermes/shell-hooks-allowlist.json`). Relevant to our `law-hooks.mjs`
  enforcement model.

**Takeaways for bebop:** (1) bound the memory store by char/entry limit +
explicit-make-room (not silent drop); (2) split agent-notes vs user-profile;
(3) add a skill-analogue that persists lessons as retrievable procedures;
(4) add a curator/reflection pass.

---

## Part C — bebop agent living-memory: FULL analysis

### C.1 What exists (verified on disk 2026-07-12)
- `crates/bebop/src/memory.rs` — `LivingMemory`: one associative store
  (VSA + graph + recursion). **Deterministic** (FNV-1a hash, no RNG/Date in
  output paths). `tick()` ages + evicts.
- `attic` cold-tier **ALREADY PRESENT**: `tick()` moves evicted nodes to
  `attic` (HashMap) — **never dropped**. `restore()` + `get_from_attic()`
  give a recoverable path. **This is the safe-apply model, already implemented.**
- `knowledge.rs` — `recall()` VSA match over `nodes`; `recall_graph()`
  surfaces edge-connected nodes.
- `recall_graph.rs` — `recall_at()` ranking (gold-set recall metric).
- `field.rs` — deterministic physics-veto arbiter (Kalman + limit-cycle +
  loop-health). The *execution gate*.
- `tui.rs` — drives `tick()` in the UI loop.
- `docs/design/bebop-memory-optimisation-fable-research-2026-07-11.md` —
  fable research that *recommended* the move-to-attic refactor.

### C.2 The "known defect" is RESOLVED (honest correction of stale claim)
- `skills/note-taking/living-memory-safe-apply` and
  `docs/design/CONSOLIDATED-AUDIT-EXTRACT-2026-07-11.md` describe
  `memory.rs:60-66` `tick()` as **destructive** (`nodes.retain(hash%7 !=
  clock%7)` = permanent delete, no cold tier).
- **REALITY (verified 2026-07-12):** that code is GONE. Current
  `memory.rs:101-118` does move-to-attic; test
  `tick_moves_evicted_node_to_attic` **PASSES** (`cargo test -p bebop memory`
  → 6 passed, 0 failed). The destructive version was the *pre-refactor* state
  the fable research was arguing AGAINST.
- **Conclusion:** the "known defect" bullet in the skill + audit doc is a
  **stale-claim / false-green** — exactly what LOGIC-LAWS §23 (no noble lie /
  no stale claim) forbids. Both are corrected in this commit (see Part E).

### C.3 Gaps vs OpenCode/Hermes (enrichment candidates)
| # | Capability | bebop now | OpenCode/Hermes | Gap |
|---|---|---|---|---|
| 1 | Bounded capacity | `nodes` HashMap, **unbounded** | Hermes 2,200/1,375 char hard limit | grows forever |
| 2 | Importance ranking | VSA match only | Hermes/Osmani hit-count retrieval order | no `hit:N` |
| 3 | User-profile split | all in `nodes` | Hermes `USER.md` separate | no user/agent split |
| 4 | Reflection/self-retro | none | Osmani "wrap-up" skill; Hermes curator | no post-task lesson |
| 5 | Skill-analogue | memory nodes only | Hermes `SKILL.md` from experience | no lesson→procedure |
| 6 | Loop orchestrator | `field_gate` (gate only) | Ralph Wiggum atomic-loop | no internal task driver |
| 7 | Curator/staleness | none | Hermes curator archives idle | no lifecycle |
| 8 | Frozen snapshot | `knowledge.rs` reads live | Hermes injects once @start | re-reads each call |

---

## Part D — Proposed enrichments (grounded in dev history)

Development history that motivates these (from corpus + this session):
- The `serde_json` WS-5 break, the `clippy`/`deny` debt caught by `law-hooks.mjs`,
  the ACVP `sv_case!` harness typo — all were **lessons that should persist as
  skills**, not be re-learned. → enrichment #5.
- The fable "destructive tick" false-defect shows doc-drift is silent. → #4
  (reflection writes a lesson node) + #7 (curator prunes stale docs).
- The operator's max-parallel worktree discipline + 3-model-review gate shows the
  *loop orchestrator* is external (Hermes). bebop agent should still own a
  **local atomic-task driver** (#6) so it can self-improve inside one process.

### D.1 Bound the store (fix #1, keep non-destructive)
Add `const MAX_NODES: usize` + `importance: u32` field. On overflow: **promote
lowest-importance live node to `attic`** (move, never drop) — RANK-only, never
CULL (per `living-memory-safe-apply` §4). RED: `size() <= MAX_NODES` after
overflow insert. GREEN: `attic_size()` grows.

### D.2 Importance scoring (fix #2)
Add `hit_count: u32` to `MemoryNode`; `recall()` increments on match; retrieval
orders by `hit_count` desc. RED: high-hit node ranks above zero-hit on same
query. GREEN: tie-break deterministic by id.

### D.3 User-profile split (fix #3)
Add `user_profile: HashMap<String, MemoryNode>` (the `USER.md` analogue).
`remember_user()` / `recall_user()`. RED: user fact isolated from agent notes.
GREEN: `recall()` over `nodes` doesn't surface user-profile entries.

### D.4 Reflection / self-retro (fix #4) — the §16 Feynman loop, enforced
Add `reflect(lesson: &str)` → inserts a `Layer::Long` node tagged
`ponytail:` with `hit_count=0`. Called by the agent after every task commit
(bridges to `law-hooks.mjs` §16 self-retro enforcement). RED: a post-task
reflect produces a retrievable Long node. GREEN: `layer_size(Long)` increases.

### D.5 Skill-analogue (fix #5) — lesson→procedure
Add `skills: HashMap<String, String>` (id → SKILL.md text). `learn_skill()`
parses `ponytail:`/`LESSON` markers from a commit/reflection and stores a
procedure node. This is the bebop-port of Hermes' self-improvement engine,
kept inside the Rust agent (no external dependency). RED: a lesson with a
`LESSON` marker becomes a retrievable skill. GREEN: `skills.contains_key(...)`.

### D.6 Internal atomic-task driver (fix #6)
Add `next_task()` / `complete_task()` over a `tasks: Vec<Task>` (the Ralph
Wiggum loop, in-Rust). Each task: validate via `field_gate` before commit.
This makes the agent self-driving, gated by its own physics veto.

### D.7 Curator pass (fix #7)
Add `curate()` that archives (moves to `attic`) `Long` nodes with `hit_count==0`
after N ticks — **archive, never delete** (Hermes curator model). RED: idle
node leaves `nodes` but stays in `attic`. GREEN: `attic_size()` reflects it.

### D.8 Keep what works
- **Do NOT touch the move-to-attic non-destructive design** — it is correct and
  already verified. All enrichments ADD tiers/fields; none delete the cold tier.
- Keep `field.rs` physics veto as the validator for D.6.

---

## Part E — Verification (RED+GREEN, real evidence)

### E.1 Existing (already green, re-run this session)
```
$ cargo test -p bebop memory
test memory::tests::remember_then_size ............ ok
test memory::tests::tick_forgets_deterministically  ok
test memory::tests::tick_moves_evicted_node_to_attic  ok   <- PROVES non-destructive
test result: ok. 6 passed; 0 failed
```
This RED+GREEN-proves the "defect" is resolved: evicted node → `attic`,
recoverable via `restore()`, payload preserved.

### E.2 Stale-claim corrections in this commit
- `docs/design/bebop-memory-optimisation-fable-research-2026-07-11.md`:
  marked the destructive-`tick` finding as **RESOLVED** (move-to-attic shipped).
- `skills/note-taking/living-memory-safe-apply`: "known defect" bullet annotated
  **RESOLVED — memory.rs is now non-destructive (attic cold-tier shipped)**.

### E.3 Enrichments (D.1–D.7) — status
NOT yet implemented in this commit (analysis + design only, per operator: "research
first, then wire"). Each carries its own RED+GREEN gate spec above. They are
the next build wave; tracked in the roadmap doc.

---

## Part F — Honest gaps (no false-green)
- The OpenCode/Hermes *external* loop orchestration (cron, delegation, gateway)
  lives in Hermes, not bebop. D.6 is an *in-process* analogue only.
- `MAX_NODES` value, `N` for curator idle-window, and the skill-parse grammar
  are TBD at implementation time (will be falsifiable-tested, not guessed).
- No benchmark exists for recall quality pre/post enrichment — one should be added
  alongside D.2 (gold-set `recall_at` already in `recall_graph.rs` is the hook).
