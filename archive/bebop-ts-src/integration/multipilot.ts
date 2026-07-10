/**
 * multipilot.ts — brain-inside-brain multidimensional verification (tensor overlay of independent
 * verifier loops). Standing directive 2026-07-09: for any agentic artifact (reasoning / review /
 * reverse-engineering / research / planning) run ≥3 INDEPENDENT verifier loops in parallel and
 * overlay their verdicts as a tensor — disagreement is a dimension of risk.
 *
 * Why ≥3 (not 1–2): a single checker (copilot) is necessary but not sufficient — cycle-consistency's
 * self-inverse blind spot proves one verifier CAN be fooled. N≥3 *independent* axes catch failure
 * modes any one blind-spots. Independence is load-bearing: each loop MUST differ in method or model;
 * identical checkers add latency, not integrity. This is the next evolution of the as-above-so-below
 * checker (Cross-pattern A) and the multipilot universal rule.
 *
 * Tensor overlay: each axis emits a verdict ∈ {approve, revise, reject} (+ optional score). The
 * overlay is the vector of axis verdicts. `converged` = all axes approve (promote). `divergent` =
 * any axis rejects/revises → surface for human triage, NEVER silently averaged into a false "ok".
 *
 * Composable with existing seams: an axis can be `logicalCot.verifyLogicalPlan` (Logical), `redteam`
 * probe (Adversarial), `dualTrackGate`/contract oracle (Truth), `cycle-consistency` gap (Integrity).
 *
 * Deterministic: pure orchestration over injected verifier fns; no RNG/SDG/Date. Falsifiable
 * RED+GREEN. FLAG-OFF: inert unless a caller builds `loops[]` and calls `multipilot`.
 */

export type AxisVerdict = 'approve' | 'revise' | 'reject';

export interface PilotLoop {
  /** axis name — must be distinct across the set (independence enforced). */
  axis: string;
  /**
   * The verifier. Receives the artifact (already typed by the caller) and returns a verdict. MAY be
   * async. Pure: it should not mutate the artifact; a reject means "do not promote as-is".
   */
  verify: (artifact: unknown) => AxisVerdict | Promise<AxisVerdict>;
  /** optional numeric confidence ∈ [0,1] for the overlay heatmap. */
  score?: (artifact: unknown) => number;
}

export interface AxisResult {
  axis: string;
  verdict: AxisVerdict;
  score: number; // 1 if approve, 0.5 if revise, 0 if reject (for the overlay vector)
  /** human-readable note for the trace. */
  note: string;
}

export type Overlay = 'converged' | 'divergent';

export interface MultipilotReport {
  /** the tensor overlay: one entry per axis, in input order. */
  axes: AxisResult[];
  /** vector form of the overlay (per-axis 0/0.5/1) — the "tensor" the orchestrator reads. */
  vector: number[];
  overlay: Overlay;
  /** true iff every axis approved. */
  promote: boolean;
  /** axes that rejected/revised (the divergence dimensions) — surfaced for human triage. */
  dissent: AxisResult[];
  /** recommended action for the orchestrator. */
  action: 'promote' | 'triage';
  note: string;
}

const SCORE: Record<AxisVerdict, number> = { approve: 1, revise: 0.5, reject: 0 };

/**
 * Run N≥3 independent verifier loops over `artifact` in parallel and overlay their verdicts.
 * Throws (deterministically) if fewer than `minLoops` (default 3) distinct-axis loops are supplied —
 * independence is the whole point; a 1–2 loop call defeats the tensor.
 *
 * On divergence, the artifact is NOT promoted and the dissenting axes are listed — never averaged away.
 */
export async function multipilot(
  artifact: unknown,
  loops: PilotLoop[],
  opts: { minLoops?: number } = {},
): Promise<MultipilotReport> {
  const minLoops = opts.minLoops ?? 3;
  if (loops.length < minLoops) {
    throw new Error(`multipilot: need >=${minLoops} independent loops, got ${loops.length} (independence is the point)`);
  }
  // independence: axis names must be distinct
  const seen = new Set<string>();
  for (const l of loops) {
    if (seen.has(l.axis)) throw new Error(`multipilot: duplicate axis '${l.axis}' — loops must be independent`);
    seen.add(l.axis);
  }

  const raw = await Promise.all(
    loops.map(async (l) => {
      const verdict = await l.verify(artifact);
      const score = l.score ? l.score(artifact) : SCORE[verdict];
      return { axis: l.axis, verdict, score, note: `${l.axis}:${verdict}` } as AxisResult;
    }),
  );

  const vector = raw.map((r) => r.score);
  const dissent = raw.filter((r) => r.verdict !== 'approve');
  const converged = dissent.length === 0;
  const promote = converged;
  return {
    axes: raw,
    vector,
    overlay: converged ? 'converged' : 'divergent',
    promote,
    dissent,
    action: promote ? 'promote' : 'triage',
    note: converged
      ? `multipilot converged (${raw.length} axes approve) — promote`
      : `multipilot DIVERGENT (${dissent.length}/${raw.length} axes dissent: ${dissent.map((d) => d.axis).join(',')}) — triage, do not promote`,
  };
}

/**
 * Build the RECOMMENDED default 3-axis set for a plan/artifact, wiring the existing deterministic
 * seams as independent axes (Logical / Adversarial / Truth). Each is a distinct METHOD, so they
 * cannot collude. The caller supplies the concrete artifact-shaped closures.
 *
 * This is the "upgrade copilot → multipilot default" hook: any agentic surface calls
 * `defaultMultipilot(artifact, {...})` instead of a single checker.
 */
export function defaultMultipilot(
  artifact: unknown,
  hooks: {
    logical: (a: unknown) => AxisVerdict | Promise<AxisVerdict>;
    adversarial: (a: unknown) => AxisVerdict | Promise<AxisVerdict>;
    truth: (a: unknown) => AxisVerdict | Promise<AxisVerdict>;
  },
): Promise<MultipilotReport> {
  return multipilot(artifact, [
    { axis: 'logical', verify: hooks.logical },
    { axis: 'adversarial', verify: hooks.adversarial },
    { axis: 'truth', verify: hooks.truth },
  ]);
}
