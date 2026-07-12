# Global Logic Laws — truth gates for every claim

> Status: ENFORCED (pre-commit + CI via `scripts/logic-gate.mjs`).
> Companion doc: `docs/design/ESCALATIONS.md` (human-arbitrated resolutions).

This document is the **single source of truth** for the logical gates that every
documentation statement, roadmap claim, and code-level "verified" assertion in
this repository must satisfy. It is derived from **peer-reviewed / canonical
sources**, not invented here. Each law names its source.

## 0. Enforcement model (how a claim is "true" here)

A claim `C` in any `.md` (README, AGENTS, `docs/**`, `bebop2/**`) is admitted
only if ONE of:

- **Grounded** — `C` is backed by a concrete artifact in-repo: a `#[test]` path,
  a `scripts/*.mjs` probe, a KAT/ACVP vector file, or a cited source `[…](url)`.
- **Escalated** — `C` is mathematically/logically unprovable *or* self-referential
  (paradox). It is then logged in `ESCALATIONS.md` with a unique `ESC-<id>` and
  resolved by a **human arbiter** (the operator or a designated user). It is NEVER
  silently dropped and NEVER auto-fabricated.

Exit contract of `logic-gate.mjs`:
- `0` — all claims grounded, no contradiction. Commit allowed.
- `1` — **hard logical violation** (direct contradiction, or a deleted canonical
  component). Commit **refused**.
- `2` — one or more claims need human arbitration (unbacked / paradox). Commit
  **allowed**, but an `ESC-<id>` entry is written and must be resolved.

## 1. Law of Identity — `A = A`
- **Source:** Aristotle (*Metaphysics* Γ); Leibniz.
- **Formal:** `∀x (x = x)`; propositionally `p → p`.
- **Gate:** a term must mean the same thing everywhere it appears. Renaming a
  component without updating its references is an identity violation and is
  caught by the build/test layer, not this gate directly.

## 2. Law of Non-Contradiction (LNC) — `¬(P ∧ ¬P)`  ← HARD GATE
- **Source:** Aristotle, *Metaphysics* Γ.3–6 (1005b–1011b): "the same attribute
  cannot at the same time belong and not belong to the same subject in the same
  respect."
- **Formal:** `¬(P ∧ ¬P)`.
- **Why hard:** even intuitionistic logic accepts LNC. A doc that asserts `P` and
  `¬P` about the same subject (e.g. "OpenSSL eliminated" vs "uses native-tls") is
  a direct logical contradiction → **exit 1, commit refused**.

## 3. Law of Excluded Middle (LEM) — `P ∨ ¬P`  ← CAVEATED
- **Source:** Aristotle, *Metaphysics* Γ; *Posterior Analytics*.
- **Formal:** `P ∨ ¬P`.
- **Honest caveat (verified source):** LEM is **rejected by intuitionistic /
  constructive logic**. Therefore this repo does **not** enforce LEM as a universal
  gate. If a claim *silently assumes* LEM in a non-classical subsystem, the gate
  **escalates to human** (exit 2) rather than asserting it. Classical subsystems
  may use LEM explicitly and are grounded by their proofs.

## 4. Principle of Sufficient Reason (PSR) — governance principle, NOT a law of logic
- **Source:** Leibniz (*Monadology* / *Principia Philosophiae*): "nothing is so
  without a reason why it is so." See Stanford Encyclopedia of Philosophy,
  *Principle of Sufficient Reason* (Melamed) — explicitly noted as **powerful and
  controversial**, not a theorem of classical logic.
- **Role here:** every non-trivial claim must have a **ground** (proof / test /
  citation). Absent a ground → escalate (exit 2). We record PSR as a *process
  rule*, not as an axiomatic truth, precisely because its logical status is
  disputed.

## 5. Bivalence (distinct from LEM)
- Every proposition is either true or false. Noted for clarity; in this repo a
  claim's truth value is decided by grounding (§0), not by declaration.

## 6. Repository constitution (explicit operator rule)
- **Both** the bebop **protocol** (`bebop2/*`) and the bebop **agent**
  (`crates/bebop`) are canonical and MUST remain in the repository. Deleting
  either is a hard violation (exit 1). `logic-gate.mjs` asserts both directories
  exist on every run.

## 7. Paradox / unprovable → human arbiter (escalation protocol)
When `logic-gate.mjs` cannot establish truth (unbacked claim, self-referential
truth claim, or a genuine logical paradox), it MUST NOT auto-resolve. It writes:
```
## ESC-<id> — <date>
- Claim: "<verbatim claim text>"  (file:line)
- Kind: unbacked | paradox | lem-assumed
- Status: OPEN
- Arbiter: <operator or @user>
- Resolution: <filled by human; e.g. "TRUE — proven by <ref>", "FALSE", "DEFER">
```
The operator (or a designated user) records the verdict. An `OPEN` escalation is
allowed to ship (so work is not blocked) but is tracked until resolved. This is
the "call the human as arbitrator" rule — paradoxes are decided by people, not
by the gate.

## 8. Honesty clauses (self-applied)
- These laws are **theorems/tautologies of classical logic, not axioms** (Wikipedia,
  *Law of thought*). We enforce them as *cited conventions with a grounding
  requirement*, never as self-justifying truths.
- If the gate itself contradicts a claim it cannot prove, that is an `ESC-` entry,
  not a silent pass.

## 9. Agent-code laws — the programmatic basis for ALL agents/subagents
Every agent or subagent writing/modifying code in this repo MUST follow these
**universally accepted, internationally recognized** software-engineering
principles. Sources are cited; they are *professional standards*, not opinions.

**A. Quality characteristics (ISO/IEC 25010)** — code must aim for:
functional suitability, reliability, security, maintainability, and readability.
Readability/maintainability are first-class, not optional. (ISO/IEC 25010;
Sonar "code quality = readable + maintainable + secure + reliable".)

**B. Secure coding (CERT / CWE Top 25 — SEI/CMU, MITRE)**
- No injection (CWE-79 XSS, CWE-89 SQLi, CWE-78 OS cmd) — use typed/parameterized APIs.
- No buffer/int overflow (CWE-120/787), no null-deref (CWE-476), no use-after-free (CWE-416).
- No secrets in source (CWE-798/259) — `zeroize` on drop; config via env/secret store.
- Validate trust boundaries (CWE-20 input validation); fail closed, never silent.
- Prefer memory-safe patterns; avoid unsafe unless justified + reviewed.
Reference: `cwe.mitre.org/top25/`, SEI CERT Secure Coding Standards.

**C. Reliability & correctness**
- **Falsifiable tests** (RED+GREEN): every non-trivial function has a test that
  CAN fail (Verified-by-Math principle 3; this repo's `guardrail-falsifiable-proof.mjs`).
- **Determinism at trust boundaries**: time/RNG/socket not reachable in the air-gapped
  kernel (`rust-core/` empty-import gate). Reproducible builds.
- **Fail-closed**: on error, deny; never approve/partial-apply silently (red-line rule).

**D. Readability & simplicity (clean code, SEI/Google practices)**
- YAGNI / smallest abstraction that works. No premature abstraction, no dead code,
  no boilerplate nobody asked for.
- Self-documenting names; comments explain *why*, not *what*. Delete > add.
- One responsibility per module/function (single-responsibility).

**E. Professional ethics (IEEE/ACM Software Engineering Code of Ethics)**
- Hold paramount the **safety, health, and welfare of the public**; protect privacy.
- Be honest about capability and limitation (no false-green, no over-claim).
- Accept and give peer review; the **3-model review** (builder ≠ reviewer ≠ overlap)
  here operationalizes "peer review" for machine agents.

**F. Agent/subagent enforcement**
- `logic-gate.mjs` does NOT attempt to prove code quality (undecidable). It
  ESCALATES (exit 2) when code/doc text asserts quality/security/safety claims
  that lack a ground (test/proof/citation) — same PSR process as §4.
- The **CI/build** layer is the real enforcer: `cargo clippy`, `cargo test`,
  `cargo-deny`, `cargo-audit` (WS-4 gate). Code that does not compile/test/
  audit-clean is refused by CI, independently of this doc gate.
- Any agent that silently drops an `OPEN` escalation, or edits tests to make them
  pass without fixing the code, violates §E (honesty) — a hard ethics breach,
  logged and human-arbitrated.

> Honest limit: "good code" is judged by human reviewers + CI, not by this gate.
> The gate only guarantees the *claim* "this code is secure/correct" is grounded,
> and that the *process* (tests, review, fail-closed) is followed.
