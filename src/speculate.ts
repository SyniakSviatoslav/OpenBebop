// Bebop speculate — semi-autoregressive AGENTIC generation.
//
// Reverse-engineered from DeepSeek "DSpark: Confidence-Scheduled Speculative Decoding with
// Semi-Autoregressive Generation" (arXiv:2607.05147, submitted 2026-07-06).
//
// The paper accelerates LLM token decoding with two ideas. We port them to an AGENTIC
// tool-call loop, where the scarce resource is not token throughput but LLM ROUND-TRIPS:
//
//   1. SEMI-AUTOREGRESSIVE DRAFT
//      A parallel "backbone" proposes a BLOCK of N candidate actions in ONE model call
//      (no inter-action dependency → fast but suffix decay). A lightweight "sequential
//      module" adds the inter-action dependency the backbone lacks, recovering the prefix
//      a pure parallel drafter would lose. This is the anti-suffix-decay mechanism.
//
//   2. CONFIDENCE-SCHEDULED VERIFICATION
//      The drafted block is checked by a VERIFIER that is authoritative over the draft
//      (in DSpark: the target model; in bebop: the DETERMINISTIC GUARD + eval gate). We
//      also adapt HOW MANY candidates we draft per request from an estimate of prefix
//      survival and engine throughput — don't burn batch capacity on doomed tokens.
//
// WHY IT FITS bebop: the loop spends one LLM round-trip per tool call. Drafting a block of
// candidate actions in one call and verifying them through the guard in one pass cuts
// round-trips (DSpark reports 60–85% faster generation at matched throughput). The guard
// stays the SOLE trust boundary — the draft is speculative until verified, exactly as the
// paper's target model verification overrides the draft.
//
// All math here is PURE and DETERMINISTIC (no RNG, no clock). Verified-by-Math: every
// property has a falsifiable RED+GREEN case in src/speculate.test.ts.

export const DRAFT_BLOCK_LEN = 4; // how many candidate actions a draft proposes by default
export const ACCEPT_FLOOR = 0.5; // min prefix-survival probability to accept a prefix
export const HEAD_TEMP = 0.6; // backbone temperature at block head (seasoned, high-prob)
export const TAIL_TEMP = 0.9; // backbone temperature at block tail (untried, exploratory)

/** A single drafted candidate action with the backbone's independent acceptance confidence. */
export interface DraftCandidate {
  name: string;
  conf: number; // ∈ [0,1] — backbone acceptance probability (independent, parallel drafter)
}

/**
 * Confidence schedule: a per-position temperature from head (cool) to tail (warm). This is a
 * deterministic stand-in for DSpark's learned scheduler; the interface (len → temperatures[])
 * is exactly what a learned/profile-aware scheduler would replace without touching callers.
 */
export function confidenceSchedule(
  len: number,
  headTemp = HEAD_TEMP,
  tailTemp = TAIL_TEMP,
): number[] {
  if (len <= 0) return [];
  if (len === 1) return [headTemp];
  const out: number[] = [];
  for (let i = 0; i < len; i++) {
    const t = i / (len - 1); // 0 at head, 1 at tail
    out.push(headTemp + (tailTemp - headTemp) * t);
  }
  return out;
}

/**
 * Semi-autoregressive combine. The backbone proposes candidates with INDEPENDENT confidences
 * (a pure parallel drafter). The sequential module adds an inter-action dependency boost that
 * grows toward the tail — later actions benefit most from the dependency model, which is exactly
 * where a parallel drafter's suffix decay is worst. Returns the surviving-prefix acceptance
 * probability for each prefix length k (1..len) and the accepted prefix length.
 */
export function semiAutoDraft(
  candidates: DraftCandidate[],
  opts: { depBoost?: number; floor?: number } = {},
): { prefixSurvival: number[]; acceptedLen: number } {
  const floor = opts.floor ?? ACCEPT_FLOOR;
  const depBoost = opts.depBoost ?? 0.12; // sequential-module dependency contribution
  const len = candidates.length;
  const prefixSurvival: number[] = [];
  let prod = 1;
  for (let k = 1; k <= len; k++) {
    const i = k - 1;
    // dependency term grows toward the tail (normalized by block length)
    const dep = Math.min(0.49, depBoost * (i / Math.max(1, len - 1)));
    const p = Math.min(1, candidates[i].conf + dep);
    prod *= p;
    prefixSurvival.push(prod);
  }
  // accept the longest prefix whose survival clears the floor
  let acceptedLen = 0;
  for (let k = 1; k <= len; k++) if (prefixSurvival[k - 1] >= floor) acceptedLen = k;
  return { prefixSurvival, acceptedLen };
}

/**
 * Pure parallel-drafter baseline (NO sequential module). Used as the RED contrast: accepted
 * length decays fast because independent confidences multiply with no dependency recovery.
 */
export function parallelDraft(candidates: DraftCandidate[], floor = ACCEPT_FLOOR): number {
  let prod = 1;
  let acceptedLen = 0;
  for (let k = 1; k <= candidates.length; k++) {
    prod *= candidates[k - 1].conf;
    if (prod >= floor) acceptedLen = k;
  }
  return acceptedLen;
}

/**
 * The VERIFIER — the real trust boundary. Runs the drafted block through a verify predicate
 * (the guard, in the loop). Returns how many of the drafted candidates ACTUALLY verify: the
 * ground truth that overrides the draft's survival estimate, exactly as DSpark's target model
 * is authoritative over the draft.
 */
export function verifyBlock<T>(
  drafted: T[],
  verify: (t: T, index: number) => boolean,
): { verifiedLen: number; rejectedAt: number | null } {
  let verifiedLen = 0;
  let rejectedAt: number | null = null;
  for (let i = 0; i < drafted.length; i++) {
    if (verify(drafted[i], i)) verifiedLen++;
    else {
      rejectedAt = i;
      break;
    }
  }
  return { verifiedLen, rejectedAt };
}

/**
 * Confidence-scheduled verification length: adapt the draft block size from an estimate of prefix
 * survival and the engine's throughput profile. Short blocks when survival is low (don't waste
 * batch capacity on doomed tokens); longer when the draft is trustworthy.
 */
export function scheduleVerificationLength(
  estimatedSurvival: number, // 0..1, from the last draft's acceptedLen/len
  engineThroughput = 1, // relative capacity the verifier can absorb
  minLen = 1,
  maxLen = DRAFT_BLOCK_LEN * 2,
): number {
  const s = Math.max(0, Math.min(1, estimatedSurvival));
  const tp = Math.max(0, Math.min(1, engineThroughput));
  const len = Math.round(minLen + (maxLen - minLen) * s * tp);
  return Math.max(minLen, Math.min(maxLen, len));
}

export interface SpeculateResult<T> {
  drafted: T[];
  draftedLen: number;
  acceptedLen: number; // semi-autoregressive survival-based accept length
  verifiedLen: number; // ground-truth accept length from the verifier
  rejectedAt: number | null;
  roundTripsSaved: number; // draftedLen - 1: one call drafted them all
}

/**
 * End-to-end propose+verify for one agentic step. The `drafter` returns a block of candidate
 * actions (the parallel backbone + sequential module combined into the proposal); the `verifier`
 * is the guard/eval predicate that decides truth. Returns the verified prefix for the loop to run.
 * Round-trip savings = draftedLen - 1 (the whole block came from ONE model call).
 */
export function proposeStep<T>(
  drafter: () => T[],
  verifier: (t: T, index: number) => boolean,
): SpeculateResult<T> {
  const drafted = drafter();
  const draftedLen = drafted.length;
  const { verifiedLen, rejectedAt } = verifyBlock(drafted, verifier);
  // Semi-autoregressive survival estimate: chain the verifier's booleans as confidences.
  const confs: DraftCandidate[] = drafted.map((_, i) => ({
    name: String(i),
    conf: i < verifiedLen ? 1 : 0.01,
  }));
  const { acceptedLen } = semiAutoDraft(confs);
  return {
    drafted,
    draftedLen,
    acceptedLen,
    verifiedLen,
    rejectedAt,
    roundTripsSaved: Math.max(0, draftedLen - 1),
  };
}
