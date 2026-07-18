# V6 — NO-COURIER-SCORING CI gate misses `<prefix>_score` / `trust_weight` compounds

**File:** `scripts/ci-no-courier-scoring.sh` (32 lines, cited @ `b87b7e2`)
**Known-finding #6.** Verdict: **REPRODUCES** (bebop-repo's own copy carries the same gap).
**Severity: MEDIUM** (a defense-in-depth gate is porous; the *real* enforcement is the
type-level absence of score fields, which holds — see V-note below).

## Claim tested

> `claim_machine.rs:13-17` is the NO-COURIER-SCORING red-line enforcement point; the CI gate
> `ci-no-courier-scoring.sh` catches `score` but misses `trust_weight` / `integrity_score`.
> Check whether bebop-repo has its own copy with the same gap.

## Findings

### The enforcement point is clean

`claim_machine.rs:13-17` documents "Structural constraint enforced here and NOWHERE ELSE:
NO-COURIER-SCORING." The `ClaimStatus` enum (`:19-30`) is a plain 4-state lifecycle
(Offered/Claimed/Released/PickedUp) with **no** score/rating/trust field. Confirmed clean — no
finding against the machine itself.

### The CI gate has a word-boundary gap (reproduces)

`ci-no-courier-scoring.sh:21` flags field definitions matching:

```
\b(score|rating|reputation|rank|trust_score|trust_level|courier_score|agent_rating)\b
```

The `\b…\b` word-boundary anchoring is the hole. In Rust identifiers `_` is a **word
character**, so there is **no word boundary** between an underscore and an adjacent letter.
Therefore:

- `integrity_score` — `\bscore\b` requires a boundary immediately before `s`; the preceding char
  is `_` (a word char) → **no match**. Slips through.
- `trust_weight` — `weight` is not in the pattern, and `trust_weight` is not one of the listed
  compounds (`trust_score`/`trust_level`) → **no match**. Slips through.
- `mover_reputation`, `courier_ranking`, `agent_scoring` — same underscore-boundary evasion →
  **slip through**.

Only the **bare** words (`score`, `rating`, …) and the four **explicitly enumerated** compounds
(`trust_score`, `trust_level`, `courier_score`, `agent_rating`) are caught. Any *novel*
`<prefix>_<bannedword>` compound evades the gate. The G7 fix noted in the script header
(`:11-15`) closed a *different* hole (a `pub ` prefix breaking the line anchor); it did not touch
the `\b`-underscore hole.

### Same gap in bebop-repo (answering the task's explicit question)

This worktree **is** bebop-repo's tree, so `scripts/ci-no-courier-scoring.sh` here **is**
bebop-repo's own copy — and it carries the identical `\b`-anchored pattern. So: **yes**, the
bebop-repo copy has the same gap the dowiz-side audit found.

### Scope caveat

The gate also **excludes `bebop2/core/` entirely** (`:25`, `grep -vE '^bebop2/core/'`, and
`:26-29` explains: core is pure crypto/linear-algebra where `rank`/`score` are legitimate math
terms). That exclusion is defensible, but it means a courier-scoring field smuggled into a
struct *under `bebop2/core/`* would also be invisible to the gate.

## Why severity is MEDIUM, not HIGH

The gate is **defense-in-depth**, not the primary control. The primary control is architectural:
the claim/coordination types simply have no scoring field, and the mesh models identities +
verbs-on-objects, not ratings. So a real regression requires a developer to *both* add a
scoring-shaped field *and* name it with an evasive compound. The gate should still be fixed — its
value is catching exactly that accidental regression, and today it would wave `integrity_score`
right through.

## Remediation sketch

Replace the word-boundary alternation with a substring/stem match on the banned roots, e.g.
match any identifier containing `score|rating|reputation|rank|trust_weight|integrity` as a
segment (`(^|_)(score|rating|reputation|rank|weight)(_|$|:)` style), and add the evasive cases
(`integrity_score`, `trust_weight`, `mover_reputation`) to `scripts/test-no-courier-scoring.sh`
as RED fixtures so the gate is pinned against this exact class.
