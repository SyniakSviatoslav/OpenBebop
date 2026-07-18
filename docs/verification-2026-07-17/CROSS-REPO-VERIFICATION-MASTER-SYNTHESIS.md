# CROSS-REPO VERIFICATION MASTER SYNTHESIS (five repos, 2026-07-17)

> **This is the file that was permanently lost** (the prior worktree was never pushed). It is
> regenerated fresh here and pushed to `openbebop`. It aggregates one fresh audit (bebop2, done
> in this worktree) with four sibling per-repo audits that survived, and synthesizes the patterns
> that recur **across** repos — the view no single per-repo doc has.

## Sources aggregated

| Repo | Worktree / branch | Code state | Audit docs (read as-is; the four siblings are NOT re-derived here) |
|------|-------------------|-----------|--------------------------------------------------------------------|
| **bebop2 / openbebop** | `/root/bebop2-verify-redteam` · `research/bebop2-verify-redteam-2026-07-17` | `b87b7e2` | **this corpus** V1–V9 + `VERIFICATION-MASTER-SYNTHESIS.md` (fresh, 3-model-reviewed) |
| **dowiz / DeliveryOS** | `/root/dowiz-verify-redteam` | `main` @ `4956faca3` | `V1-adversarial-claim-verification.md`, `V3-red-team-attack-catalog.md` |
| **agentic-mesh** | `/root/agentic-mesh-verify-redteam` | `feat/agentic-mesh-protocol` @ `84a1e272d` | `V1-V3-adversarial-and-redteam.md` |
| **spectral-evolution** | `/root/spectral-evolution-verify-redteam` | `feat/spectral-energy-flow-evolution` @ `6bd181a02` | `V1-V3-adversarial-and-redteam.md` |
| **hermes-agent-kernel** | `/root/hermes-verify-redteam` | `feat/kernel-rust-rewrite` @ `45520a7` | `V1-V3-adversarial-and-redteam.md` |

**Numbering note (important).** The four sibling docs cite bebop2 findings by the **prior (lost)**
bebop2 numbering. This regenerated corpus renumbers. Reconciliation so the cross-refs resolve:

| Sibling docs say (old) | = this corpus (new) | Finding |
|---|---|---|
| bebop2 "V1 #4" / "V3 4.1" | **V4** | unbounded per-anchor issuance / no chain-depth cap |
| bebop2 "V3 2.2" | **V8.1** | nonce eviction half-drop → replay |
| bebop2 "V3 4.2/4.3" | **V1** | unauthenticated / forged revocation |
| bebop2 "V3 6.1" | **Pattern 7 (not a numbered finding)** | caller-supplied `now` expiry bypass — acknowledged in Pattern 7 below as under-weighted in this pass; **not** formalized as a numbered bebop2 finding (my V8.2 is the plans-vs-impl ledger, not this) |
| bebop2 "V3 6.4" | **V2** | dormant red-line gate |
| bebop2 "V7" (referenced as a poison finding) | dowiz V1 #6 / V3 | `.lock().unwrap()` cascade (bebop2's own gate uses the *good* pattern) |

---

## The cross-repo patterns (the actual synthesis)

### Pattern 1 — Non-finite (NaN/inf) fails **OPEN**: the "NaN-masking" family — *shared kernel crate*

The single most-recurring correctness class, and it is **literally the same code** in two repos:
**dowiz and spectral-evolution share the `kernel/` crate** (spectral E1/E2 added `incidence.rs` /
`stats.rs` to dowiz's kernel). So the NaN-masking primitives are one implementation audited twice:

- `spectral_radius(...).fold(0.0, f64::max)` **silently drops NaN eigenvalues** → a poisoned
  operator reads as ρ=0 = *stable*. Found independently as **dowiz V3 §4.1** (`spectral_nan_mask_fails_open`, HIGH) and **spectral-evolution V1 #4** (`spectral.rs:217-219`). Same fold, same repo-shared file.
- `x > tol` accept-by-default with **no `is_finite` guard**: `NaN > tol == false` ⇒ a diverged
  step is certified safe. **spectral-evolution V1 #2a** (`lyapunov_nonincreasing`, the authors even
  work around it in the *test* not the *gate*) and the shared self-mod guard (`evals.rs`).
- `f64` budget ceiling defeated by a `NaN`/negative estimate ⇒ **degrade-closed flips to
  degrade-open, permanently**: **dowiz V1 #5 / V3 §3.5** (`budget.rs`, "the most concerning finding").

**bebop2 is largely immune on its authorization path** — the auth gate compares **integer**
nonces/expiry, not `f64` spectra, so this pass surfaced no NaN-mask finding there. (bebop2 *does*
have an integer money law in `delivery-domain/src/lib.rs`, but this pass audited the auth/crypto
path, not that module, so I don't cite it as evidence.) The sharper lesson is **intra-dowiz**: the
same repo applies rigorous integer discipline to its *money* law (`checked_mul`/`checked_add`,
`i128` range-checks — dowiz V1 #5) yet leaves `budget.rs` and the spectral gates on **unguarded
`f64`**. Finite/integer discipline is the fix the shared kernel's `f64` gates need. *(Attribution
corrected in review: the `checked_*`/`i128`-money evidence is dowiz's, not bebop2's.)*

**One-line fix class, everywhere:** reject non-finite in every fold/compare that feeds a
stability/budget/integrity verdict (`if !v.is_finite() { return <fail-closed> }`;
`fold(f64::NEG_INFINITY, ...)` with a NaN guard, not `fold(0.0, f64::max)`).

### Pattern 2 — Capability/Sybil admission: **derived from bebop2, inherited unmitigated**

agentic-mesh's admission logic is a **direct derivative of bebop2's `roster.rs`/`verify_chain`**,
so bebop2's V4 rides along:

- **bebop2 V4**: no per-anchor issuance bound + no `MAX_CHAIN_DEPTH`.
- **agentic-mesh A5**: *identical* unbounded per-anchor issuance (`cap.rs:370-402`) — "N anchored
  Sybils from one anchor", no `IssuanceBudget`.
- **The one place the fork improved on the origin:** agentic-mesh added
  `MAX_VERIFY_CHAIN_LINKS = 16` at admission (`admission.rs:48`), **closing the depth-DoS half**
  of bebop2 V4 pre-crypto. So the port hardened the DoS but left the issuance-count Sybil gap.
- **Cross-repo owed fix:** an `IssuanceBudget` / real `RootDelegationPolicy` dispatch (which also
  ties to **bebop2 V5** — the policy enum that is a pure marker) is owed in *both* repos; and
  bebop2 should backport agentic-mesh's `MAX_CHAIN_DEPTH`.

### Pattern 3 — Unauthenticated, monotonic revocation: **same design, copied**

- **bebop2 V1**: `RevocationSet::{revoke_key,merge,gossip_payload}` unsigned, monotonic, no unrevoke.
- **agentic-mesh B-5**: the same design (the sibling doc's own word; `cap.rs:417-437`), rated MEDIUM there only
  because the Poly-Network guard keeps frames off the `&mut` mutators and no gossip consumer is
  wired *yet* — i.e. **latent in both**, and both bite the moment a sync/gossip consumer folds a
  peer's set. Same fix (anchor-signed revocation entries; `merge` verifies before union).

### Pattern 4 — Safety brakes exist but the **production path doesn't arm/reach them**

The most pervasive *architectural* pattern: a correct, unit-tested safe primitive that the prod
seam bypasses.

- **bebop2 V2**: `HybridGate::new_redlined` exists + tested, but `KernelFacade` (the only prod
  seam) hard-codes the unarmed `HybridGate::new` and offers no way to arm it.
- **agentic-mesh A7**: same footgun (`Admitter` doesn't force `new_redlined`) **plus** the gate
  inspects the *wrong scope field* — it checks the forced `(AgentBridge,AdmitAgent)` admission
  scope, never the manifest's `action_scopes`, so money/auth/secret operating scopes pass admission
  unchecked.
- **spectral-evolution V1 #5**: the E3 "Phase-B blocked on P06 `key_V`" invariant is enforced by
  **nothing structural** — the one built self-mod path auto-applies to a real kernel knob with no
  `key_V` precondition; the boundary is a *comment*, not a gate.
- **Common root:** "the armed constructor / the required precondition exists; the production path
  uses the unarmed one / skips it." Fix class: make the safe posture the **only constructible**
  one (or a structural/type gate), not a caller option.

### Pattern 5 — Replay-ledger eviction reopens replay: **verbatim clone**

- **bebop2 V8.1**: `seen`-set prune drops half the nonces (`hybrid_gate.rs:201-205`), so evicted
  nonces become replayable; the comment "any half is fine" is wrong.
- **agentic-mesh B-1**: the **exact same half-drop**, copied into `admission.rs:243-247`, rated HIGH.
  *Severity-reconciliation note:* the code and preconditions are identical, yet bebop2 rates it
  **MEDIUM (V8.1)** and agentic-mesh **HIGH (B-1)** — the rating differs, not the mechanism; treat
  both as the same MEDIUM-to-HIGH replay-after-eviction bug.
- dowiz has its own distinct replay family (**V3 §2.1** zero-`prev` double-commit; **§2.2** TOCTOU
  double-`decide`). **Fix class:** evict by **expiry/epoch**, never arbitrary half; a consumed
  nonce must never become admissible again.

### Pattern 6 — `.lock().unwrap()` poison-cascade: **risk relocated, and one repo is immune by design**

- **dowiz V1 #6 / V3**: `token_bucket.rs` was hardened (`unwrap_or_else(|e| e.into_inner())`) but
  the **identical raw `.lock().unwrap()` survives in `budget.rs:147,156`** on the spend path — risk
  *relocated, not eliminated*.
- **agentic-mesh B-4**: `TokenBucket.lock().unwrap()` (`token_bucket.rs:68`) underlies the
  `AdmissionLimiter`, **inconsistent** with the gate's own typed `LockPoisoned` handling one layer up.
- **bebop2 is the reference-good pattern:** `HybridGate` uses
  `seen.lock().map_err(|_| CapError::LockPoisoned)` — the typed-error degrade the others should adopt.
- **hermes is structurally immune:** zero `Mutex`/`RwLock`/`.lock()` in the whole kernel (pure
  functions over a single-threaded stdin→stdout subprocess) — and Python has no lock poisoning, so
  the class has no analog there (hermes claim 4). **Lesson:** the cascade is an artifact of shared
  mutable lock state; the single-threaded-pure-subprocess and the typed-error patterns both defeat it.

### Pattern 7 — Caller-supplied trust values crossing a boundary **unvalidated**

- **dowiz V3 §1.2 / §6.3**: `apply_event` trusts client `subtotal`/`total`; tampered JS context
  forges money numbers. **§5.6**: `price_trusted` is *set but never enforced*.
- **agentic-mesh B-2**: caller-controlled `now = 0` ⇒ every capability/link is "fresh" ⇒ total
  expiry bypass.
- **bebop2**: the gate takes `now` from a host `clock` closure with **no floor** (same class,
  under-weighted in my first pass; agentic-mesh's B-2 surfaced it — credited).
- **Fix class:** validate/bound every caller-supplied scalar that gates a security decision
  (finiteness, sign, monotonic floor, host-authoritative clock).

### Pattern 8 — "Built ≠ integrated / plan ≠ implementation" drift (every repo)

The charter's core demand ("correspondence to plans, not just implementation") is where **every**
repo shows daylight:

- **bebop2**: brake unwired (V2); native mobile unwired behind a correct fail-closed (V7);
  keygen-secret path on self-declared Wave-1 (V3).
- **agentic-mesh**: `WorkReceipt`/`Settlement`/`ExposureLedger` (B2/B3) are **blueprint-only** — all
  their claimed properties (counterparty verification, DvP atomicity, exposure caps) are untestable;
  the 0x12 discriminant collision is "found + flagged + deferred (`UNRATIFIED`)", not "fixed".
- **spectral-evolution**: E3 Phase-B unwired; "Lyapunov gate" over-promises ("per-step ≤ tol", not
  globally non-increasing).
- **hermes**: "governance.sh doesn't call it" is stale (the script was *replaced by* the kernel);
  the kernel **is** on the live turn-loop (routing + degrade-closed verification gate), but
  HK-07/08/10/11 (`gov_*`/`report_*`/control) are **built, green, unwired**.
- **Meta-lesson:** planning docs describe integrated systems; code ships a subset with the safe
  primitives present but the wiring lagging. Audits must test the *wired prod path*, not the unit tests.

### Pattern 9 — Prompt-injection: kernels closed-by-construction; the live surface is a **product** one

- **hermes** is the only repo with a **live** product-surface prompt-injection finding: **T1**,
  observed group chatter stored as a `role:"user"` turn with *prompt-only* isolation (`adapter.py:7306-7339`), config-gated MED-HIGH — a non-allowlisted group member can steer the operator's agent.
- Every **kernel** boundary audited (hermes, bebop2, dowiz) is **injection-closed by construction**:
  only scalars/enums/bytes cross the trust boundary, never model-reachable free text. **Lesson:** the
  feature-extraction / typed-boundary is the load-bearing control; keep untrusted text out of the
  decision core, and apply the same untrusted-value escaping to *observed* text that is applied to metadata.

---

## Unique, non-inherited standouts (per repo)

- **agentic-mesh B-3 (HIGH, arc-original):** `RefSigner` is a `pub` (not `#[cfg(test)]`) signer whose
  "signature" is `secret XOR H(msg)` — observing **one** signature recovers the secret; observing one
  anchor-rooted delegation **recovers the anchor's secret key** → remote unlimited anchored Sybils
  (compounds Pattern 2 into a *remote* break). Nothing structurally prevents it being the injected
  verifier in a release build.
- **dowiz V3 §5.2 / §5.6 (HIGH):** `CompensatedRefund` reachable without reversing the ledger;
  `price_trusted` set-but-never-enforced — money-integrity gaps unique to the order machine.
- **bebop2 V3 (HIGH latent):** `keygen_from_entropy` returns the *public* key as the "secret" and
  drops the entropy seed — a keypair generator that can't produce a signable secret.
- **bebop2 V9.1 (positive):** the project's own #1 CRITICAL (ML-DSA sampling A from CBD) is
  **genuinely remediated** with the FIPS uniform sampler + committed NIST ACVP vectors — the one
  clean "claim held" benchmark result across the five repos.

## Meta-finding: decorrelated review catches *shared* blind spots

This synthesis is itself evidence for the practice that produced it. The bebop2 pass's **V7 was
overstated** (a manufactured "mobile reach" plan-claim + mislabeling a prescribed fail-closed as a
defect) — and that overstatement was **shared by the builder and the first reviewer**; only the
**third (overlap) agent** caught it. That is precisely the failure mode the repo's 3-model gate
(builder ≠ reviewer ≠ overlap) and the ODR "decorrelation" principle exist to catch, and it fired
correctly. Across repos, the same lesson recurs (spectral #1 the strongest artifact was the one
with a *red-provable in-place* falsifier; the §A.3.1 Poly1305 shared-blind-spot the gate cites).
**Independent, decorrelated verification is not ceremony — it changed three severities in this pass.**

## Consolidated remediation priorities (cross-repo, by pattern)

1. **Kill NaN-fails-open in the shared `kernel/` crate** (Pattern 1) — one fix serves dowiz **and**
   spectral-evolution: guard `is_finite` in `spectral_radius`/`lyapunov`/budget compares. *Highest
   leverage — one crate, two repos, the most-cited HIGH.*
2. **Make safety brakes the only constructible posture** (Pattern 4) — bebop2 `KernelFacade` arms
   the red-line; agentic-mesh `Admitter` forces `new_redlined` **and** red-line-checks
   `manifest.action_scopes`; spectral E3 gets a structural `key_V` precondition.
3. **Evict replay nonces by expiry, never half-drop** (Pattern 5) — bebop2 + agentic-mesh, same edit.
4. **Authenticate revocation before any gossip consumer lands** (Pattern 3) — bebop2 + agentic-mesh.
5. **Bound issuance + backport `MAX_CHAIN_DEPTH`** (Pattern 2) — bebop2 gets the depth cap
   agentic-mesh already has; both get an issuance budget / real policy dispatch (bebop2 V5).
6. **`#[cfg(test)]`-gate `RefSigner`** (agentic-mesh B-3) — the one arc-original HIGH; unshippable, not doc-discouraged.
7. **Validate caller-supplied scalars** (Pattern 7) — bound `now`; validate totals; enforce `price_trusted`.
8. **Sweep `.lock().unwrap()` to typed-error/into_inner** (Pattern 6) — dowiz `budget.rs`,
   agentic-mesh `token_bucket.rs`; adopt bebop2's `map_err(LockPoisoned)` as the standard.
9. **Escape observed untrusted text like metadata** (hermes T1) — the one live product prompt-injection.

## Integrity statement

The bebop2 findings (V1–V9) are backed by fresh `file:line` reads of `b87b7e2` and were 3-model
peer-reviewed (severities corrected as noted). The four sibling repos' findings are cited **as
written** in their surviving docs (read-only, not re-derived per the charter). No cross-repo
pattern is asserted without a concrete finding in each repo it spans. Where a repo is the
**counter-example** to a pattern (bebop2's integer money law vs Pattern 1; hermes' lockless kernel
vs Pattern 6; every kernel vs Pattern 9), that is stated as the fix the others should adopt.
