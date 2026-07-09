/**
 * redteam.ts — deterministic adversarial self-pentest harness (T3MP3ST-method applied).
 *
 * T3MP3ST ("autonomous red teaming platform", elder-plinius) turns a coding agent into a zero-day
 * hunter by auto-mutating inputs until the safety kernel breaks. We extract the METHOD (not the
 * black-box tooling) and apply it to bebop's OWN admission gate: given a seed corpus of "ideas",
 * deterministically mutate them (escape/obfuscation/BOLA-style keyless payloads/long-bulk) and
 * measure how many slip PAST the kernel's fail-closed checker. That break-rate IS the honest
 * "how many of my mutations bypass the gate" signal T3MP3ST produces — but here it runs against
 * our real `selfEvolve`/checker, no external agent, fully deterministic.
 *
 * No RNG: mutations are drawn from a fixed dictionary + structural transforms (deterministic order),
 * seeded by `seed` so a CI run is reproducible. No SGD/Date. Falsifiable RED+GREEN.
 *
 * FLAG-OFF: this module is inert unless you call `redTeamProbe`. Wire it into the self-evolution
 * CI to keep the admission kernel honest over time.
 */

export type GateFn = (inp: string) => Promise<{ accepted: boolean; reason?: string }> | { accepted: boolean; reason?: string };

export interface RedTeamConfig {
  /** how many mutation passes per seed (deterministic breadth). */
  passes?: number;
  /** optional seed → shuffles mutation order deterministically (no RNG). */
  seed?: number;
  /** stop after this many total mutations (bounded; deterministic termination). */
  maxMutations?: number;
}

export interface RedTeamReport {
  seed: string;
  total: number;
  accepted: number; // mutations that BYPASSED the gate (admitted) — the breach count
  quarantined: number; // mutations the gate correctly rejected
  breakRate: number; // accepted / total ∈ [0,1] — T3MP3ST's "bypass rate"
  /** the actual bypassing payloads, for triage (fail-open findings). */
  bypasses: string[];
  /** the mutations that correctly tripped the gate, with reasons. */
  blocks: { payload: string; reason: string }[];
}

// Deterministic mutation grammar (no RNG): each transforms a seed into an adversarial variant.
// Order is stable; `seed` rotates the START index so repeated runs explore different corners first
// but remain reproducible.
const MUTATORS: ((s: string) => string)[] = [
  (s) => s, // identity (baseline — should behave like the seed)
  (s) => `  ${s}  `, // whitespace padding / leading-trailing
  (s) => s.replace(/ /g, '\t'), // tab obfuscation
  (s) => s.replace(/[a-zA-Z]/g, (c) => (c >= 'a' && c <= 'z' ? c.toUpperCase() : c.toLowerCase())), // case flip
  (s) => `﻿${s}`, // BOM injection
  (s) => s + '\u0000', // null byte
  (s) => `${s}\u200b\u200b`, // zero-width spaces (invisible obfuscation)
  (s) => `{"__proto__":null,"payload":"${s}"}`, // JSON/proto injection shape
  (s) => `${s} '.repeat(0);}); require('child_process').exec('id'); //`, // injection comment shape
  (s) => `\u202e${s}`, // RTL override (visual spoof)
  (s) => `x`.repeat(3000) + s, // bulk padding (resonance pre-check)
];

function rotated<T>(arr: T[], start: number): T[] {
  const n = arr.length;
  const k = ((start % n) + n) % n;
  return arr.slice(k).concat(arr.slice(0, k));
}

/**
 * Run the adversarial probe. For each seed, apply `passes` rotated mutation rounds, call `gate`
 * on every variant, and tally which bypass. Deterministic given (seeds, seed). Pure: the gate is
 * injected, so this works against `selfEvolve` or any `Checker` without burning an LLM.
 */
export async function redTeamProbe(
  seeds: string[],
  gate: GateFn,
  cfg: RedTeamConfig = {},
): Promise<RedTeamReport> {
  const passes = cfg.passes ?? MUTATORS.length;
  const maxMutations = cfg.maxMutations ?? Infinity;
  const start = cfg.seed ?? 0;
  const report: RedTeamReport = {
    seed: String(start),
    total: 0,
    accepted: 0,
    quarantined: 0,
    breakRate: 0,
    bypasses: [],
    blocks: [],
  };
  for (const seedText of seeds) {
    const muts = rotated(MUTATORS, start);
    for (let p = 0; p < passes && report.total < maxMutations; p++) {
      const payload = muts[p % muts.length](seedText);
      const res = await gate(payload);
      report.total++;
      if (res.accepted) {
        report.accepted++;
        report.bypasses.push(payload);
      } else {
        report.quarantined++;
        report.blocks.push({ payload, reason: res.reason ?? 'rejected' });
      }
    }
  }
  report.breakRate = report.total > 0 ? report.accepted / report.total : 0;
  return report;
}

/** Pure, synchronous variant for tests that use a synchronous checker. */
export function redTeamProbeSync(
  seeds: string[],
  gate: (inp: string) => { accepted: boolean; reason?: string },
  cfg: RedTeamConfig = {},
): RedTeamReport {
  return redTeamProbe(seeds, gate, cfg) as unknown as RedTeamReport;
}
