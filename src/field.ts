// Bebop field — divergence (∇·F) and curl (∇×F)_z as a FUNDAMENTAL reasoning/search/actioning signal.
//
// Operator directive (2026-07-08): apply the basic laws of vector calculus — the 3 states
//   (pure divergence | pure curl | divergence+curl) — to model reasoning, vectorized search,
//   and actioning. Divergence = net outward flow (spread/explore vs converge/focus); curl = net
//   rotation (cycle/reconsider vs proceed). This is grounded in real physics, not a metaphor:
//   we compute the DISCRETE divergence and z-component of curl of the agent's natural
//   candidate-traversal flow field in the embedding plane.
//
//   3-state logic (the operator's law):
//     ∇·F > 0, (∇×F)_z = 0   → PURE DIVERGENCE  → generate / explore (draft a larger block)
//     ∇·F = 0, (∇×F)_z > 0   → PURE CURL         → reconsider / reflect (don't act yet)
//     ∇·F > 0, (∇×F)_z > 0   → DIVERGENCE+CURL   → generate AND reconsider
//   plus the degenerate states sink (∇·F<0) and stable (no clear field) → focus / single action.
//
// All math PURE + DETERMINISTIC (no RNG, no clock). Verified-by-Math: RED+GREEN in src/field.test.ts.

import { similarity, embed, type HyperVector } from './memory.ts';

export type FieldState = 'diverge' | 'rotate' | 'both' | 'sink' | 'stable';
export type FieldDirective = 'generate' | 'reconsider' | 'generate+reconsider' | 'focus';

export interface FieldPoint { x: number; y: number; s: number; }

export interface FieldAnalysis {
  div: number; // ∇·F  (net outward flow; >0 spread, <0 converge)
  curl: number; // (∇×F)_z (rotation; >0 circulates clockwise)
  state: FieldState;
  directive: FieldDirective;
}

// A discretized finite-difference field leaks a little of each component into the other
// (a "circle" traversal has a small inward divergence; a real source has tiny curl). We classify by
// RELATIVE DOMINANCE, not an absolute epsilon: an axis counts only if it is the dominant signal
// (≥1/3 of the larger magnitude). This is the honest way to read the 3-state law from samples, and
// it keeps the classifier falsifiable.
const DOMINANCE = 1 / 3;

/**
 * Discrete divergence & z-curl of the TRAVERSAL flow field.
 *
 * Candidates are ordered by DESCENDING similarity (the agent's natural visit order: most-alike
 * first). That order defines an open path p0→p1→…→p_{N-1} (an agent walks a PATH through
 * candidates; the reconsider signal is the curl sign, not a forced return-to-start). At each point
 * the flow vector is the step to the next candidate, weighted by its similarity. Each step is
 * decomposed into radial (outward from the query) and tangential (perpendicular) components:
 *
 *   divergence  = mean_i  s_i · (step_i · rhat_i)     // net outward flow
 *   curl_z      = mean_i  s_i · (step_i × rhat_i)_z   // rotation about the query
 *
 * A purely outward exploration gives high divergence, ~zero curl. A circular revisit pattern gives
 * high curl, ~zero divergence. Both together ⇒ divergence+curl. This is the exact 3-state law.
 */
export function fieldState(query: [number, number], candidates: FieldPoint[]): FieldAnalysis {
  if (candidates.length < 2) return { div: 0, curl: 0, state: 'stable', directive: 'focus' };
  const [qx, qy] = query;
  const ordered = [...candidates].sort((a, b) => b.s - a.s);
  const N = ordered.length;
  let div = 0, curl = 0;
  const last = N - 1;
  for (let i = 0; i < last; i++) {
    const p = ordered[i];
    const nxt = ordered[i + 1];
    const sx = nxt.x - p.x, sy = nxt.y - p.y; // step vector = the flow
    let rx = p.x - qx, ry = p.y - qy; // radial from query
    const rlen = Math.hypot(rx, ry) || 1;
    rx /= rlen; ry /= rlen; // rhat (unit radial)
    div += p.s * (sx * rx + sy * ry); // radial (divergent) component
    curl += p.s * (sx * ry - sy * rx); // z = sx*ry - sy*rx (tangential, signed)
  }
  div /= last; curl /= last;
  return classify(div, curl);
}

function classify(div: number, curl: number): FieldAnalysis {
  const mag = Math.max(Math.abs(div), Math.abs(curl), 1e-9);
  const divRel = Math.abs(div) / mag;
  const curlRel = Math.abs(curl) / mag;
  const divSig = divRel > DOMINANCE;
  const curlSig = curlRel > DOMINANCE;
  let state: FieldState;
  if (!divSig && !curlSig) state = 'stable';
  else if (curlSig && !divSig) state = 'rotate';
  else if (divSig && !curlSig) state = div < 0 ? 'sink' : 'diverge';
  else state = 'both'; // divergence AND rotation both dominate
  return { div, curl, state, directive: directiveFor(state) };
}

/** Map a field state to the actioning directive (how the loop should behave next). */
export function directiveFor(state: FieldState): FieldDirective {
  switch (state) {
    case 'diverge': return 'generate'; // spread → draft a larger block, explore
    case 'rotate': return 'reconsider'; // cycle → reflect, do not act yet
    case 'both': return 'generate+reconsider'; // spread + cycle → draft AND reflect
    case 'sink':
    case 'stable':
    default: return 'focus'; // converge / none → narrow, single action
  }
}

// ── VSA adapter: project candidate hypervectors into the 2D embedding plane via two FIXED seeds,
//    then analyze the traversal flow field. Deterministic — the seeds are content-derived, no RNG.

let _AX: HyperVector | null = null, _AY: HyperVector | null = null;
function axes(): [HyperVector, HyperVector] {
  if (!_AX) _AX = embed('field-axis-x::bebop');
  if (!_AY) _AY = embed('field-axis-y::bebop');
  return [_AX, _AY];
}

function dot(a: HyperVector, b: HyperVector): number {
  let s = 0;
  for (let i = 0; i < a.length; i++) s += a[i] * b[i];
  return s;
}

/** Analyze the field of a query hypervector against a set of candidate hypervectors (VSA recall). */
export function searchFieldState(
  query: HyperVector,
  candidates: HyperVector[],
): FieldAnalysis & { q: [number, number] } {
  const [AX, AY] = axes();
  const qx = dot(query, AX), qy = dot(query, AY);
  const pts: FieldPoint[] = candidates.map((c) => ({
    x: dot(c, AX), y: dot(c, AY), s: similarity(query, c),
  }));
  return { ...fieldState([qx, qy], pts), q: [qx, qy] };
}

/** Convenience: analyze the field of a query string against candidate strings (re-embedded). */
export function searchFieldStateText(query: string, candidates: string[]): FieldAnalysis {
  const [AX, AY] = axes();
  const q = embed(query);
  const qx = dot(q, AX), qy = dot(q, AY);
  const pts: FieldPoint[] = candidates.map((c) => {
    const v = embed(c);
    return { x: dot(v, AX), y: dot(v, AY), s: similarity(q, v) };
  });
  return fieldState([qx, qy], pts);
}

// ── MEMORY-FIELD ADAPTER: point the field law at the agent's own living memory. ──
// The recall field of a concept is the ordered set of nodes spreading-activation surfaces; it IS the
// traversal flow field. Divergence there = the memory fans OUT (explore more related concepts);
// curl = it CYCLES (reconsider / revisit); both = expand-and-revisit. This makes the physics the
// agent's introspection signal — run the new tool on the living-memory history itself.

export interface MemoryFieldResult extends FieldAnalysis {
  concept: string;
  spread: number; // how many distinct concepts the recall fans into
  depth: number; // max activation depth reached
}

/**
 * Run the ∇·F/∇×F law over the recall graph of `concept` in a LivingMemory store.
 * `recallHits` is the ordered list of (id, concept) the memory surfaces for `concept`
 * (most-similar first) — the traversal path. If empty, the field is `stable` (nothing to explore).
 */
export function memoryField(
  concept: string,
  recallHits: { id: string; concept: string }[],
): MemoryFieldResult {
  const spread = new Set(recallHits.map((h) => h.concept)).size;
  // The flow field lives in the VSA plane: project each recalled concept by content-address hash.
  const pts: FieldPoint[] = recallHits.map((h, i) => {
    const v = embed(h.concept);
    // step weight = decay rank (earlier recall = stronger flow); similarity used as `s`.
    const s = 1 - i / Math.max(1, recallHits.length);
    return { x: dot(v, axes()[0]), y: dot(v, axes()[1]), s };
  });
  const base = fieldState([0, 0], pts); // query is the origin for self-recall geometry
  return { ...base, concept, spread, depth: recallHits.length };
}
