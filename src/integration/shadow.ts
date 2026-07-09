/**
 * shadow.ts — shadow-mode composition of the proven-but-flag-OFF seams (logicalCot + dualTrack +
 * validate). Per Universal rule "Flag-OFF → shadow → gate", this runs them NON-BLOCKING over an
 * agent's proposed action so we can observe their false-positive rate before any of them is promoted
 * to a hard gate. It never rejects; it only records what each axis would have said.
 *
 * This is the bridge from "proven in isolation" to "safe to promote": the loop calls `shadowVerify`
 * on every proposed tool call; each seam is a distinct *axis* (structural/logical, graph/truth,
 * well-formedness). The returned `ShadowReport` is the tensor overlay the multipilot (and the
 * operator) reads. Promotion to gate happens per-flag after shadow proves low FP (Cross-pattern C).
 *
 * Deterministic, pure, no IO. FLAG-OFF: runLoop only calls this when cfg.shadowVerify is set.
 */
import { validateToolArgs } from '../validate.ts';
import { verifyLogicalPlan, type LogicalStep } from './logicalCot.ts';
import { dualTrackGate, type TruthGraph, type GnnAdvisor } from './analytics/dual-track.ts';
import type { ToolName } from '../loop.ts';

export interface ShadowReport {
  /** which axes fired (for ops visibility). */
  axes: {
    validate?: { ok: boolean; reason?: string };
    logical?: { ok: boolean; message: string };
    dualTrack?: { honored: boolean; reason: string };
  };
  /** would ANY axis have blocked this action? (informational only — shadow never blocks). */
  wouldBlock: boolean;
  /** human-readable shadow trace line. */
  note: string;
}

/**
 * Run the three seams over a proposed tool call. `logicalSteps` lets the caller supply a PDDL-INSTRUCT
 * proof for the action (the executor's LogicalStep[]); `graph`/`advisor` wire the dual-track truth
 * layer. Any omitted axis is skipped (so a caller can shadow just one seam). Pure + deterministic.
 */
export function shadowVerify(
  tool: ToolName,
  args: unknown,
  opts: {
    logicalSteps?: LogicalStep[];
    graph?: TruthGraph;
    advisor?: GnnAdvisor;
    focus?: string;
  } = {},
): ShadowReport {
  const axes: ShadowReport['axes'] = {};

  // Axis 1 — well-formedness (Pydantic boundary).
  const v = validateToolArgs(tool, args as Record<string, unknown>);
  axes.validate = v.ok ? { ok: true } : { ok: false, reason: v.reason };

  // Axis 2 — logical CoT (preconditions/effects/invariants), if a proof was supplied.
  if (opts.logicalSteps && opts.logicalSteps.length) {
    const lp = verifyLogicalPlan({} as Record<string, never>, opts.logicalSteps);
    axes.logical = { ok: lp.ok, message: lp.message };
  }

  // Axis 3 — dual-track truth-layer (graph consistency), if a graph/advisor was supplied.
  if (opts.graph && opts.advisor && opts.focus) {
    const dt = dualTrackGate(opts.graph, opts.advisor, opts.focus);
    axes.dualTrack = { honored: dt.honored, reason: dt.reason };
  }

  const wouldBlock = !!(
    axes.validate && !axes.validate.ok ||
    (axes.logical && !axes.logical.ok) ||
    (axes.dualTrack && !axes.dualTrack.honored)
  );
  const parts: string[] = [];
  if (axes.validate) parts.push(`validate:${axes.validate.ok ? 'ok' : 'REJECT'}`);
  if (axes.logical) parts.push(`logical:${axes.logical.ok ? 'ok' : 'REJECT'}`);
  if (axes.dualTrack) parts.push(`dualTrack:${axes.dualTrack.honored ? 'ok' : 'REJECT'}`);
  return { axes, wouldBlock, note: `shadow[${tool}] ${parts.join(' · ') || 'no axes'}${wouldBlock ? ' → WOULD BLOCK (shadow only)' : ''}` };
}
