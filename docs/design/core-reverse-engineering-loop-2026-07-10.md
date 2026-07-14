# Core Reverse-Engineering Loop — bebop math kernel (Wasm artifact + real properties)

> Upgrade of the source-level reverse-engineering loop to a **core** loop that verifies bebop's
> deterministic math kernel (`rust-core` / `bebop-core`) at the level of the Wasm artifact it ships
> and bebop's own executed axiom tests. Script: `scripts/core-reverse-engineering-loop.sh`.
> Run: `bash scripts/core-reverse-engineering-loop.sh`

## What it ACTUALLY proves (property-based, each with a real RED path)
- **P0** — Builds `rust-core` (`bebop-core`) to `wasm32-unknown-unknown --release` **inside the loop**
  (no stale-artifact blind spot). The wasm is the actual shipped artifact.
- **P1** — Parses the Wasm binary (no external tooling; a tiny LEB128 walker):
  - the **Import section is EMPTY** ⇒ the module imports nothing ⇒ at the machine level it **cannot
    reach a clock / RNG / socket**. This is the one check that converts the "machine code" rhetoric
    into a real machine-code property (fable D3b).
  - the **5 math primitives are EXACT exported function names** (`vsa_similarity`, `cosine_similarity`,
    `cross_product`, `sinc`, `field_build`) — word-anchored, so `field_build_f32` cannot satisfy the
    check by substring (fable D2/F3).
- **P2** — `cargo test -p bebop-core` runs bebop's **own** axiom tests and the loop greps the NAMED
  tests (`test_sinc_singularity_and_zero`, `test_cosine_similarity_bounds`,
  `test_cross_product_orthogonality`, `test_vsa_self_similarity_is_dim`). Deleting or breaking one
  turns the loop RED (fable D5). This executes real Rust code — no Python re-derivation of the formula.
- **P3** — Counts the process-global mutable state exactly: **two** `static Mutex` globals (`STATE`,
  `ACCUM`). `ACCUM` carries Δu history across calls, so the kernel is stateful across propagations —
  the loop states this honestly rather than claiming "pure determinism" (fable D4/F5/F7).

## Why this is the "upgrade" the operator asked for
The source-level loop enumerates modules and classifies leaks. The **core** loop goes to the artifact:
it builds the Wasm, parses its export/import structure, and runs bebop's executed math axioms. This is
the "bare-metal + fundamental math principles" layer made **empirical** — and, after fable's
adversarial self-review (below), every check now has a genuine RED path.

## Fable adversarial self-review (2026-07-10) — the loop was itself fallacious
`claude --model fable` reviewed the loop and found it committed the repo's own named meta-fallacy:
*it verified labels, not properties*. Key findings (all fixed in the current script):
- **F1/F2** — the Python "axioms" were tautologies that never called Rust/wasm, and a failed axiom
  printed `✗` but incremented no counter (so GREEN could print with a failure on screen). → removed;
  replaced by named Rust-test greps that feed the verdict.
- **F3** — `grep "field_build"` matched `field_build_f32` (delete the real fn → still GREEN); fallback
  grepped the whole binary. → exact export-name parse.
- **F4** — the determinism grep excluded any line with `*` (so `use rand::prelude::*;` and
  `SystemTime::now(); // seed` were invisible) and omitted `Instant`/`getrandom`/`UdpSocket`. →
  replaced by the empty-Import-section check (stronger, tooling-free).
- **F5** — grepped the word `PROCESS-GLOBAL` while rust-core actually has TWO mutable globals
  (`STATE`, `ACCUM`) and `ACCUM` is stateful. → exact global count + honest stateful note.
- **F6** — `cargo test` ✓ was printed by `awk`, not counted; the "8/8" was only the greps. → cargo
  results routed through `ok()`/`bad()`.
- **F7/F9** — equivocated "bare metal" (Wasm is VM bytecode, never executed) / "air-gapped" /
  "deterministic"; doc claimed a P4 constant-time check that didn't exist and said "4 primitives".
  → reworded; counts fixed.
- **F10** — stamped "math axioms hold" over the OPEN `fexp` range-reduction bug (catalog C8) in the
  hottest math path. → noted as follow-up; the loop does not claim coverage of C8.

The full fable report: `/tmp/fable-review-core-re-loop-2026-07-10.md`. The corrected loop is the
repo's answer to its own §0·GP principle applied to itself.

## Status
- `scripts/core-reverse-engineering-loop.sh` (committed) — GREEN 8/8 with real RED paths (RED-path
  probe: renaming a test target → loop exits 1).
- All 294 workspace tests pass; guardrail 310/310 falsifiable; doc-claims green.
- This loop is the deterministic, deep, total-scope twin of the fable adversarial review: fable reads
  *logic*; the core loop reads the *shipped artifact + executed axioms*. After fable's self-review,
  both now check properties, not labels.
