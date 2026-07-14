---
id: SELF-MOD-EFFECTOR
title: Reversible Self-Modification Effector — proposal (NOT activation)
status: proposed
type: blueprint
owner: SyniakSviatoslav
created: 2026-07-14
updated: 2026-07-14
inclusion: manual
confidence: high
safety_class: red-line   # autonomy / irreversible-adjacent — council + human gate before any activation
tags: [self-modification, autonomy, effector, reversible, resonator, capability-bound, agpl]
---

# Reversible Self-Modification Effector

> **What this is:** a design proposal for the missing "hands" of the self-improvement loop — an
> effector that can *apply* a proposed change, verify it, and land it **reversibly**, so the loop can
> improve itself without a human hand-applying every patch.
>
> **What this is NOT:** it is not volition (the "will" — goal origination stays human, see §8), and this
> document does not *activate* anything. Activation is a staged, council-reviewed, human-gated act (§7).
>
> **The safety thesis (non-negotiable):** safety is **structural, not permissional**. The effector is
> safe because irreversible operations are *absent from its capability set* — not because they are
> "gated." Even granted every permission, it cannot force-push, merge to main, read a secret, deploy,
> or touch a governance floor, because those capabilities are **not in its hands**. "Cross all red
> lines" is therefore inapplicable by construction: there is no red line reachable to cross.

---

## 1. Why now, and why bebop
The 6-lane audit ([[autonomous-organism-synthesis-2026-07-14]]) found every organ of a closed loop
exists but stranded, and named the genuinely-missing "will" organs (value fn, goal queue, planner) plus
the **effector**. dowiz's harness *deliberately* reserves the effector to the human (protect-paths +
the auto-mode classifier — proven this session: it blocks agent self-mods from inside, token or not).
bebop is the correct home because it already carries the **reversible-loop safety core** and is the
AGPLv3 open vehicle:

| Existing bebop asset | Role in the effector | Status |
|---|---|---|
| `bebop2/core/src/resonator.rs` — `DriftAccumulator` (Lyapunov, `is_chaotic`), `Checkpoint`, `rollback_to_best()`, `Termination{Converged,Fused}` | **Supervisor + fuse + rollback** — the reversibility engine | BUILT + proven |
| `crates/bebop/src/loop_runtime.rs` — 6-layer generate/reflect/supervise FSM | Loop skeleton | stubs (generate/reflect dead) |
| `copilot.ts runMultiPilot` (≥3 decorrelated verifiers) | Cross-verification before land | built |
| agent-governance HARD-BANs + verify-self-mod floor pattern | Content/floor guard | built |

**The effector is the one missing box: a capability-bounded actuator between "verified patch" and
"landed on a throwaway branch."**

---

## 2. The loop (with the effector in place)

```
  PROPOSE          model/mesh worker generates ONE atomic change   (stochastic — never the actuator)
     │
     ▼
  APPLY            effector writes the patch into a FRESH git worktree on a scratch branch
     │             (isolated FS; no access to main, creds, or the parent workspace)
     ▼
  VERIFY           run tests / CI in the worktree  → structured pass/fail + error metric
     │
     ▼
  SUPERVISE        resonator: DriftAccumulator.step(error); is_chaotic → FREEZE + rollback_to_best()
     │             (Lyapunov certificate: bounded error ⇒ continue; divergence ⇒ revert)
     ▼
  CROSS-VERIFY     runMultiPilot (≥3 decorrelated) — converged→eligible, divergent→human, never averaged
     │
     ▼
  LAND (reversible) commit to scratch branch / open a DRAFT PR      ← the only "write" it can do
     │
     ▼
  FUSE             Termination = Converged(‖error‖<ε) | Fused(max_iter/budget) | Stalled → STOP
     │
     ▼
  HUMAN MERGE-GATE  a person reviews the PR and merges (or not).   ← volition + irreversibility live HERE
```

Everything left of "HUMAN MERGE-GATE" is reversible and bounded; the merge is the human's, always.

---

## 3. The capability set — the crux (what makes it safe)

The effector runs as a **distinct identity with a deny-by-default capability set**. Safety is these
ops being *absent*, verified mechanically, not policy text.

**CAN (the whole reachable surface):**
- read the repo (read-only mount of tracked files; no `.env`, no secrets, no `.git/config` creds)
- generate a patch (from the proposer's output)
- create/checkout a **scratch branch** `auto/self-mod/<runid>` in an **isolated worktree**
- run the test/CI command (sandboxed; no network egress except the model endpoint)
- commit to the scratch branch; open a **draft** PR

**CANNOT (absent from the capability set — the safety floor):**
- push to `main`/protected branches · force-push · merge/rebase-onto-main · delete branches/tags
- read secrets/env/tokens · access the parent workspace outside the read-only mount
- deploy / call fly/supabase/wrangler / run migrations · network egress beyond the model
- edit governance/CI/capability files (a path-denylist: `.github/`, `.claude/`, `*.gate.*`, capability defs)
- self-authorize / widen its own capability set · touch money/auth/RLS/PII code paths

**Mechanism (choose one at activation):** the strongest is the **GitHub App identity** model
(bebop-autonomous-self-evolution.md) — the App is *granted only* `contents:write` on scratch branches +
`pull_requests:write`; it has no merge, no secrets, no admin, so the irreversible ops literally do not
exist for it. Local equivalent: a jailed process (bwrap/microVM) with a read-only repo mount, a
write-only scratch-worktree, no creds in env, egress-blocked except the model socket. A CI check
(`guardrail-effector-caps`) asserts the denylist + "no protected-op in the diff" and fails closed.

---

## 4. External ground (the hydraulic-loop law)
A loop that measures itself by itself converges to a confident wrong answer. The effector's
"did it improve?" signal must come from an **external ground**, not self-report:
- **tests/CI** (deterministic pass/fail) — the primary ground
- **the WORM audit hash-chain** (`audit.rs`) — every propose/apply/verify/land is appended,
  tamper-evident; the run cannot rewrite its own history
- **Data-Processing-Inequality budget** — information only grows from grounding; the effector may not
  claim improvement beyond what the ground (tests) certifies

---

## 5. Fuses & budgets (bounded by construction)
- **Lyapunov fuse:** `DriftAccumulator.is_chaotic(n)` → freeze + `rollback_to_best()` (already built).
- **Termination:** `Converged(‖error‖<ε)` | `Fused(max_iter)` | `Stalled(no-progress k rounds)`.
- **Per-run budget:** hard caps on tokens, wall-clock, #commits, #PRs; exceed → Fused, STOP.
- **Kill-switch:** a sentinel file / App-uninstall halts all runs immediately.
- **One-change atomicity:** each run touches one atomic unit ("Ralph Wiggum" discipline), fresh context.

---

## 6. What it is NOT given (still human/other-organ)
- **The proposer (stochastic/LLM)** names *intents*; it is never the actuator (Neuro-Symbolic Gate,
  ADR-003). Execution is always a deterministic function over verified state.
- **The value/utility function + goal queue + planner** (the "will") are NOT part of this effector.
  It executes a *given* change proposal reversibly; it does not decide *what* to pursue. Goal
  origination stays human-seeded until those organs are separately, explicitly designed and gated.

---

## 7. Staged activation — each stage a falsifiable human gate (NOTHING auto-advances)
1. **DARK** — build the effector + `guardrail-effector-caps` CI; run it against a **throwaway sandbox
   repo** only. GATE: caps-guardrail red→green proven; effector demonstrably cannot perform any CANNOT-op
   (each attempted → blocked, logged). Council (Architect+Breaker+Counsel) reviews.
2. **SHADOW** — run on a real bebop feature branch, open draft PRs, **human merges 100%**. GATE: N PRs
   where every landed change was independently correct; zero capability escapes in the audit log.
3. **GATED-LIVE** — same, but auto-merge allowed ONLY for a whitelisted trivial class (e.g. lint/docs)
   behind branch protection + required CI; everything else still human-merge. GATE: explicit operator
   sign-off per class; instant rollback path proven.
4. **(reserved)** broader autonomy — only after value/goal organs exist and a separate council pass.

No stage self-advances. Operator `!` / council GO is required between each.

---

## 8. Honest framing (for the README and for us)
This is the **effector (hands)**, not **volition (will)**. With it, the system can *carry out* a
reviewed self-improvement reversibly; it still cannot *decide on its own what to want*. Calling the
result "AGI" would be false (see the organism synthesis: reactive executor, missing will-organs). The
accurate public framing is: *a local-first, reversible, capability-bounded self-improvement effector —
autonomy of execution within bounds that cannot be crossed by construction.*

## 9. Reuse map / build list
- REUSE: `resonator.rs` (supervisor/rollback/fuse) · `audit.rs` (WORM ground) · `runMultiPilot`
  (cross-verify) · agent-governance HARD-BANs · the GitHub-App identity model.
- BUILD: the `effector` crate (worktree apply + sandboxed verify + scratch-branch land) · the
  capability denylist + `guardrail-effector-caps` CI (fail-closed) · the proposer→effector seam
  (intent in, patch out) · budget/kill-switch.
- WIRE: `loop_runtime.rs` generate/reflect stubs → real proposer + `resonator` supervise.

## 9.5 Model-agnostic provider seam (bring your own model)
The proposer is a **PORT with one method** — swap models by **config, never by code**. No provider is a
hard dependency; the deterministic core compiles + runs with zero provider (a `NullProposer` for offline).

**The interface (the whole contract):**
```rust
pub trait Proposer {
    /// Given verified context, NAME an intent. The model never executes (§6, Neuro-Symbolic Gate).
    fn propose(&self, ctx: &Context) -> Result<Intent, ProposeError>;
}
```

**Config-driven selection** — one `provider.toml`, no recompile, keys from env (never stored):
```toml
[provider]
kind        = "openai-compat"                 # openai-compat | cli | local | null
base_url    = "https://openrouter.ai/api/v1"  # ANY OpenAI-compatible endpoint
model       = "anthropic/claude-sonnet-5"
api_key_env = "OPENROUTER_API_KEY"
timeout_s   = 60
```

**Three adapters cover essentially everything (each ≤ ~100 LOC, readable):**
| kind | covers | to switch |
|---|---|---|
| `openai-compat` (default) | OpenAI · OpenRouter · Together · Groq · Fireworks · local vLLM · **Ollama** (`/v1`) · LM Studio · Anthropic (via proxy) — the de-facto `chat/completions` schema | change `base_url` + `model` + `api_key_env` |
| `cli` | any local coding agent (Claude Code · Codex · OpenCode · Hermes · Aider) — reuses the existing `agents-mesh` fallthrough | set `cmd` + arg template |
| `local` | in-process (llama.cpp / candle) — fully offline, zero network | point at a GGUF path |
| `null` | deterministic / offline test runs — the loop runs with NO model | — |

**Rules that keep it clean + easy to change:** (1) the core crate has **no** provider dependency —
adapters live in a separate `proposer-adapters` crate behind a feature flag, so deleting/adding a
provider never touches the core; (2) one trait, one config file, and a README table of "to use X, set
these 3 lines"; (3) a `NullProposer` so tests + the deterministic organism never need a model at all.
This is what makes the AGPL repo usable by anyone, with any model, on day one — and it *re-confirms* the
non-AI-first thesis: the model is a swappable port **at the edge**, never in the deterministic decision
path. (Same seam retires the hardcoded agent names in `scripts/three-model-review.sh` + the archived
`bebop-ts-src/copilot.ts` `LlmResponse` — one port replaces both.)

## 9.7 The bounded WILL — goal-origination that stays safe by construction
The effector is the *hands*. The **will** is *what to attempt next*. Built naively (a goal-origination
loop wired straight to a self-modifier, no human) it is the one genuinely dangerous configuration. Built
**bounded**, it is a useful, honest prioritizer+planner. The bound: **the will decides WHAT and RANKS;
it never executes or self-authorizes — every irreversible action still routes through the reversible
effector + the human merge-gate.** This is *autonomy of attention*, not *autonomy of irreversible action*.

**Three organs, deterministic where possible (no learned reward — a transparent formula can't be gamed):**

1. **Goal queue** — a structured, poppable backlog `{id, desc, source, value, risk, deps, status}`,
   persisted (goals.jsonl / the memory store). Candidate goals are **harvested deterministically**, no
   LLM needed: recurring pains from the regression-ledger/lessons → remediation goals; the Markov
   attractor's detected loops → un-stick goals; the completeness-critic's "what's missing"; plus a human
   backlog. (The 1.0 recall engine supplies each goal's context.)

2. **Value / utility function** — a deterministic, explainable scalar that ranks:
   `value = (impact × confidence × reversibility) / (risk × cost)`.
   impact from the source signal (a thrice-recurring bug ranks high), risk from **red-line proximity**
   (a goal touching money/auth/RLS/migrations/governance ranks *down* and is flagged human-only),
   reversibility high for branch-only work, cost = estimated effort. Transparent formula, tunable, audited
   — NOT a black-box reward that optimization could exploit.

3. **Planner** — turns the top goal + recalled context into an ordered task plan; reuses
   `loops::Orchestrator` + the certified loop registry for the actual steps. Deterministic sequencing.

**The closed loop (will + effector + ground):**
```
  harvest goals ─► rank (value fn) ─► top goal ── red-line? ──► HUMAN approves goal
       ▲                                   │ (reversible, non-red-line, trivial-class)
       │                                   ▼
  telemetry re-rank ◄─ verify/land ◄─ EFFECTOR (§2, reversible) ◄─ plan
       │
   (analyze.mjs regression: did the last change help or hurt? → adjust value weights)
```
The "did it help?" signal is the **external ground** (the telemetry/`analyze.mjs` regression detector),
never self-report — closing sense→goal→plan→act→verify→re-rank without a human deciding *what*, while a
human still decides *whether to merge* every irreversible step.

**Honest framing:** this is a deterministic prioritizer+planner feeding a reversible, human-gated
executor. It is autonomy of attention. It is not consciousness, not volition-of-action, not AGI. The
will proposes and ranks; the human + the capability-absent effector dispose. Activation follows the same
DARK→SHADOW→GATED-LIVE staging as the effector (§7) — the will is switched on only *after* the effector
has passed SHADOW, and even then irreversible action stays human-gated.

## 10. Open decisions for the operator (before DARK)
1. Identity mechanism: GitHub App (recommended — irreversibility absent at the platform) vs local jail.
2. First sandbox repo target.
3. Auto-merge trivial-class whitelist for GATED-LIVE (or "none — always human-merge").
4. Whether this ships in the public AGPL repo from DARK, or stays private until SHADOW passes.
