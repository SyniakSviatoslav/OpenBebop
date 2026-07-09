/**
 * logicalCot.ts — PDDL-INSTRUCT Logical Chain-of-Thought verifier (arXiv:2509.13351 applied).
 *
 * The paper's finding: LLMs hallucinate in PLANNING not for lack of knowledge but for lack of
 * step-wise logical decomposition. Their fix (PDDL-INSTRUCT / Logical CoT) forces every plan step
 * to carry an explicit proof: (1) Action Applicability — are all PRECONDITIONS satisfied in the
 * current state? (2) State Transition — what EFFECTS change the world? (3) Invariant Preservation —
 * does the step break any global constraint? Verification moves from REACTIVE (judge the final plan)
 * to STRUCTURAL (compile each transition).
 *
 * We already have the substrate: goap.ts WorldState + Action{pre,eff,invariant}. This module is the
 * VERIFIER half: given a start world and an executor-produced logical plan (each step a proof block),
 * `verifyLogicalPlan` replays the transitions deterministically and returns either a valid trace or
 * the FIRST violation with a precise, re-plannable error — exactly the paper's feedback signal
 * ("cannot apply Action X: precondition Y unmet in state Z").
 *
 * This upgrades the copilot doer→checker exchange: the doer emits `LogicalStep[]` (a logical
 * transaction, not a free-form intent); the checker becomes a logic auditor, not a text judge.
 *
 * Deterministic: pure state replay over the finite world. No RNG/SGD/Date. Falsifiable RED+GREEN.
 * FLAG-OFF: inert unless a caller emits a plan and calls the verifier.
 */

import type { WorldState } from './analytics/goap.ts';

/** A single executor step carrying its PDDL-style logical proof (the paper's core artifact). */
export interface LogicalStep<S extends WorldState = WorldState> {
  action: string;
  /** Action Applicability — keys that MUST hold (deep-equal) in the current state. */
  preconditions: Partial<S>;
  /** State Transition — keys the action writes to the world. */
  effects: Partial<S>;
  /**
   * Invariant Preservation — global constraints that must remain true in the POST state.
   * Each is a named predicate over the resulting world (e.g. "battery>=15", "branch-clean").
   */
  invariants?: { name: string; holds: (s: S) => boolean }[];
}

export type LogicalViolation =
  | { kind: 'precondition'; action: string; unmet: string[]; state: WorldState }
  | { kind: 'invariant'; action: string; broken: string[]; state: WorldState }
  | { kind: 'effect-noop'; action: string; state: WorldState };

export interface LogicalProof<S extends WorldState = WorldState> {
  ok: boolean;
  /** the world after each admitted step (length = admitted steps). */
  trace: S[];
  /** terminal world (start when nothing admitted). */
  world: S;
  /** the FIRST violation encountered, with a human/agent-readable message for re-planning. */
  violation?: LogicalViolation;
  message: string;
}

function unmetPreconditions<S extends WorldState>(world: S, want: Partial<S>): string[] {
  const bad: string[] = [];
  for (const k of Object.keys(want)) {
    const w = (want as Record<string, unknown>)[k];
    if (world[k] !== w) bad.push(`${k}=${JSON.stringify(w)} (actual ${JSON.stringify(world[k])})`);
  }
  return bad;
}

/**
 * Replay a logical plan step-by-step (PDDL-INSTRUCT structural verification). Stops at the FIRST
 * step whose preconditions are unmet OR whose post-state breaks an invariant, returning a precise
 * error the executor can re-plan against. GREEN when every transition is valid to the end.
 *
 * `requireProgress` (default true): a step whose effects change nothing is flagged as an effect-noop
 * (a common hallucination — an "action" that claims to advance the world but is inert).
 */
export function verifyLogicalPlan<S extends WorldState>(
  start: S,
  steps: LogicalStep<S>[],
  opts: { requireProgress?: boolean } = {},
): LogicalProof<S> {
  const requireProgress = opts.requireProgress ?? true;
  const trace: S[] = [];
  let world = start;

  for (const step of steps) {
    // 1. Action Applicability
    const unmet = unmetPreconditions(world, step.preconditions);
    if (unmet.length) {
      return {
        ok: false,
        trace,
        world,
        violation: { kind: 'precondition', action: step.action, unmet, state: world },
        message: `cannot apply "${step.action}": precondition(s) unmet — ${unmet.join(', ')}. Re-plan from current state.`,
      };
    }
    // 2. State Transition
    const next = { ...world, ...step.effects } as S;
    if (requireProgress && JSON.stringify(next) === JSON.stringify(world)) {
      return {
        ok: false,
        trace,
        world,
        violation: { kind: 'effect-noop', action: step.action, state: world },
        message: `"${step.action}" is an effect-noop: its effects change nothing (inert action / hallucinated progress).`,
      };
    }
    // 3. Invariant Preservation (checked on the POST state)
    const broken = (step.invariants ?? []).filter((inv) => !inv.holds(next)).map((inv) => inv.name);
    if (broken.length) {
      return {
        ok: false,
        trace,
        world,
        violation: { kind: 'invariant', action: step.action, broken, state: next },
        message: `"${step.action}" breaks invariant(s): ${broken.join(', ')}. Action refused (symbolic firewall).`,
      };
    }
    world = next;
    trace.push(world);
  }

  return { ok: true, trace, world, message: `logical plan verified: ${steps.length} step(s), all transitions valid.` };
}

/**
 * Copilot checker adapter: turn the structural verifier into a doer→checker verdict. A valid proof
 * APPROVES; a violation REVISES (with the precise re-plan message as the note). Fail-closed: a plan
 * that cannot be parsed/verified is never approved. Mirrors copilot.ts CheckerFn semantics but over
 * a LOGICAL plan rather than free text.
 */
export function logicalChecker<S extends WorldState>(
  start: S,
  steps: LogicalStep<S>[],
): { verdict: 'approve' | 'revise'; note: string } {
  const proof = verifyLogicalPlan(start, steps);
  return proof.ok ? { verdict: 'approve', note: proof.message } : { verdict: 'revise', note: proof.message };
}
