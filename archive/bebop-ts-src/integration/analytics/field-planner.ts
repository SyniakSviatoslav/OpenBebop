/**
 * PDDL ↔ FIELD BRIDGE SEAM (2026-07-09c): wires `rustFieldArbiter` into the deterministic planner.
 *
 * This is "The Final Arbiter" made concrete for the planning layer:
 *   • GOAP (goap.ts) is the SYMBOLIC EXECUTOR — it searches a finite state graph and returns the
 *     cheapest action sequence, or ok:false if the goal is unreachable (anti-hallucination).
 *   • The Rust field core is the COST SURFACE — `rustFieldArbiter` rates each proposed action's real
 *     downstream impact and PERMITS / WARNS / OVERRIDES the planner.
 *
 * The seam is FLAG-OFF: inert until `fieldGatePlan` is called with a built graph + seed mapping.
 * No RNG, no SGD, no Date. Deterministic + falsifiable.
 *
 * Workflow:
 *   1. Build the order/dependency graph once (rustBuild) and run a few propagations so the kernel
 *      accrues per-node sensitivity (rustFieldSensitivity) — ZERO extra infra.
 *   2. For each candidate GOAP action, map it to a disruption `seed` (which node it would take down).
 *   3. Call `fieldGatePlan` → returns the plan annotated with { verdict, fieldCost, contours }.
 */
import { rustFieldArbiter, rustTopKContours, type ArbiterVerdict, type ArbiterResult } from '../field-rust.ts';
import type { PlanResult } from './goap.ts';

export interface FieldGatedAction {
  name: string;
  /** node index in the field graph this action would disrupt (impulse seed source). */
  seedNode: number;
  verdict: ArbiterVerdict;
  fieldCost: number;
  pddlCost: number;
  reason: string;
  /** Top-K nodes the disruption would hit hardest (explainability surface). */
  contours: { index: number; impact: number }[];
}

export interface FieldGatedPlan {
  ok: boolean;
  plan: string[];
  /** per-action field verdict; the plan is OVERRIDDEN if any action is 'override'. */
  actions: FieldGatedAction[];
  /** 'permit' | 'warn' | 'override' — the worst verdict across the plan. */
  overall: ArbiterVerdict;
  reason: string;
}

/**
 * Gate a GOAP plan against the field cost surface.
 * `seedOf(actionName)` maps a planned action to the field-graph node it would disrupt.
 * `pddlCostOf(actionName)` is the planner's own symbolic cost estimate (defaults to 1.0).
 */
export async function fieldGatePlan(
  plan: PlanResult,
  opts: {
    seedOf: (actionName: string) => number;
    pddlCostOf?: (actionName: string) => number;
    k?: number; // Top-K contours per action (default 3)
    t?: number;
    mismatchRatio?: number;
  },
): Promise<FieldGatedPlan> {
  if (!plan.ok) {
    return { ok: false, plan: plan.plan, actions: [], overall: 'override', reason: `goap: ${plan.reason ?? 'unreachable'}` };
  }
  const k = opts.k ?? 3;
  const actions: FieldGatedAction[] = [];
  let worst: ArbiterVerdict = 'permit';
  const rank = (v: ArbiterVerdict) => (v === 'override' ? 2 : v === 'warn' ? 1 : 0);
  for (const name of plan.plan) {
    const node = opts.seedOf(name);
    const n = await graphSize();
    if (node < 0 || node >= n) {
      actions.push({ name, seedNode: node, verdict: 'override', fieldCost: -1, pddlCost: 0, reason: 'seed node out of graph range', contours: [] });
      worst = 'override';
      continue;
    }
    const seed = new Float64Array(n);
    seed[node] = 1.0;
    const pddlCost = opts.pddlCostOf ? opts.pddlCostOf(name) : 1.0;
    const arb: ArbiterResult = await rustFieldArbiter(seed, pddlCost, { t: opts.t, mismatchRatio: opts.mismatchRatio });
    const contours = await rustTopKContours(seed, k, { t: opts.t });
    actions.push({ name, seedNode: node, verdict: arb.verdict, fieldCost: arb.fieldCost, pddlCost: arb.pddlCost, reason: arb.reason, contours });
    if (rank(arb.verdict) > rank(worst)) worst = arb.verdict;
  }
  const overridden = worst === 'override';
  return {
    ok: !overridden,
    plan: plan.plan,
    actions,
    overall: worst,
    reason: overridden
      ? 'FIELD OVERRIDE: an action would ripple beyond PDDL tolerance — plan blocked, escalate'
      : worst === 'warn'
        ? 'FIELD WARN: plan permitted but physics exceeds PDDL estimate — surface to human'
        : 'FIELD PERMIT: field concurs with PDDL cost',
  };
}

// graph size is tracked in field-rust.ts; re-export a thin reader to avoid a circular import.
import { rustFieldSensitivity } from '../field-rust.ts';
async function graphSize(): Promise<number> {
  const s = await rustFieldSensitivity();
  return s.length;
}
