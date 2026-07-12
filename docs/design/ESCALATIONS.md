# Escalations — human-arbitrated truth resolutions

When `scripts/logic-gate.mjs` (Enforcement model §0 of `LOGIC-LAWS.md`) cannot
establish that a claim is true — because it is **unbacked**, **self-referential
(paradox)**, or **silently assumes LEM** in a non-classical context — it writes
an entry here and returns exit code `2` (commit allowed, but tracked). A human
arbiter (the operator or a designated user) fills the `Resolution` field.

**Rules**
- `OPEN` escalations may ship, but must be resolved before a release cut.
- Resolution values: `TRUE — <ref>`, `FALSE`, `DEFER — <reason>`.
- Never delete an `OPEN` entry to make the gate green. That itself is a
  non-contradiction violation (hiding a claim) and will be caught.

---

## ESC-mrhmo0gp-1 — 2026-07-12
- Claim: "- **Narration + looks** — `bebop init` picks a voice (bebop / plain / sarcastic / corporate-killer)"  (README.md:38)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gq-2 — 2026-07-12
- Claim: "| **zkVM `decide()` journal** | Every admitted command gets a tamper-evident digest over `(state, commandHash, seq)`. On by default at the kernel gate. Replay-v"  (README.md:123)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gr-3 — 2026-07-12
- Claim: "anomaly signals. Fed quality streams; emits math-proven authority. Applied live to any"  (docs/ARCHITECTURE.md:85)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gr-4 — 2026-07-12
- Claim: "`main` (or a version tag is cut), the docs MUST be brought back in sync and verified with the same"  (docs/RULES.md:58)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gr-5 — 2026-07-12
- Claim: "- **Same manner, every time.** The procedure is codified as the `release-docs` skill"  (docs/RULES.md:68)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gr-6 — 2026-07-12
- Claim: "(`.bebop/skills/release-docs/SKILL.md`) and the `bebop docs` command. Future releases reuse it"  (docs/RULES.md:69)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gr-7 — 2026-07-12
- Claim: "release isn't verified."  (docs/RULES.md:75)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gs-8 — 2026-07-12
- Claim: "| 23 | slash `/status`, `/model`, `/skills` | `bebop` then `/status`, `/model`, `/skills` | each returns correct state | PASS |"  (docs/VERIFICATION-MATRIX.md:42)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gs-9 — 2026-07-12
- Claim: "| 24 | `dispatch` empty task does not silently "approve" a red-line | `bebop dispatch "edit packages/db/migrations/x.sql"` (empty-arg form) | DENIED — guard run"  (docs/VERIFICATION-MATRIX.md:43)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gs-10 — 2026-07-12
- Claim: "| 27 | With key, real OpenRouter call issued | `OPENROUTER_API_KEY=[dummy] bebop dispatch "say hi"` | HTTP request to openrouter.ai (got 401 on dummy key — prov"  (docs/VERIFICATION-MATRIX.md:51)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gs-11 — 2026-07-12
- Claim: "| `/skills` | List loaded skills (`.bebop/skills/*`). |"  (docs/commands.md:35)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gs-12 — 2026-07-12
- Claim: "implementation BEFORE building. Stated = mathematically + code-verified; if unsure,"  (docs/design/BEBOP-CLAIM-AUDIT-2026-07-12.md:5)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gt-13 — 2026-07-12
- Claim: "> Red-line invariant: **NO-COURIER-SCORING** (structural, no rating fields) + **hybrid-only-until-audit** (both sig legs non-`Option`) + **no serde on the signe"  (docs/design/BEBOP2-REMEDIATION-PARALLEL-PLAN.md:6)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gt-14 — 2026-07-12
- Claim: "## 6. INTEGRATION COMPLETE — 2026-07-12 (operator waived sign-off; commit verified+tested)"  (docs/design/BEBOP2-REMEDIATION-PARALLEL-PLAN.md:85)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gt-15 — 2026-07-12
- Claim: "- Then Phase 1–5 per blueprint. Red-team 3A/4A addressed; §2 (OpenSSL) KILLED."  (docs/design/BEBOP2-REMEDIATION-PARALLEL-PLAN.md:116)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gt-16 — 2026-07-12
- Claim: "> (main bebop2-core = 94/0 verified this session). Sources cited file:line."  (docs/design/CONSOLIDATED-AUDIT-EXTRACT-2026-07-11.md:6)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gt-17 — 2026-07-12
- Claim: "### D3. Privacy substrate (verified crypto)"  (docs/design/CONSOLIDATED-AUDIT-EXTRACT-2026-07-11.md:54)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gu-18 — 2026-07-12
- Claim: "## CATEGORY 5 — CRITIQUE OF APPROACHES (over-claims to kill)"  (docs/design/CONSOLIDATED-AUDIT-EXTRACT-2026-07-11.md:111)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gu-19 — 2026-07-12
- Claim: "- 3+ rounds of parallel subagents returned FALSE-GREEN (claimed tests green while failing;"  (docs/design/CONSOLIDATED-AUDIT-EXTRACT-2026-07-11.md:138)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gu-20 — 2026-07-12
- Claim: "claimed FIPS bit-exact while pinning own bytes). Trust literal `cargo test`, not agent summaries."  (docs/design/CONSOLIDATED-AUDIT-EXTRACT-2026-07-11.md:139)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gu-21 — 2026-07-12
- Claim: "reconcile deep-research (stale) vs blueprint (current) so no doc claims un-cargo-verified state."  (docs/design/CONSOLIDATED-AUDIT-EXTRACT-2026-07-11.md:164)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gu-22 — 2026-07-12
- Claim: "> VERDICTs from these were CLAIMED by research agents; file:line evidence index exists in"  (docs/design/CONSOLIDATED-AUDIT-EXTRACT-2026-07-11.md:176)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gu-23 — 2026-07-12
- Claim: "> hub-review. NOT independently cargo-verified by me (dowiz is Node/Astro, out of this Rust session)."  (docs/design/CONSOLIDATED-AUDIT-EXTRACT-2026-07-11.md:177)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gu-24 — 2026-07-12
- Claim: "hand one claimed venue the QR kit + "orders by channel" card → out-of-app courier beep (only net-new"  (docs/design/CONSOLIDATED-AUDIT-EXTRACT-2026-07-11.md:225)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gu-25 — 2026-07-12
- Claim: "- VERIFIED: Law 87/2019 records only venue sale, NO buyer fields → client anonymous by default."  (docs/design/CONSOLIDATED-AUDIT-EXTRACT-2026-07-11.md:263)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gu-26 — 2026-07-12
- Claim: "- Verified 2026: SimpleX (no identifiers, self-host SMP relays, not pure P2P); Session (no phone, but"  (docs/design/CONSOLIDATED-AUDIT-EXTRACT-2026-07-11.md:275)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gu-27 — 2026-07-12
- Claim: "- "No courier scoring" decision is legally sound (couriers = venue staff → dowiz avoids platform-law)."  (docs/design/CONSOLIDATED-AUDIT-EXTRACT-2026-07-11.md:284)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gu-28 — 2026-07-12
- Claim: "- WEAKENED ADR-0015 framing: PROPOSED not council-approved; admin/courier voice REMOVED from active"  (docs/design/CONSOLIDATED-AUDIT-EXTRACT-2026-07-11.md:319)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gu-29 — 2026-07-12
- Claim: "- PLAN: 10–13 sessions (vs claimed 7–9); never pre-empts Wave 0/1; earliest Wave-4-parallel; D-PC1 funding gate."  (docs/design/CONSOLIDATED-AUDIT-EXTRACT-2026-07-11.md:324)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gu-30 — 2026-07-12
- Claim: "- REAL (verified/implemented): bebop math core (spectral/Kalman/Lyapunov/FFT); bebop2 crypto 94/0;"  (docs/design/CONSOLIDATED-AUDIT-EXTRACT-2026-07-11.md:345)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gu-31 — 2026-07-12
- Claim: "- POETRY/MISLABEL (kill in docs): "0% fee = moat"; "field-wave replaces binary search"; Emden "demand"  (docs/design/CONSOLIDATED-AUDIT-EXTRACT-2026-07-11.md:348)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gu-32 — 2026-07-12
- Claim: "documentation statement, roadmap claim, and code-level "verified" assertion in"  (docs/design/LOGIC-LAWS.md:7)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gu-33 — 2026-07-12
- Claim: "- Resolution: <filled by human; e.g. "TRUE — proven by <ref>", "FALSE", "DEFER">"  (docs/design/LOGIC-LAWS.md:84)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gu-34 — 2026-07-12
- Claim: "+ dowiz Sovereign-Core doctrine. Crypto layer status verified green same session"  (docs/design/UNIFIED-DELIVERY-PROTOCOL-BLUEPRINT-v3-2026-07-11.md:204)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gu-35 — 2026-07-12
- Claim: "0. Constant Doubt (unverified=false; enforced by verify-doc-claims + guardrail scripts)"  (docs/design/_fable-brief-2026-07-10.md:43)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gv-36 — 2026-07-12
- Claim: "P2  bit-level math axioms — sinc(0)=1, cosine(v,v)=1, cross(a,a)=0, dot(ortho)=0, proven at bit level"  (docs/design/_fable-brief-core-re-loop-2026-07-10.md:31)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gv-37 — 2026-07-12
- Claim: "- LABEL-WITHOUT-PROPERTY: P1 "symbol present in machine code" — does `grep` on `objdump` output prove"  (docs/design/_fable-brief-core-re-loop-2026-07-10.md:40)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gv-38 — 2026-07-12
- Claim: "the primitive is CORRECT, or merely that the string exists? Could a no-op/garbage function pass?"  (docs/design/_fable-brief-core-re-loop-2026-07-10.md:41)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gv-39 — 2026-07-12
- Claim: "- CIRCULAR / SELF-SEALING: does P2's `cargo test` call "prove the axioms" or just re-run tests the"  (docs/design/_fable-brief-core-re-loop-2026-07-10.md:42)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gv-40 — 2026-07-12
- Claim: "### A. What the loop ACTUALLY proves (file:line) vs what it CLAIMS to prove (its own doc/stdout)."  (docs/design/_fable-brief-core-re-loop-2026-07-10.md:58)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gv-41 — 2026-07-12
- Claim: "| Check | Claims (doc/stdout) | Actually proves |"  (docs/design/_fable-review-core-re-loop-2026-07-10.md:11)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gv-42 — 2026-07-12
- Claim: "## 3. Invariants the gate guarantees (test-backed)"  (docs/design/adr-003-neuro-symbolic-gate-2026-07-09.md:41)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gv-43 — 2026-07-12
- Claim: "2. **A dead factor (ICIR < kill) floors authority to uMin.** The advisor is overruled, not trusted. (N7 RED test.)"  (docs/design/adr-003-neuro-symbolic-gate-2026-07-09.md:44)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gv-44 — 2026-07-12
- Claim: "RED invariant violation, RED effect-noop each proven to be caught with a precise message."  (docs/design/adr-004-logical-cot-pddl-instruct-2026-07-09.md:53)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gv-45 — 2026-07-12
- Claim: "- Verified by the doc-claim gate (check Z) so the claim cannot rot."  (docs/design/adr-004-logical-cot-pddl-instruct-2026-07-09.md:54)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gv-46 — 2026-07-12
- Claim: "behind `GnnAdvisor` with **zero change to the kernel** (this satisfies ADR-003 §4 decoupling)."  (docs/design/bebop-L5-dual-track-gnn-2026-07-09.md:19)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gv-47 — 2026-07-12
- Claim: "it to `dualTrackGate`. No kernel change. The gate's RED cases already prove it cannot be fooled by"  (docs/design/bebop-L5-dual-track-gnn-2026-07-09.md:59)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gv-48 — 2026-07-12
- Claim: "_Date: 2026-07-09 · Author: Hermes agent · Status: RESEARCH + PLAN (no new code in this doc; prior wave D0–D6 already landed & verified)_"  (docs/design/bebop-L5-research-roadmap-2026-07-09.md:3)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gv-49 — 2026-07-12
- Claim: "The 2026-07-09 D0–D6 wave landed all of this in `bebop` (flag-OFF, RED+GREEN proven; `npm run verify` GREEN at 434 pass / 0 fail):"  (docs/design/bebop-L5-research-roadmap-2026-07-09.md:25)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gv-50 — 2026-07-12
- Claim: "normal telemetry, then flip `cfg.beta>0`; RED+GREEN proves on/off behavior."  (docs/design/bebop-L5-research-roadmap-2026-07-09.md:63)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gv-51 — 2026-07-12
- Claim: "and asserts latent mean≈0/var≈1 on normal data; flip `beta>0` only after calibration."  (docs/design/bebop-L5-research-roadmap-2026-07-09.md:131)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gv-52 — 2026-07-12
- Claim: "- **RED+GREEN:** GREEN — calibrated β improves separation on sharp excursion; RED — uncalibrated"  (docs/design/bebop-L5-research-roadmap-2026-07-09.md:132)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gv-53 — 2026-07-12
- Claim: "both close real gaps the dump identified, both falsifiable. Implement, prove, wire."  (docs/design/bebop-L5-research-roadmap-2026-07-09.md:168)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gv-54 — 2026-07-12
- Claim: "floor / dead-factor kill / resonance cap / any clamp). Surfaced on `GovernorState` too."  (docs/design/bebop-L5-research-roadmap-2026-07-09.md:248)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gv-55 — 2026-07-12
- Claim: "## 7b. Phase-3 extensions — IMPLEMENTED & VERIFIED (2026-07-09)"  (docs/design/bebop-L5-research-roadmap-2026-07-09.md:259)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gv-56 — 2026-07-12
- Claim: "Following the operator's "implement A+B+C + research next tools" directive, the three approved"  (docs/design/bebop-L5-research-roadmap-2026-07-09.md:261)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gv-57 — 2026-07-12
- Claim: "Deferred as a reference for a future bebop interactive TUI (the launch animation already proves TTY"  (docs/design/bebop-L5-research-roadmap-2026-07-09.md:345)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gw-58 — 2026-07-12
- Claim: "_Date: 2026-07-09 · Author: Hermes agent · Status: landed, flag-OFF, RED+GREEN proven_"  (docs/design/bebop-analytics-loss-anomaly-2026-07-09.md:3)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gw-59 — 2026-07-12
- Claim: "(score must exceed the floor by 10% to declare an anomaly, killing numerical-noise trips)."  (docs/design/bebop-analytics-loss-anomaly-2026-07-09.md:43)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gw-60 — 2026-07-12
- Claim: "1. **Wire `selectZenoh` + `prove` into kernel dispatch** (flag-OFF, RED+GREEN) — the standing"  (docs/design/bebop-analytics-loss-anomaly-2026-07-09.md:103)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gw-61 — 2026-07-12
- Claim: "| Skills / procedural memory | skills | — | **self-creating skills + auto-improve** | skills (port) + self-maintain |"  (docs/design/bebop-cli-2026-07-09.md:39)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gw-62 — 2026-07-12
- Claim: "| Guard / determinism | permission modes | — | approvals | **Guard OS: deny-on-red, Verified-by-Math** |"  (docs/design/bebop-cli-2026-07-09.md:46)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gw-63 — 2026-07-12
- Claim: "| Math proven core | — | — | — | **graph-PDE field + Kalman + GOAP (Rust)** |"  (docs/design/bebop-cli-2026-07-09.md:47)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gw-64 — 2026-07-12
- Claim: "2. **narration** — `bebop | plain | sarcastic | corporate-killer` (the dry"  (docs/design/bebop-cli-2026-07-09.md:114)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gw-65 — 2026-07-12
- Claim: "path-set through BOTH `KERNEL.decide` and the TS `toRegExp` port and asserts identical `ok`/`kind`."  (docs/design/bebop-determinism-hardening-HANDOFF-2026-07-08.md:42)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gw-66 — 2026-07-12
- Claim: "RED test: `createIdentity()` (random) gives two different `id`s across calls (proves randomness path"  (docs/design/bebop-determinism-hardening-HANDOFF-2026-07-08.md:137)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gw-67 — 2026-07-12
- Claim: "use floor 0.6 → noise passes (proves floor bites). Keep the conservative value for "never present noise"  (docs/design/bebop-determinism-hardening-HANDOFF-2026-07-08.md:150)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gx-68 — 2026-07-12
- Claim: "honest-boundary discipline enforced (WormGPT refused; git-spoofing called provenance red-line)."  (docs/design/bebop-fable-research-2026-07-11.md:5)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gx-69 — 2026-07-12
- Claim: "VERIFIED AGAINST PRIMARY SOURCES (official repos, arXiv 2310.10688, RFCs, docs):"  (docs/design/bebop-fable-research-2026-07-11.md:11)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gx-70 — 2026-07-12
- Claim: "7. claude-video — UNVERIFIED/ambiguous. No canonical Anthropic product; likely a third-party"  (docs/design/bebop-fable-research-2026-07-11.md:32)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gx-71 — 2026-07-12
- Claim: "Author-spoofing is native `git commit --author`; the ACT is the provenance red-line, not this tool."  (docs/design/bebop-fable-research-2026-07-11.md:41)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gx-72 — 2026-07-12
- Claim: "25. sentinel octopus — UNVERIFIED canonical tool; appears a YouTube creator project (SentinelProxy/"  (docs/design/bebop-fable-research-2026-07-11.md:62)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gx-73 — 2026-07-12
- Claim: "git author-spoofing = provenance red-line (act, not gitghost tool)."  (docs/design/bebop-fable-research-2026-07-11.md:85)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gx-74 — 2026-07-12
- Claim: "(proves integrator choice is load-bearing, mirrors propagator_red_breaks_on_coeff_change :276)."  (docs/design/bebop-fable-research-2026-07-11.md:115)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gx-75 — 2026-07-12
- Claim: "# Bebop — Fundamental Working & Proven Principles (cross-layer analysis)"  (docs/design/bebop-fundamental-principles-2026-07-09.md:1)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gx-76 — 2026-07-12
- Claim: "> the fundamental working & proven principles from each layer, integrated tool, system, and rule —"  (docs/design/bebop-fundamental-principles-2026-07-09.md:4)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gx-77 — 2026-07-12
- Claim: "import section (core-RE-loop proves no clock/RNG/socket reachable). Any new primitive must earn its"  (docs/design/bebop-fundamental-principles-2026-07-09.md:32)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gx-78 — 2026-07-12
- Claim: "verified against primary sources (datasheet / NASA NTRS / archival spec), not repeated from secondary"  (docs/design/bebop-fundamental-principles-2026-07-09.md:37)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gx-79 — 2026-07-12
- Claim: "### Cross-pattern E — "Proven blind spots, stated not hidden""  (docs/design/bebop-fundamental-principles-2026-07-09.md:149)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gx-80 — 2026-07-12
- Claim: "- cycle-consistency proves `gap=0` ≠ correct (self-inverse bijection) and keeps explicit contract tests."  (docs/design/bebop-fundamental-principles-2026-07-09.md:151)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gx-81 — 2026-07-12
- Claim: "- RULES.md: "better less than sorry" — cut the sentence rather than state an unproven claim."  (docs/design/bebop-fundamental-principles-2026-07-09.md:155)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gx-82 — 2026-07-12
- Claim: "2. Verified-by-Math (falsifiable RED+GREEN, always)."  (docs/design/bebop-fundamental-principles-2026-07-09.md:186)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gx-83 — 2026-07-12
- Claim: "| `candidateSkills(concepts)` | whole corpus | 12 skill candidates (all spread ≥3) |"  (docs/design/bebop-living-memory-harvest-PATTERNS-2026-07-08.md:16)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gx-84 — 2026-07-12
- Claim: "The agent expanded itself: 3 new `SKILL.md` files were written from the findings"  (docs/design/bebop-living-memory-harvest-PATTERNS-2026-07-08.md:21)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gx-85 — 2026-07-12
- Claim: "(`field`, `harvest`, `boundary`) and are now listed under `existingSkills` — the harvest loop is"  (docs/design/bebop-living-memory-harvest-PATTERNS-2026-07-08.md:22)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gx-86 — 2026-07-12
- Claim: "self-closing (mine → write skill → re-harvest confirms it exists)."  (docs/design/bebop-living-memory-harvest-PATTERNS-2026-07-08.md:23)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gx-87 — 2026-07-12
- Claim: "skills    divergence-hot"  (docs/design/bebop-living-memory-harvest-PATTERNS-2026-07-08.md:37)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gx-88 — 2026-07-12
- Claim: "field. Interpretation: the agent's centre of gravity is focusing, not exploring. That is correct for a"  (docs/design/bebop-living-memory-harvest-PATTERNS-2026-07-08.md:46)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gx-89 — 2026-07-12
- Claim: "**Pattern P2 — explorer concepts:** `validate`, `speculate`, `memory`, `skills`, `vsm`, `react` read as"  (docs/design/bebop-living-memory-harvest-PATTERNS-2026-07-08.md:49)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gx-90 — 2026-07-12
- Claim: "memory/skills into neighbours). This is the "generate" directive class."  (docs/design/bebop-living-memory-harvest-PATTERNS-2026-07-08.md:51)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gx-91 — 2026-07-12
- Claim: "- **DSpark (explore/branch)** ↔ `divergence-hot` (validate, speculate, memory, skills, vsm, react)"  (docs/design/bebop-living-memory-harvest-PATTERNS-2026-07-08.md:104)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gx-92 — 2026-07-12
- Claim: "1. `guard`/`recall` are `both-hot` and under-served by dedicated skills → a `recall`/`verify` skill is"  (docs/design/bebop-living-memory-harvest-PATTERNS-2026-07-08.md:118)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gx-93 — 2026-07-12
- Claim: "the highest-value new skill (it is the topological bridge, C2)."  (docs/design/bebop-living-memory-harvest-PATTERNS-2026-07-08.md:119)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gy-94 — 2026-07-12
- Claim: "Arfken&Weber §12). Rodrigues' Pₙ=1/(2ⁿn!)dⁿ/dxⁿ(x²−1)ⁿ is CORRECT."  (docs/design/bebop-math-physics-fable-research-2026-07-11.md:16)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gy-95 — 2026-07-12
- Claim: "PER-CONCEPT VERDICT (a=genuine math, b=applicable to protocol, c=over-claimed analogy):"  (docs/design/bebop-math-physics-fable-research-2026-07-11.md:30)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gy-96 — 2026-07-12
- Claim: "contour-integral "network stability" poetry; over-claimed Emden/redshift/vorticity/noether/fock/catalan"  (docs/design/bebop-math-physics-fable-research-2026-07-11.md:117)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gy-97 — 2026-07-12
- Claim: "| 5 | Compression lossless (VSA frame 34.3%, reversible) | ✅ SAFE | decode(encode(x)) == x byte-exact |"  (docs/design/bebop-memory-optimisation-fable-research-2026-07-11.md:22)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gy-98 — 2026-07-12
- Claim: "ATTIC/INBOX tiering is a SOUND non-destructive design — already discipline-correct:"  (docs/design/bebop-memory-optimisation-fable-research-2026-07-11.md:31)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gy-99 — 2026-07-12
- Claim: "## STATUS (verified on remote, not just local)"  (docs/design/bebop-release-gate-postmortem-2026-07-09.md:7)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gy-100 — 2026-07-12
- Claim: "## 0. Operator directive — the field law (∇·F / ∇×F) as a fundamental improvement"  (docs/design/bebop-research-synthesis-FIELD-DSPARK-PYDANTIC-2026-07-08.md:14)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gy-101 — 2026-07-12
- Claim: "> used for any model reasoning and vectorized search as a fundamental physical improvement, since it"  (docs/design/bebop-research-synthesis-FIELD-DSPARK-PYDANTIC-2026-07-08.md:17)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gy-102 — 2026-07-12
- Claim: "> is the basic law. This is an excellent robust way to improve the preciseness of reasoning, signaling,"  (docs/design/bebop-research-synthesis-FIELD-DSPARK-PYDANTIC-2026-07-08.md:18)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gy-103 — 2026-07-12
- Claim: "`rotate` (proves the two operators are genuinely independent, not a single scalar mislabeled)."  (docs/design/bebop-research-synthesis-FIELD-DSPARK-PYDANTIC-2026-07-08.md:51)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gy-104 — 2026-07-12
- Claim: "- The number of tokens **verified in one shot** is scheduled **per request by a confidence model** —"  (docs/design/bebop-research-synthesis-FIELD-DSPARK-PYDANTIC-2026-07-08.md:65)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gy-105 — 2026-07-12
- Claim: "erased"). Proven by 3 tests (low-cost/high-volume ⇒ hit; generous-cost/tiny-volume ⇒ not hit; the old"  (docs/design/bebop-research-synthesis-FIELD-DSPARK-PYDANTIC-2026-07-08.md:165)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gy-106 — 2026-07-12
- Claim: "(a unit disruption ripples to total mass 1 — proven GREEN)."  (docs/design/bebop-rust-field-core-2026-07-09.md:86)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gy-107 — 2026-07-12
- Claim: "the physics → physics wins). Proven RED→GREEN: tiny `pddlCost` forces OVERRIDE; large `pddlCost`"  (docs/design/bebop-rust-field-core-2026-07-09.md:93)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gz-108 — 2026-07-12
- Claim: "Already present and proven:"  (docs/design/bebop-tensor-field-theory-2026-07-09.md:26)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gz-109 — 2026-07-12
- Claim: "Naïve explicit Euler on `∂²u/∂t² = −c²L u` **injects energy every step** (I proved this in the test:"  (docs/design/bebop-tensor-field-theory-2026-07-09.md:44)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gz-110 — 2026-07-12
- Claim: "- `diffuse` (heat): contractive, energy decays (memory fade) — proven: energy 1 → 0.21 over 20 steps."  (docs/design/bebop-tensor-field-theory-2026-07-09.md:70)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gz-111 — 2026-07-12
- Claim: "- `wave` (velocity-Verlet): energy-conserving (Hamiltonian ½vᵀv + ½c²uᵀLu) — proven: 0.9998 over 50 steps."  (docs/design/bebop-tensor-field-theory-2026-07-09.md:71)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gz-112 — 2026-07-12
- Claim: "- **Verified-by-Math**: any Rust twin must pass the SAME RED+GREEN tests (ported) before a TS→WASM switch,"  (docs/design/bebop-tensor-field-theory-2026-07-09.md:116)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gz-113 — 2026-07-12
- Claim: "so the replacement is proven, not assumed."  (docs/design/bebop-tensor-field-theory-2026-07-09.md:117)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gz-114 — 2026-07-12
- Claim: "just ungated. `cargo build --target wasm32-unknown-unknown --no-default-features` now BUILDS CLEAN with **0 imports** (verified 2026-07-12 via `scripts/verify-e"  (docs/design/bebop2-deep-research-2026-07-11.md:16)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gz-115 — 2026-07-12
- Claim: "The field-sim wave architecture ALREADY EXISTS and is correctly scoped. Claiming it "replaces"  (docs/design/bebop2-deep-research-2026-07-11.md:69)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0gz-116 — 2026-07-12
- Claim: "3. **Verified-by-Math.** Every fix ships a falsifiable RED+GREEN test. No false-green metrics."  (docs/design/bebop2-roadmap-2026-07-10.md:21)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0h0-117 — 2026-07-12
- Claim: "authoritative event→journal, invest in the prover path when CI allows."  (docs/design/bleeding-edge-EV-REANALYSIS-2026-07-08.md:35)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0h0-118 — 2026-07-12
- Claim: "## What it ACTUALLY proves (property-based, each with a real RED path)"  (docs/design/core-reverse-engineering-loop-2026-07-10.md:8)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0h0-119 — 2026-07-12
- Claim: "and the system asserts `gap(x) ≈ 0`. In the user's L5 framing ("double-rotational"  (docs/design/cycle-consistency-theorem.md:16)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0h0-120 — 2026-07-12
- Claim: "and ONLY after shadow has proven the false-positive rate is ~0. Never gate red-line"  (docs/design/cycle-consistency-theorem.md:98)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0h0-121 — 2026-07-12
- Claim: "**Math note (verified):** a damped wavefront with edge speed `F_uv = 1/W_uv` *is* the"  (docs/design/delivery-protocol/DECOUPLED-MATCHER.md:77)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0h0-122 — 2026-07-12
- Claim: "Fast Marching Method solving the Eikonal equation `|∇T| = 1/F`; Tsitsiklis (1995) proved"  (docs/design/delivery-protocol/DECOUPLED-MATCHER.md:78)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0h0-123 — 2026-07-12
- Claim: "# Matcher API — Open, Replicable Dispatch (kills DANGER #1)"  (docs/design/delivery-protocol/MATCHER-API.md:1)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0h0-124 — 2026-07-12
- Claim: "- `LocalMatcherClient` — runs `match_orders` **in-process**. The default: proves"  (docs/design/delivery-protocol/MATCHER-API.md:79)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0h0-125 — 2026-07-12
- Claim: "### Why this kills DANGER #1"  (docs/design/delivery-protocol/MATCHER-API.md:85)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0h0-126 — 2026-07-12
- Claim: "- `matcher_is_replicable_no_hidden_server` — RED+GREEN (the DANGER #1 killer):"  (docs/design/delivery-protocol/MATCHER-API.md:98)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0h1-127 — 2026-07-12
- Claim: "`Transport` prove any node serves identically over the wire"  (docs/design/delivery-protocol/SYSTEM-ARCHITECTURE-AUDIT.md:14)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0h1-128 — 2026-07-12
- Claim: "proves two nodes *compute the same output from the same input* — it says"  (docs/design/delivery-protocol/SYSTEM-ARCHITECTURE-AUDIT.md:18)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0h1-129 — 2026-07-12
- Claim: "the operator's "kill-switch at consensus, not in code" requirement, met."  (docs/design/delivery-protocol/SYSTEM-ARCHITECTURE-AUDIT.md:37)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0h1-130 — 2026-07-12
- Claim: "vault id (a hash of their public key) — verifiers prove authorship without"  (docs/design/delivery-protocol/SYSTEM-ARCHITECTURE-AUDIT.md:48)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0h1-131 — 2026-07-12
- Claim: "2. **What's the risk?** — fail-closed by design (guards refuse; PoD is non-repudiable; kill-switch is consensus, not a vendor's mood)."  (docs/design/delivery-protocol/SYSTEM-ARCHITECTURE-AUDIT.md:116)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0h1-132 — 2026-07-12
- Claim: "- **PoD hardware attestation** — `pod` proves *authorship* of a claim; it cannot"  (docs/design/delivery-protocol/SYSTEM-ARCHITECTURE-AUDIT.md:133)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0h1-133 — 2026-07-12
- Claim: "prove the courier was *physically present* without a hardware anchor (phone"  (docs/design/delivery-protocol/SYSTEM-ARCHITECTURE-AUDIT.md:134)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0h1-134 — 2026-07-12
- Claim: "Date: 2026-07-09 · Author: Hermes agent · Status: IMPLEMENTED + VERIFIED (all phases D0–D6 + Parts 1–4 landed; `npm run verify` GREEN at 434 pass / 0 fail)"  (docs/design/dev-system-anti-hallucination-plan-2026-07-09.md:3)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0h1-135 — 2026-07-12
- Claim: "dev/verification loop and kill agent (self-)hallucination — both mine and the project's."  (docs/design/dev-system-anti-hallucination-plan-2026-07-09.md:5)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0h1-136 — 2026-07-12
- Claim: "| D3 | `selectZenoh` + `prove` into kernel dispatch (flag-OFF, RED+GREEN) | **DONE 2026-07-09** — wired into the dispatch SHELL (`runDispatch`, not the pure ker"  (docs/design/dev-system-anti-hallucination-plan-2026-07-09.md:29)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0h1-137 — 2026-07-12
- Claim: "false-green risk this plan exists to kill."  (docs/design/dev-system-anti-hallucination-plan-2026-07-09.md:37)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0h1-138 — 2026-07-12
- Claim: "5. **Don't over-claim wiring.** "verified" ≠ "wired into runtime". Be explicit about"  (docs/design/dev-system-anti-hallucination-plan-2026-07-09.md:144)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0h1-139 — 2026-07-12
- Claim: "> · **(c)** over-claimed analogy = poetry. RED=refuted/fails-check,"  (docs/design/fable-protocol-2026-07-11/F1-protocol-vs-platform.md:5)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0h1-140 — 2026-07-12
- Claim: "- PRIMARY (Monegro 2016 *Fat Protocols*; Monegro 2020 *Thin Applications*): value can concentrate at the shared protocol layer *in crypto* because the data laye"  (docs/design/fable-protocol-2026-07-11/F1-protocol-vs-platform.md:41)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0h1-141 — 2026-07-12
- Claim: "- **Direct-ordering demand is empirically proven.** ChowNow charges **0% commission** on direct orders and survives on a **$99–$199/month SaaS fee** (ChowNow pr"  (docs/design/fable-protocol-2026-07-11/F1-protocol-vs-platform.md:53)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0h1-142 — 2026-07-12
- Claim: "- Replace single dispatcher with open matcher market + force-inclusion fallback (`SYSTEM-ARCHITECTURE-AUDIT.md` §6; `platform-vs-protocol-logistics.md:104`). Sh"  (docs/design/fable-protocol-2026-07-11/F1-protocol-vs-platform.md:78)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0h1-143 — 2026-07-12
- Claim: "- **Milestone M2-D (RED if > 80% of matches still served by one operator node after week 24** — proves re-centralization, the TradeLens failure mode)."  (docs/design/fable-protocol-2026-07-11/F1-protocol-vs-platform.md:79)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0h1-144 — 2026-07-12
- Claim: "the edge half is proven, but the settlement half is poetry until someone writes"  (docs/design/fable-protocol-2026-07-11/F3-architecture-hidden-centralization.md:256)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0h2-145 — 2026-07-12
- Claim: "issuer, consensus kill-switch). The ONE real exposure is the **SDK/bootstrap"  (docs/design/fable-protocol-2026-07-11/F3-architecture-hidden-centralization.md:322)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0h2-146 — 2026-07-12
- Claim: "make the hosted SDK a thin wrapper — kills DANGER #2 before it metastasizes."  (docs/design/fable-protocol-2026-07-11/F3-architecture-hidden-centralization.md:331)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0h2-147 — 2026-07-12
- Claim: "TS reference; the comparison below proves it is faster *and* bit-for-bit consistent."  (docs/design/field-sim-comparison-2026-07-09.md:16)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0h2-148 — 2026-07-12
- Claim: "> **Note on ceiling (documented, not hidden):** the run was capped at n=1000. At n≥2000 the JS-side `ArrayBuffer`/wasm linear-memory path (the dense `Laplacian("  (docs/design/field-vs-kdtree-scale-report-2026-07-09.md:41)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0h2-149 — 2026-07-12
- Claim: "1. **Spectral propagator (one-shot exp(−Lt))** — killed the 40-iteration SpMV chain. 10.9→15.7× over JS solely from removing the loop. This is the headline addi"  (docs/design/field-vs-kdtree-scale-report-2026-07-09.md:93)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0h2-150 — 2026-07-12
- Claim: "**Verified:** 530 TS tests + 8 Rust tests green; doc-gate clean; typecheck clean. The Mutex-deadlock fix (nested `with_graph` lock under native targets) is RED-"  (docs/design/field-vs-kdtree-scale-report-2026-07-09.md:113)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0h2-151 — 2026-07-12
- Claim: "## PART A — Findings applied (already landed, all RED+GREEN proven)"  (docs/design/integration-applied-findings-and-EV-plan.md:11)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0h2-152 — 2026-07-12
- Claim: "Bugs caught by grounding to external truth (Verified-by-Math doing its job):"  (docs/design/integration-applied-findings-and-EV-plan.md:26)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0h2-153 — 2026-07-12
- Claim: "within ε over N Verlet steps. (RED under explicit Euler — proves"  (docs/design/multi-channel-field-integration-plan-2026-07-11.md:54)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0h3-154 — 2026-07-12
- Claim: "primitives (ledger/killswitch/reputation). **Fail-closed law:** any timeout/ambiguity"  (docs/design/plan-audit-bebop-2026-07-11.md:134)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0h3-155 — 2026-07-12
- Claim: "Juror Schelling reward + bonded stake = sound theory, **zero code** (`F2:59-67`)."  (docs/design/plan-audit-bebop-2026-07-11.md:138)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0h3-156 — 2026-07-12
- Claim: "core (route + prove + attribute + replicable + fail-closed) is real and unusually"  (docs/design/plan-audit-bebop-2026-07-11.md:144)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0h3-157 — 2026-07-12
- Claim: "## INTEGRATED (native, verified)"  (docs/design/research-12tool-ev-2026-07-10.md:22)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0h3-158 — 2026-07-12
- Claim: "## 2. Findings (honest — corrected after a first pass over-claimed)"  (docs/design/reverse-engineering-loop-2026-07-09.md:13)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0h3-159 — 2026-07-12
- Claim: "src/doctrine.test` (strength 7.17) — the agent's memory/dispatch core is correctly the gravitational center."  (docs/design/reverse-engineering-loop-2026-07-09.md:41)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0h3-160 — 2026-07-12
- Claim: "- **NOISE / NOT NEEDED (pruned):** Nvidia/SkillSpector, Ideogram, music/TTS,"  (docs/design/tool-survey-2026-07-10.md:27)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0h3-161 — 2026-07-12
- Claim: "## Integrated Pass 3 — close the gaps + final 3 additions (Verified-by-Math)"  (docs/design/tool-survey-2026-07-10.md:87)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0h4-162 — 2026-07-12
- Claim: "- **Falsifiable**: `consciousness.test.ts` asserts maintenance reports health (GREEN) and that a"  (docs/features/consciousness.md:48)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0h4-163 — 2026-07-12
- Claim: "resonance flagged `RISKY` before any destabilizing gain change. `governor.test.ts` asserts"  (docs/features/governor.md:44)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0h4-164 — 2026-07-12
- Claim: "Recall is falsifiable: `memory.test.ts` asserts that the nearest concept to a known query is the"  (docs/features/memory.md:32)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0h4-165 — 2026-07-12
- Claim: "in-process storage. It never crashes on a missing external tool — verified by `memory.test.ts`."  (docs/features/memory.md:43)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0h4-166 — 2026-07-12
- Claim: "## Verified exchange"  (docs/features/mesh.md:17)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0h4-167 — 2026-07-12
- Claim: "- **Falsifiable** — `core.test.ts` asserts a piece's hash matches (GREEN) and that a corrupted"  (docs/features/mesh.md:47)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0h4-168 — 2026-07-12
- Claim: "| `bebop docs check` | asserts `openwiki/` + the CI workflow exist before a release |"  (docs/features/openwiki.md:27)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0h4-169 — 2026-07-12
- Claim: "## What "boot" proves"  (docs/getting-started.md:52)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0h4-170 — 2026-07-12
- Claim: "`bebop boot` runs the **guard-OS self-certification**: it asserts the gate *denies on red* and"  (docs/getting-started.md:54)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0h4-171 — 2026-07-12
- Claim: "*passes on green*. If the gate can't be proven to block the bad cases, Bebop refuses to run"  (docs/getting-started.md:55)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0h4-172 — 2026-07-12
- Claim: "| Skills | `SKILL.md` (frontmatter + body, `@`-mention/auto) | skills system | `src/skills.ts` loader (agent-skills fmt) |"  (docs/integrations/agent-parity.md:20)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0h4-173 — 2026-07-12
- Claim: "6. **`src/skills.ts`** — load `SKILL.md` files (agent-skills frontmatter) from `.bebop/skills/`;"  (docs/integrations/agent-parity.md:40)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0h4-174 — 2026-07-12
- Claim: "`loadSkills()` + `findSkill(query)`. One sample skill shipped (`/review`)."  (docs/integrations/agent-parity.md:41)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0h4-175 — 2026-07-12
- Claim: "read-only; skill loads. `tsc` clean; full suite green on both install paths."  (docs/integrations/agent-parity.md:53)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0h4-176 — 2026-07-12
- Claim: "the cheapest model that satisfies it:"  (docs/integrations/backends.md:29)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0h4-177 — 2026-07-12
- Claim: "- **Falsifiable** — `bebop.test.ts` asserts the routing decision for each class (RED+GREEN: redline→opus, doer→haiku, and a redline routed to haiku is a violati"  (docs/integrations/backends.md:50)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0h4-178 — 2026-07-12
- Claim: "- `auth.test.ts` asserts (GREEN) signup+login round-trips and issues a session, and (RED)"  (docs/integrations/sync.md:39)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0h4-179 — 2026-07-12
- Claim: "vendor's black box, so you can't prove what the robot was allowed to do."  (docs/narration/README-narration.md:26)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0h4-180 — 2026-07-12
- Claim: "replacement for human review of money or auth changes; when a human approves, the human owns it. It"  (docs/narration/README-narration.md:41)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0h4-181 — 2026-07-12
- Claim: "human approves, the human owns the consequence."  (docs/narration/limitations-narration.md:19)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0h5-182 — 2026-07-12
- Claim: "Every claim in Bebop is **falsifiable** (Verified-by-Math): an assertion that goes RED on bad input,"  (docs/wiki/Verification.md:3)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0h5-183 — 2026-07-12
- Claim: "`extern` calls to any transport** — verified by reloop v2."  (bebop2/ARCHITECTURE.md:96)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0h5-184 — 2026-07-12
- Claim: "PANICS on any `alloc::alloc` / vtable / HashMap call → proves the contract."  (bebop2/ARCHITECTURE.md:140)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0h5-185 — 2026-07-12
- Claim: "- reloop v2 asserts: wasm imports NOTHING (no I/O latency) + bounded `.text` size (icache/decode proxy)."  (bebop2/ARCHITECTURE.md:141)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0h5-186 — 2026-07-12
- Claim: "**Method:** 8 parallel adversarial agents (Fable model), read-only, each mapped to an attack surface + red-team skill (owasp-security / systematic-debugging / d"  (bebop2/RED-TEAM-REVIEW-2026-07-12.md:4)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0h5-187 — 2026-07-12
- Claim: "- **Protocol / authorization layer — insecure, and it was weaponized.** A random attacker can self-authorize the highest-privilege action, replay it across node"  (bebop2/RED-TEAM-REVIEW-2026-07-12.md:15)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0h5-188 — 2026-07-12
- Claim: "**Security posture in one line:** the break is **authorization and PQ validation, not the classical signature primitive** — the crypto foundation is sound; the "  (bebop2/RED-TEAM-REVIEW-2026-07-12.md:18)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0h5-189 — 2026-07-12
- Claim: "## 2. Attacker kill-chain — WEAPONIZED (runs today)"  (bebop2/RED-TEAM-REVIEW-2026-07-12.md:49)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

## ESC-mrhmo0h5-190 — 2026-07-12
- Claim: "- **"Zero-dep AGC-class" is true only for `core`.** `proto-wire` pulls **66 crates** including `tokio`, `tokio-tungstenite → native-tls → openssl → openssl-sys`"  (bebop2/RED-TEAM-REVIEW-2026-07-12.md:114)
- Kind: unbacked (no test/proof/citation in ±3 lines)
- Status: OPEN
- Arbiter: operator
- Resolution: <fill>

<!-- New ESC entries are appended by logic-gate.mjs above this line. -->
