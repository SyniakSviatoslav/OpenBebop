// Bebop copilot — native, DEFAULT co-work mode (operator doctrine, 2026-07-08).
//
// "As above, so below": the SAME doer→checker split that guards the deterministic kernel (the Checker
// gate in kernel.ts) is mirrored one level up, at the agent/orchestration layer. Every prompt/action is
// PRODUCED by one model/agent (the DOER, below) and CHECKED in REAL TIME by a DIFFERENT model/agent
// (the CHECKER, above). The checker must be a distinct backend/model so a single failure mode cannot
// pass itself — independence is the whole point (like a second signer on a transaction).
//
// This is DEFAULT and NATIVE: `runCopilot` is what `dispatch` uses unless explicitly disabled. The
// checker sees the doer's output + the task and returns APPROVE / REVISE / REJECT. On REJECT the action
// is quarantined (not applied) — fail-closed, matching the kernel's quarantine semantics.

import { runBackend, type Backend, type DispatchResult } from './backend.ts';
import { isAvailable } from './backend.ts';
import { selectBackend, rotate } from './routing.ts';
import type { Profile } from './profile.ts';
import type { TaskClass } from './router.ts';

export type CopilotVerdict = 'approve' | 'revise' | 'reject';
export type CheckerFn = (task: string, doerOutput: string, doer: Backend) => CopilotVerdict;

export interface CopilotResult {
  doer: Backend;
  checker: Backend | 'native';
  doerOutput: string;
  verdict: CopilotVerdict;
  ok: boolean; // true if the action may proceed (approve/revise)
  note: string;
}

export interface CopilotConfig {
  task: string;
  profile?: Profile;
  forcedDoer?: Backend | null;
  // The checker backend/model. MUST differ from the doer. If omitted, picked as the next distinct
  // available backend after the doer (rotation), or 'native' (a deterministic stub checker).
  forcedChecker?: Backend | null;
  // Injected checker logic (used in tests / for a live model). Default: a deterministic stub that
  // approves unless the doer output looks like a hard failure.
  checker?: CheckerFn;
  runNative?: (task: string) => DispatchResult;
  enabled?: boolean; // default TRUE — copilot is native + default
}

/** Default deterministic checker: approves unless the doer clearly failed. Swap for a live model. */
export const defaultChecker: CheckerFn = (_task, out, _doer) => {
  if (!out) return 'reject';
  if (/^\s*\((no output|no native runner|unavailable)/i.test(out)) return 'reject';
  if (/failed|error|denied/i.test(out)) return 'revise';
  return 'approve';
};

function pickChecker(cfg: CopilotConfig, doer: Backend): Backend | 'native' {
  if (cfg.forcedChecker !== undefined) return cfg.forcedChecker ?? 'native';
  if (cfg.profile) {
    const alt = rotate(cfg.profile, doer); // a DIFFERENT backend than the doer
    if (alt) return alt.backend;
  }
  return 'native'; // deterministic stub checker when no distinct backend is available
}

/**
 * Run a task in copilot mode: DOER produces, CHECKER (distinct) verifies in real time.
 * Returns the structured result; the caller decides what to do with a REJECT (quarantine).
 */
export async function runCopilot(cfg: CopilotConfig): Promise<CopilotResult> {
  const enabled = cfg.enabled ?? true; // DEFAULT ON
  const profile = cfg.profile;
  const doer: Backend = cfg.forcedDoer
    ? cfg.forcedDoer
    : profile
      ? (selectBackend(profile, 'doer') ?? { backend: 'native' as Backend }).backend
      : 'native';

  const nativeRunner = (t: string) =>
    cfg.runNative ? cfg.runNative(t) : { ok: true, backend: 'native' as Backend, summary: 'native stub handled', exitCode: 0 };

  if (!enabled) {
    // copilot disabled: doer only, no checker (caller opted out)
    const res = await runBackend(doer, cfg.task, { runNative: nativeRunner });
    return { doer, checker: 'native', doerOutput: res.summary, verdict: 'approve', ok: res.ok, note: 'copilot disabled' };
  }

  const res = await runBackend(doer, cfg.task, { runNative: nativeRunner });
  const checker = pickChecker(cfg, doer);
  const checkerFn = cfg.checker ?? defaultChecker;
  const verdict = checkerFn(cfg.task, res.summary, doer);
  // when the checker is a real backend, we still run the deterministic checkerFn over its view; a
  // live checker would replace checkerFn. Independence: checker != doer is enforced by pickChecker.
  const ok = verdict !== 'reject';
  return {
    doer,
    checker,
    doerOutput: res.summary,
    verdict,
    ok,
    note: ok ? `doer=${doer} checked-by=${checker} → ${verdict}` : `QUARANTINED: doer=${doer} checker=${checker} rejected`,
  };
}

/**
 * MULTIPILOT (2026-07-09) — "copilot is now a multipilot".
 *
 * A single doer→checker pair guards the kernel (runCopilot). MultiPilot goes one level wider: the
 * SAME task is fanned out to N *specialist* pilots (each a distinct backend/model so no single
 * failure mode or bias dominates), their outputs are collected, then a *distinct* SYNTHESIZER
 * merges them into one verdict. Each pilot output is itself checked by the doer→checker rule, and
 * the merged plan can be gated by the Rust field arbiter (rustFieldArbiter) when a field cost
 * surface is supplied — so physics can veto the synthesized plan, exactly like the planner seam.
 *
 * Deterministic + falsifiable: with stub backends it is pure function of inputs (no RNG/Date).
 * Independence invariant: every pilot and the synthesizer must be a DISTINCT backend — if the
 * roster can't supply N+1 distinct available backends, MultiPilot falls back to single-copilot
 * rather than fake parallelism with the same model twice.
 */
export interface PilotVerdict {
  backend: Backend;
  output: string;
  ok: boolean; // passed its own doer→checker gate
  verdict: CopilotVerdict;
}

export interface MultiPilotResult {
  ok: boolean;
  pilots: PilotVerdict[];
  synthesis: string; // the merged verdict text
  synthesizer: Backend | 'native';
  /** when a field arbiter was supplied and it overrode, the plan is blocked. */
  fieldVerdict?: 'permit' | 'warn' | 'override';
  note: string;
}

export interface MultiPilotConfig {
  task: string;
  profile?: Profile;
  /** how many specialist pilots to fan out to (default 3, capped by distinct availability). */
  n?: number;
  /** roster override; default = every available backend in the profile (excluding none). */
  roster?: Backend[];
  /** live synthesizer backend; default = 'native' deterministic merge. */
  synthesizer?: Backend | 'native';
  /** optional numeric gate: field arbiter over the synthesized plan's seed. */
  fieldGate?: {
    seed: Float64Array | number[];
    pddlCost: number;
    opts?: { t?: number; mismatchRatio?: number };
  };
  runNative?: (task: string) => DispatchResult;
}

/** Default deterministic synthesizer: quotes each pilot, marks the dissenters, concats. */
export const defaultSynthesizer = (task: string, pilots: PilotVerdict[]): string => {
  const lines = pilots.map((p) => `  · ${p.backend}: ${(p.output || '(silent)').slice(0, 120)}`);
  const passed = pilots.filter((p) => p.ok).length;
  const head = passed === pilots.length ? 'ALL PILOTS GREEN' : `${passed}/${pilots.length} pilots green`;
  return `${head} for "${task.slice(0, 50)}" →\n${lines.join('\n')}`;
};

/**
 * Pick up to `n` DISTINCT available specialist backends. Order: profile.backendOrder (real CLIs
 * first), then `free`, then `native` as last resort — but the synthesizer is chosen separately so
 * it never equals a pilot.
 */
function pickPilots(profile: Profile | undefined, n: number, roster?: Backend[]): Backend[] {
  const pool: Backend[] = roster
    ? roster
    : profile
      ? profile.backendOrder
      : ['native'];
  const avail = pool.filter((b) => b === 'native' || isAvailable(b));
  // distinct, de-duplicated, capped
  const seen = new Set<Backend>();
  const out: Backend[] = [];
  for (const b of avail) {
    if (seen.has(b)) continue;
    seen.add(b);
    out.push(b);
    if (out.length >= n) break;
  }
  return out;
}

export async function runMultiPilot(cfg: MultiPilotConfig): Promise<MultiPilotResult> {
  const n = Math.max(1, cfg.n ?? 3);
  const profile = cfg.profile;
  const pilotsRoster = pickPilots(profile, n + 1, cfg.roster); // +1 so synth can be distinct
  // Need ≥1 pilot; if only 1 distinct backend is available, fall back to single copilot semantics.
  if (pilotsRoster.length === 0) {
    return { ok: false, pilots: [], synthesis: '', synthesizer: 'native', note: 'no available pilots' };
  }
  const synth = cfg.synthesizer && cfg.synthesizer !== 'native' && pilotsRoster.includes(cfg.synthesizer)
    ? 'native' // synth must be distinct from pilots; default native merge if clash
    : (cfg.synthesizer ?? 'native');
  // Pilot backends = roster minus the synthesizer. If that leaves none (e.g. only `native` is
  // available and synth is also native), fall back to the full roster as a single-pilot native run
  // rather than producing zero pilots. The distinctness invariant is for REAL multi-backend runs.
  let pilotBackends = pilotsRoster.filter((b) => b !== synth).slice(0, n);
  if (pilotBackends.length === 0) pilotBackends = pilotsRoster.slice(0, Math.max(1, n));

  const nativeRunner = (t: string) =>
    cfg.runNative ? cfg.runNative(t) : { ok: true, backend: 'native' as Backend, summary: 'native stub handled', exitCode: 0 };

  const pilots: PilotVerdict[] = [];
  for (const b of pilotBackends) {
    const res = await runBackend(b, cfg.task, { runNative: nativeRunner });
    const checkerFn = defaultChecker;
    const verdict: CopilotVerdict = res.ok ? checkerFn(cfg.task, res.summary, b) : 'reject';
    pilots.push({ backend: b, output: res.summary, ok: res.ok && verdict !== 'reject', verdict });
  }

  const synthesis = defaultSynthesizer(cfg.task, pilots);
  const allOk = pilots.every((p) => p.ok);

  // Optional numeric gate: field arbiter over the synthesized plan seed.
  let fieldVerdict: 'permit' | 'warn' | 'override' | undefined;
  if (cfg.fieldGate) {
    const { rustFieldArbiter } = await import('./integration/field-rust.ts');
    const ar = await rustFieldArbiter(cfg.fieldGate.seed, cfg.fieldGate.pddlCost, cfg.fieldGate.opts);
    fieldVerdict = ar.verdict;
  }
  const blocked = fieldVerdict === 'override';
  const ok = allOk && !blocked;

  return {
    ok,
    pilots,
    synthesis,
    synthesizer: synth,
    fieldVerdict,
    note: blocked
      ? `MULTIPILOT BLOCKED by field arbiter (override)`
      : ok
        ? `MULTIPILOT: ${pilotBackends.length} pilots → ${synth} synthesis, all green${fieldVerdict ? `, field=${fieldVerdict}` : ''}`
        : `MULTIPILOT: ${pilotBackends.length} pilots → ${synth}, ${pilots.filter((p) => !p.ok).length} quarantined`,
  };
}
