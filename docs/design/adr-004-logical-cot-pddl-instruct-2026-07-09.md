# ADR-004 — Logical Chain-of-Thought plan verification (PDDL-INSTRUCT)

- Status: ACCEPTED, implemented (flag-OFF seam) — 2026-07-09
- Source: arXiv:2509.13351 "Teaching LLMs to Plan: Logical Chain-of-Thought Instruction Tuning for Symbolic Planning" (PDDL-INSTRUCT)
- Supersedes nothing; extends ADR-003 (Neuro-Symbolic Gate) and N8c (GOAP planner).

## Context

The paper's empirical finding: LLMs hallucinate in *planning* not from missing knowledge but from a
failure to **decompose logical steps**. Free-form Chain-of-Thought lets a model assert a plan without
proving each transition is valid. PDDL-INSTRUCT replaces free-form CoT with **Logical CoT**, which
forces every step to carry an explicit proof of three things:

1. **Action Applicability** — are all PRECONDITIONS satisfied in the current state?
2. **State Transition** — which EFFECTS change the world?
3. **Invariant Preservation** — does the step break any global constraint?

This converts verification from *reactive* (judge the finished plan) to *structural* (compile each
transition before it is admitted).

## Decision

bebop already runs a doer→checker split (`copilot.ts`) and a symbolic planner with PDDL-shaped
`WorldState` / `Action{pre, eff, invariant}` (`analytics/goap.ts`, N8c). We apply the paper by adding
the **verifier half** rather than retraining anything (sovereign-core: no runtime SGD):

- `src/integration/logicalCot.ts`
  - `LogicalStep` — the executor's per-step artifact: `{ action, preconditions, effects, invariants }`.
    The doer no longer emits a free-form "intent"; it emits a **logical transaction**. A step that
    cannot fill these fields is not a valid proposal (the paper's ~"cuts hallucinations at creation").
  - `verifyLogicalPlan(start, steps)` — deterministic step-wise replay. Stops at the FIRST violation
    (precondition unmet / invariant broken / effect-noop) and returns a **precise, re-plannable
    message**: *"cannot apply X: precondition Y unmet in state Z. Re-plan from current state."* This is
    the paper's error-feedback signal that drives the executor's self-correction loop.
  - `logicalChecker(start, steps)` — adapter that turns the structural proof into a copilot verdict
    (`approve` on a valid trace, `revise` with the re-plan note otherwise). Fail-closed: an
    unverifiable plan is never approved.

### How this upgrades reasoning / review / copilot
- **Executor (reasoning/doer):** emits `LogicalStep[]` — a proof, not a wish. Preconditions + effects
  are mandatory fields.
- **Verifier (review/checker):** becomes a *logic auditor*, not a text judge. It checks state-transition
  validity per step, not "is the plan good".
- **Loop:** a violation returns a specific error → the doer re-plans from the actual current state,
  exactly as the paper prescribes (Error → Re-plan, not "bad, try again").

## Consequences

- Deterministic, pure state replay. No RNG / SGD / Date — consistent with the sovereign core.
- FLAG-OFF: inert until a caller emits a plan and calls the verifier; existing copilot text-check path
  is untouched, so all prior tests stay green.
- Falsifiable RED+GREEN (`logicalCot.test.ts`): GREEN valid plan replays; RED precondition failure,
  RED invariant violation, RED effect-noop each proven to be caught with a precise message.
- Verified by the doc-claim gate (check Z) so the claim cannot rot.

## Not doing (deferred, with reason)
- **Instruction-tuning a model on PDDL traces** (the paper's training half): requires offline SGD +
  a dataset; re-open only if/when an offline-trained planner model is actually needed and exported as
  a static artifact. The *verifier* captures the deployable EV without any training.
