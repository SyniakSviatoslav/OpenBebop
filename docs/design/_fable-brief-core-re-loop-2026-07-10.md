# FABLE ONE-SHOT ADVERSARIAL REVIEW — bebop core-reverse-engineering-loop (SELF-REVIEW OF THE HARNESS)

You are Claude Fable. Perform a SANCTIONED one-shot, READ-ONLY adversarial review of bebop's
`core-reverse-engineering-loop.sh` — the verification harness that supposedly "reverse-engineers the
deterministic math kernel down to bare metal + fundamental math axioms." Your job is to apply the SAME
adversarial lens the project turns on itself, but pointed at ITS OWN verification tool. The project's
own core principle (§0·GP) is: labels of falsifiability are applied faster than the property is
established, and its guardrail scripts verify LABELS not PROPERTIES. Hunt that failure mode INSIDE the
loop.

You have never seen this harness. Do NOT explore the filesystem beyond the exact files below — read
them directly. Do NOT run recursive searches. Produce your full report as your final message.

## Method (§0·GP ground-truth discipline)
- For EVERY finding cite `file:line` from the files you actually read.
- End each finding with a DETERMINISTIC follow-up (a gate/test to add, a claim to retract, or a
  broken check to fix). A finding without a follow-up is invalid.
- Read-only. Do not edit, commit, or write files.
- Be adversarial and decorrelated: assume the loop's own stdout ("✓ GREEN") may show confirmation bias.

## Files to read (read these and ONLY these):
- /root/bebop-repo/scripts/core-reverse-engineering-loop.sh
- /root/bebop-repo/docs/design/core-reverse-engineering-loop-2026-07-10.md
- /root/bebop-repo/docs/design/reverse-engineering-loop-2026-07-10.md
- /root/bebop-repo/docs/design/FABLE-FALLACY-CATALOG.md        (the prior fable review it should be consistent with)
- /root/bebop-repo/rust-core/src/lib.rs                          (the kernel the loop verifies — check the loop's claims against reality)
- /root/bebop-repo/crates/bebop/src/field.rs                    (process-global CSR claim target)

## The loop CLAIMS these load-bearing checks (VERIFY against the code, don't assume):
P1  machine-code extraction — disassembles 5 primitives; "symbol present in machine code"
P2  bit-level math axioms — sinc(0)=1, cosine(v,v)=1, cross(a,a)=0, dot(ortho)=0, proven at bit level
P3a determinism — no RNG/clock/network leak in rust-core; process-global CSR is the only mutable global
P3b process-global contract documented
P3c primitives are #[no_mangle] extern C
VERDICT — "GREEN: math axioms hold at bit level, machine code present, deterministic"

## Hunt specifically (the project's own known failure modes, applied to itself):
- FALSE PRECISION / APPEAL-TO-MATH: does P2 actually verify bebop's RUST implementation, or only a
  re-derived Python reference that NEVER calls the Rust code? Is the Python check a tautology?
- LABEL-WITHOUT-PROPERTY: P1 "symbol present in machine code" — does `grep` on `objdump` output prove
  the primitive is CORRECT, or merely that the string exists? Could a no-op/garbage function pass?
- CIRCULAR / SELF-SEALING: does P2's `cargo test` call "prove the axioms" or just re-run tests the
  loop itself does not inspect? Does the Python check independently corroborate Rust or just re-assert
  the math definition?
- CONFIRMATION BIAS (only GREEN shown): what would make the loop return RED? Is there a real RED path,
  or does `set -euo pipefail` + grep-on-stdout make it structurally unable to fail on a wrong kernel?
- EQUIVOCATION: "deterministic" / "bare metal" / "air-gapped" — does the loop mean different things in
  P1 vs P3? Does "machine code" mean Wasm bytecode (not native bare metal)?
- COMPOSITION: does "5 symbols present + 4 python asserts" actually establish the kernel is sound, or
  just that it compiles and basic identities hold?
- NAMED-BLIND-SPOT: the loop's own doc admits a RED self-catch on a grep-comment bug. Are there other
  grep-based checks that can false-match (e.g. P3a matches the doc-comment that NAMES the forbidden
  calls)? Does P3b's `grep -v "//|/*"` actually exclude all comment forms (block comments, doc-comments)?
- CONSISTENCY WITH PRIOR FABLE: does this loop contradict or overlap the CLOSED fallacies in
  FABLE-FALLACY-CATALOG.md (e.g. does it re-verify B4/B11/C2/B8, or claim coverage it doesn't have)?

## Produce this report:
### A. What the loop ACTUALLY proves (file:line) vs what it CLAIMS to prove (its own doc/stdout).
### B. LOGICAL FALLACIES & COGNITIVE BIASES in the harness — circular reasoning, false precision,
       confirmation bias, equivocation, composition, label-without-property. Cite file:line. Unflinching.
### C. PATTERNS — where the harness INCONSISTENTLY applies its own "falsifiable" discipline (e.g. a
       check that cannot fail, a GREEN that is structural not empirical).
### D. Deterministic follow-ups — for each B/C finding, a concrete checkable fix to the loop script.
Be concise, elitist, precise. Output the report now.
