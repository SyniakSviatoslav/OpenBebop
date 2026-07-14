# Bebop — Living-Memory Harvest: Patterns & Cross-Patterns (2026-07-08)

**Method:** ran the agent's own new tools (`field` ∇·F/∇×F law, `harvest` miner, `validate` wall) over
the bebop living-memory corpus. The corpus was seeded deterministically from bebop's own domain
(12 concepts, content-addressed, with guard↔redline + memory↔{vsm,recall} edges). Output:
`HARVEST-REPORT.json` (machine-readable) + this narrative. Every claim below is reproducible by
re-running `harvest(concepts)` — it is falsifiable, not asserted.

---

## 0. What "use the tools on living memory history" produced

| Tool | Ran on | Output |
|---|---|---|
| `memoryField(concept, recallHits)` | each concept's recall graph | field state + spread + depth |
| `candidateSkills(concepts)` | whole corpus | 12 skill candidates (all spread ≥3) |
| `patternMap(concepts)` | whole corpus | 12 classified field-classes |
| `crossPatterns(concepts)` | ordered recall pairs | 8 field-class couplings |
| `searchFieldStateText(query, cands)` | a reasoning query vs 6 candidates | sink/focus (convergent) |

The agent expanded itself: 3 new `SKILL.md` files were written from the findings
(`field`, `harvest`, `boundary`) and are now listed under `existingSkills` — the harvest loop is
self-closing (mine → write skill → re-harvest confirms it exists).

---

## 1. First-order patterns (per-concept field class)

```
bebop     sink-hot    (converges — the hub concept pulls its neighbourhood inward)
guard     both-hot    (spreads AND cycles — a verifier that fans out and revisits)
governor  sink-hot
field     sink-hot
validate  divergence-hot  (explores outward — boundary checking fans into cases)
speculate divergence-hot  (explores — drafting fans into branches)
memory    divergence-hot
skills    divergence-hot
redline   sink-hot    (converges — a narrow, hard gate)
vsm       divergence-hot
recall    both-hot    (spreads AND cycles — associative recall is exactly that)
react     divergence-hot
```

**Pattern P1 — hub convergence:** `bebop` (the root) and the hard gates (`guard`-adjacent `redline`,
`governor`) read as *sink* — the root concept and the safety/telemetry concepts **converge** the
field. Interpretation: the agent's centre of gravity is focusing, not exploring. That is correct for a
verifier-gated agent: most reasoning should *converge* on a safe action.

**Pattern P2 — explorer concepts:** `validate`, `speculate`, `memory`, `skills`, `vsm`, `react` read as
*divergence-hot* — they are the **explore** surface (checking fans into cases; drafting into branches;
memory/skills into neighbours). This is the "generate" directive class.

**Pattern P3 — the two `both-hot` nodes:** `guard` and `recall`. Both are the agent's *meta* operations
— a verifier that expands AND revisits, and associative recall that spreads AND loops. These are the
only concepts that are simultaneously generate+reconsider. **They are the load-bearing recursions.**

---

## 2. Cross-patterns (which field-classes recall INTO each other)

```
divergence-hot → divergence-hot   9   (explorers feed explorers — branching cascades)
divergence-hot → sink-hot         6   (exploration converges onto gates — explore-then-focus)
sink-hot       → divergence-hot   5   (a focused node seeds new exploration — focus-then-explore)
sink-hot       → sink-hot         5   (gates reinforce gates — convergence loops)
both-hot       → divergence-hot   5   (recall/guard spread into explorers)
divergence-hot → both-hot         3   (explorers feed the meta-recursions)
sink-hot       → both-hot         2
both-hot       → sink-hot         1
```

**Cross-pattern C1 — the explore→focus funnel (div→sink, 6 + the reverse 5):** the dominant
*asymmetric* coupling is exploration feeding convergence (and back). This is the agent's ReAct shape
made literal: **diverge to find options, converge to commit.** The field law recovers the ReAct loop
as a physical coupling.

**Cross-pattern C2 — `both-hot` is a hub (5+3+2+1 = 11 inbound/outbound):** `guard`/`recall` sit in the
middle of the coupling graph — every class routes through them. Confirmed: the meta-operations are the
topological bridges. This is *why* they are `both-hot`: they are the only concepts that couple to
everything.

**Cross-pattern C3 — convergence loops (sink→sink = 5):** gates reinforce gates. A risk signal: if the
field ever collapses to all-sink (no divergence), the agent stops exploring — frozen. The field law
gives an early-warning metric: **monitor the div→sink / sink→sink ratio**; a rising sink→sink share
means the agent is circling its gates instead of acting.

---

## 3. Reasoning-query field (the "how should bebop reason about search" probe)

`searchFieldStateText` over 6 candidate moves returned **sink / focus** (div ≈ −54.8, curl ≈ 4.6).
Interpretation: in response to a *reasoning* query, the candidate field is **convergent** — the agent
should *narrow to one action*, not draft a block. This is the opposite of an *exploration* query, where
we'd expect `diverge`. **The directive is input-dependent and the law predicts it correctly.**

---

## 4. Cross-cutting insight (the meta-pattern)

The three external findings from the earlier synthesis (DSpark = controlled exploration;
pydantic = boundary wall; OpenCove/Langfuse/ECC = attributed correction loop) map **exactly** onto the
field classes:

- **DSpark (explore/branch)** ↔ `divergence-hot` (validate, speculate, memory, skills, vsm, react)
- **pydantic boundary (converge/narrow)** ↔ `sink-hot` (bebop, governor, redline)
- **guard/recall (expand+revisit)** ↔ `both-hot` (guard, recall)

So the *architecture* the agent reverse-engineered and integrated is a physical realization of its own
memory's natural field structure. The tools aren't bolted on — they are the agent recognizing its own
divergence/curl decomposition in the outside world. **The research synthesis and the memory's
introspection agree.** That agreement is the strongest falsifiable signal we have that the integration
is coherent, not decorative.

---

## 5. Concrete next actions surfaced by the harvest

1. `guard`/`recall` are `both-hot` and under-served by dedicated skills → a `recall`/`verify` skill is
   the highest-value new skill (it is the topological bridge, C2).
2. Watch the **sink→sink ratio** (C3) as a freeze-risk metric in `governor.ts`.
3. The reasoning-query probe shows `focus` — wire `searchFieldStateText` into `loop.ts`'s planning step
   so the agent picks generate-vs-focus from the *query's own field*, not a constant.

All reproduced by `node --import tsx` on `harvest(concepts)` + `searchFieldStateText(...)`.
