// Bebop harvest — turn the agent's own living-memory + skills corpus into NEW tooling.
//
// Operator directive (2026-07-08): "use tools on living memory history to expand the existing tools,
// systems, create new skills. After analyze everything for patterns and cross-patterns."
//
// This module is the instrument that DOES that. It runs the field law (src/field.ts) and structural
// detectors over the real corpus and emits:
//   • skill candidates (concepts whose recall field is `both`/high-spread → write a SKILL.md)
//   • pattern report (recurring shapes + cross-pattern couplings)
//
// PURE + DETERMINISTIC (no RNG, no clock). Verified-by-Math: RED+GREEN in src/harvest.test.ts.

import { memoryField, type MemoryFieldResult, type FieldState } from './field.ts';
import { loadSkills, type Skill } from './skills.ts';
import { livingMemory } from './memory.ts';

// ── corpuses we mine ──

export interface RecallHit { id: string; concept: string; }

/** Replay a concept through the live memory store and return its ordered recall hits.
 *  Uses content-address spreading-activation first (deterministic, no hallucination); if that
 *  surfaces nothing, falls back to VECTOR-SIMILARITY nearest() — the associative recall that the
 *  field law actually wants (a concept's neighbourhood in the embedding plane). */
export function recallField(concept: string): RecallHit[] {
  const m = livingMemory();
  const activated = m.recall(concept, 3, 0.5);
  if (activated.length >= 2) {
    return activated
      .map((id) => m.node(id))
      .filter((n): n is NonNullable<typeof n> => !!n)
      .map((n) => ({ id: n.id, concept: n.concept }));
  }
  // associative fallback: the k nearest concepts by Hamming similarity
  return m.nearest(concept, 4)
    .filter((x) => x.sim > 0.5) // VSA_NOISE_FLOOR — never present noise as confident
    .map((x) => ({ id: x.id, concept: m.node(x.id)?.concept ?? '' }))
    .filter((h) => h.concept !== '');
}

// ── skill-candidate detector ──

export interface SkillCandidate {
  concept: string;
  field: MemoryFieldResult;
  reason: string; // why this concept warrants a skill
}

/**
 * A concept earns a skill when its recall field shows EVOLVING structure:
 *   • `both` (divergence+curl) → it spreads AND cycles → a reusable procedure is implied
 *   • OR spread ≥ MIN_SPREAD distinct concepts → a real neighbourhood exists to document
 * Otherwise: not yet (memory too thin) — honest, no fabrication.
 */
const MIN_SPREAD = 3;

export function candidateSkills(concepts: string[]): SkillCandidate[] {
  const out: SkillCandidate[] = [];
  for (const c of concepts) {
    const hits = recallField(c);
    if (hits.length < 2) continue; // nothing to learn from
    const f = memoryField(c, hits);
    const reasons: string[] = [];
    if (f.state === 'both') reasons.push('recall field is divergence+curl (spreads and cycles)');
    if (f.spread >= MIN_SPREAD) reasons.push(`fans into ${f.spread} distinct concepts`);
    if (reasons.length === 0) continue;
    out.push({ concept: c, field: f, reason: reasons.join('; ') });
  }
  return out;
}

// ── pattern + cross-pattern analysis ──

export type PatternKind =
  | 'divergence-hot'   // concept whose field diverges → generate/explore downstream
  | 'curl-hot'         // concept whose field curls → reconsider downstream
  | 'both-hot'         // concept that both spreads and cycles
  | 'sink-hot'         // concept that converges (focus)
  | 'isolated';        // concept with no recall (thin memory)

export interface Pattern {
  concept: string;
  kind: PatternKind;
  state: FieldState;
  support: number; // number of concepts in this pattern class
}

/**
 * Pattern map: classify every mined concept by its field state. The map itself is the FIRST-order
 * pattern (which concepts behave which way). Cross-patterns (below) couple classes.
 */
export function patternMap(concepts: string[]): Pattern[] {
  return concepts.map((c) => {
    const hits = recallField(c);
    if (hits.length < 2) return { concept: c, kind: 'isolated' as PatternKind, state: 'stable' as FieldState, support: 0 };
    const f = memoryField(c, hits);
    const kind: PatternKind =
      f.state === 'both' ? 'both-hot'
        : f.state === 'rotate' ? 'curl-hot'
          : f.state === 'sink' ? 'sink-hot'
            : f.state === 'diverge' ? 'divergence-hot'
              : 'isolated';
    return { concept: c, kind, state: f.state, support: f.spread };
  });
}

export interface CrossPattern {
  a: PatternKind;
  b: PatternKind;
  count: number; // how many concept-pairs couple these two classes (recall-overlap)
  note: string;
}

/**
 * CROSS-PATTERN: which field-classes tend to recall INTO each other. We measure, for every ordered
 * pair of concepts, whether concept A's recall set contains a concept whose OWN class is B; that
 * co-occurrence is a coupling. The dominant couplings are the cross-patterns (e.g. divergence-hot
 * concepts feeding curl-hot ones ⇒ "explore then reflect" loops).
 */
export function crossPatterns(concepts: string[]): CrossPattern[] {
  const pats = new Map<string, Pattern>();
  for (const p of patternMap(concepts)) pats.set(p.concept, p);
  const couple = new Map<string, number>();
  for (const a of concepts) {
    const aHits = recallField(a);
    for (const h of aHits) {
      const bPat = pats.get(h.concept);
      if (!bPat || bPat.concept === a) continue;
      const key = `${pats.get(a)!.kind}->${bPat.kind}`;
      couple.set(key, (couple.get(key) ?? 0) + 1);
    }
  }
  const notes: Record<string, string> = {
    'divergence-hot->curl-hot': 'explore-then-reflect loops: spreading begets revisiting',
    'both-hot->both-hot': 'self-similar recursion: structured concepts recall structured concepts',
    'curl-hot->divergence-hot': 'reflect-then-explore: revisiting seeds new spread',
    'isolated->isolated': 'no coupling (memory too thin)',
  };
  return [...couple.entries()]
    .map(([k, count]) => {
      const [a, b] = k.split('->') as [PatternKind, PatternKind];
      return { a, b, count, note: notes[k] ?? 'coupling observed' };
    })
    .sort((x, y) => y.count - x.count);
}

// ── full harvest report (the tool you run on the living-memory history) ──

export interface HarvestReport {
  concepts: string[];
  candidates: SkillCandidate[];
  patterns: Pattern[];
  cross: CrossPattern[];
  existingSkills: string[]; // names of skills already on disk (so we don't duplicate)
}

export function harvest(concepts: string[], skills?: Skill[]): HarvestReport {
  const existingSkills = (skills ?? loadSkills()).map((s) => s.name);
  return {
    concepts,
    candidates: candidateSkills(concepts),
    patterns: patternMap(concepts),
    cross: crossPatterns(concepts),
    existingSkills,
  };
}
