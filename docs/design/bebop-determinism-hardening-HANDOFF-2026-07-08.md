# bebop — Determinism & Resilience Hardening (Investigation + Handoff)

> Date: 2026-07-08 · Operator directive (verbatim UA):
> "наступне досліди bebop, де можливо відмовитись від абстракцій та нестабільних систем і
> зробити ще більш детеремінованими та стійкими стики чи функціонал. Де можливо використати
> фундаментальну математику для ще кращого алгоритмічного результату?"
>
> Translation: investigate bebop for places to drop abstractions / unstable systems and make
> interfaces/functions more deterministic & resilient; where possible use fundamental math for a
> better algorithmic result.
>
> STATUS: INVESTIGATION COMPLETE. Findings below are evidence-backed (file:line cited). Three
> concrete fixes are specified and ready to implement with RED+GREEN proofs. Implementation was
> intentionally deferred in favour of this handoff (operator said "save the findings, prepare a
> session handoff"). Nothing was changed in code this turn.

---

## 0. What bebop already does WELL (keep it)

These are genuine strengths — do NOT regress them:

- **Zero RNG in the trust boundary.** `core-wasm.ts` loads a memory-safe WASM kernel; the Rust/WASM
  `decide` is the enforced guard. No `Math.random`/`Date.now` in `guard.ts`, `crypto.ts` (sign/verify),
  `memory.ts` (VSA is hashing-seeded, deterministic), `governor.ts` (pure math). Verified by reading
  every math-bearing module.
- **Falsifiable proofs exist.** `guard.ts::certifyGate` / `selfTest()`, `governor.test.ts` RED+GREEN,
  `verify-doc-claims.mjs` §I (ReAct). The Verified-by-Math bar is live.
- **Honest degradation.** `knowledge.ts::recall` falls back to local memory and says so; `core-wasm.ts`
  returns null when wasm is absent and callers use the TS port.

## 1. Findings (severity-ordered)

### F1 — CRITICAL: guard seam can silently DRIFT between WASM core and TS port
`guard.ts` claims parity ("parity-tested so both paths agree", lines 14-16) but there is **NO such
parity test armed in the running binary**. When `KERNEL` is present, `checkRedLine`/`checkScope` delegate
to `KERNEL.decide`; when absent, the TS `toRegExp` port runs. If the Rust globs/semantics ever diverge
from the TS `RED_LINE_GLOBS` / `DEFAULT_SCOPE_GLOBS`, the two engines disagree and the agent either
over-permits (silent bypass) or over-denies — with no detection. The TS port is the fallback "stability"
path, yet it is the one that can rot.
- **Resilience fix (specified, §A):** add `parityCheck()` that, when `KERNEL` is set, runs the same probe
  path-set through BOTH `KERNEL.decide` and the TS `toRegExp` port and asserts identical `ok`/`kind`.
  Wire it into `selfTest()`. Arming is automatic once the wasm is built; no-op pass when `KERNEL` is null.

### F2 — HIGH: crypto identity is non-reproducible (drop the unnecessary randomness)
`crypto.ts::createIdentity` (line 33) draws `ml_dsa65.keygen()` fresh every call → node identity is
**never reproducible from a seed**. Only Ed25519 takes a seed; the PQ keypair is always random. For a
mesh of self-certifying nodes this means a node can never be deterministically re-derived/recovered,
and tests can't pin an identity. Violates "drop unstable systems / make deterministic."
- **Resilience fix (specified, §B):** add `deriveIdentity(seed: Uint8Array)` that deterministically
  derives BOTH the Ed25519 secret AND the ML-DSA secret from the seed (HKDF-style expansion over
  `@noble/hashes` sha512), so identity is `id = nodeIdFromPublic(...) = f(seed)` and reproducible.
  Keep `createIdentity` (random) as the "generate new" path. Add RED+GREEN test.

### F3 — MEDIUM: VSA noise floor is a magic number, not principled math
`knowledge.ts` + `memory.ts`: the vector-recall floor is `n.sim <= 0.85` (knowledge.ts:53) — a hard-coded
constant with no derivation. The underlying VSA is a **char-codebook** (memory.ts:61-88): `embed(token)`
bundles per-character vectors, so expected Hamming similarity of two *unrelated* vectors is 0.5 (chance),
std σ = 0.5/√D = 0.5/√1024 ≈ 0.0156. A principled "reject noise" floor is `0.5 + k·σ` (k≈3 ⇒ ≈0.547).
0.85 is wildly conservative — which is *safe* (it never surfaces char-noise as confident) but it is
unexplained magic, and it means `nearest()` essentially never returns vector hits (every cross-token
similarity is meaningful char-overlap only).
- **Better-math fix (specified, §C):** name the constant `VSA_NOISE_FLOOR` and document the bound
  `0.5 + 3σ` where σ = 0.5/√VEC_DIM. Add a falsifiable test: random/char-noise query returns `[]` at the
  floor (GREEN), and *lowering* the floor to 0.6 lets noise through (RED) — proving the floor is not a
  no-op. Do NOT lower 0.85 to 0.55 unconditionally; the conservative value is correct for "never present
  noise as confident." The win is *principled naming + a proof it bites*.

### F4 — BUG: governor thermo floor compares mismatched units (cost vs Joules)
`governor.ts:224-226`: `thermoFloorHit = s.cost < landauerFloor(bitsErased(s.volume))`. `s.cost` is
"resource units" (TelemetrySample:17) but `landauerFloor` returns **Joules** (governor.ts:124-128,
≈2.87e-21 J/bit). The comparison is dimensionally inconsistent: a normal `cost: 1e-18` (resource units)
is compared against ~2e-20 J and reads "floor not hit" — but the numbers are in different units. The flag
can never meaningfully fire. This is a textbook "unstable abstraction" (mixing a physics constant with an
uncalibrated UI metric).
- **Fix (specified, §D):** either (a) rename to `costBits` and compare bits→Joules consistently, or
  (b) drop the cross-unit comparison and instead assert `cost >= bitsErased(volume)` in *resource-unit*
  space (cheapest adequate: you must spend ≥1 unit per bit touched). Option (b) is simpler and keeps the
  "thinking isn't free" semantics without a fake physics equality. Flag honestly, recommend (b).

### F5 — LOW: temp-file nondeterminism in `estimateTokens`
`knowledge.ts:100`: `path.join(os.tmpdir(), '.bebop-recall-${process.pid}-${Date.now()}.json')`. Uses
`process.pid`+`Date.now` → filename varies per call (not a correctness bug — the file is tmp and cleaned
up — but it is needless nondeterminism and a (tiny) race if called concurrently). Replace with a
content-addressed temp name (sha of text) so identical input → identical path, and it's collision-safe.

---

## 2. Ready-to-implement fixes (each with RED+GREEN proof)

### §A — Guard parity self-check (`src/guard.ts`)
Add after `selfTest()`:
```ts
// Armed only when KERNEL present. Proves the TS port and the WASM core agree on the
// same red-line/scope verdicts — catches seam drift deterministically at boot.
const PARITY_PROBES = [
  'packages/db/migrations/002_users.sql',   // must DENY (redline)
  'apps/api/src/routes/payments.ts',        // must DENY (redline)
  'tools/bebop/src/loop.ts',                // must ALLOW
];
export function parityCheck(): { ok: boolean; mismatches: string[] } {
  if (!KERNEL) return { ok: true, mismatches: [] }; // nothing to compare against
  const mism: string[] = [];
  for (const p of PARITY_PROBES) {
    const rust = KERNEL.decide(p, 'edit');
    const tsRed = toRegExp; // use the same probe the TS port uses
    const ts = checkRedLine(p); // runs TS port (KERNEL is set → delegates! see note)
    // NOTE: because KERNEL is set, checkRedLine delegates to KERNEL. So to compare, call the
    // TS port logic directly. Extract a `checkRedLineTs(targetPath, extraGlobs)` pure fn and
    // compare checkRedLineTs(p) vs KERNEL.decide(p). Assert ok+kind match.
    if (rust.ok !== ts.ok) mism.push(`${p}: rust.ok=${rust.ok} ts.ok=${ts.ok}`);
  }
  return { ok: mism.length === 0, mismatches: mism };
}
```
Refactor `checkRedLine` to split the TS port into a pure `checkRedLineTs(targetPath, extraGlobs)` and
have both the `KERNEL`-null branch and `parityCheck` call it. Wire `parityCheck()` into `selfTest()`:
`ok = ... && parityCheck().ok`. Add test: a fake KERNEL that disagrees flips `parityCheck().ok=false`
(RED), and an agreeing one passes (GREEN).

### §B — Deterministic identity (`src/crypto.ts`)
```ts
import { sha512 } from '@noble/hashes/sha2.js';
import { bytesToHex } from '@noble/hashes/utils.js';
// HKDF-lite: expand seed → two 32-byte secrets (ed + pq-deterministic).
export function deriveIdentity(seed: Uint8Array): NodeIdentity {
  const edSecret = sha512(seed).slice(0, 32);
  const pqSeed   = sha512(bytesToHex(seed) as unknown as Uint8Array).slice(32, 64);
  // NOTE: @noble/post-quantum ml_dsa65 does NOT accept a seed directly in current API.
  // Confirm API; if no seed param, derive pqSecret via sha512(pqSeed) and use
  // ml_dsa65.keygen()'s secret-recovery path, OR keep PQ random and only Ed deterministic
  // (documented). Prefer: Ed deterministic + PQ deterministic if API allows.
  ...
}
```
GREEN test: `deriveIdentity(seedA).id === deriveIdentity(seedA).id` (reproducible) and `!== seedB`.
RED test: `createIdentity()` (random) gives two different `id`s across calls (proves randomness path
still differs). **Verify `@noble/post-quantum` seed API before writing** (the current code uses
`ml_dsa65.keygen()` with no seed — that's the investigation gap, see §B note).

### §C — VSA noise floor (`src/knowledge.ts` + `src/memory.ts`)
```ts
// memory.ts: principled bound. Unrelated bipolar vectors have E[sim]=0.5, σ=0.5/√D.
export const VSA_EXPECTED_SIM = 0.5;
export const VSA_NOISE_FLOOR = VSA_EXPECTED_SIM + 3 * (0.5 / Math.sqrt(VEC_DIM)); // ≈0.547
// knowledge.ts: replace `if (n.sim <= 0.85) continue;` with
if (n.sim <= VSA_NOISE_FLOOR) continue;
```
GREEN test: random/char-noise query → `recall`/`nearest` returns `[]` above floor. RED test: temporarily
use floor 0.6 → noise passes (proves floor bites). Keep the conservative value for "never present noise
as confident."

### §D — Governor thermo unit fix (`src/governor.ts`)
Replace cross-unit compare with resource-unit-space assertion:
```ts
// cost is in resource-units; bitsErased is a lower bound on units that MUST be spent.
this.thermoFloorHit = s.cost < bitsErased(s.volume); // both resource-units now
```
And update `TelemetrySample.cost` doc + the test that used `cost: 1e-18` (change to a unit-comparable
value, e.g. `cost: 1, volume: 100` → bitsErased(100)=7 → 1<7 → thermoFloorHit=true, GREEN). Add RED:
`cost: 100, volume: 1` → bitsErased=1 → 100>=1 → not hit.

---

## 3. Verification state (this turn)
- **No code changed.** All findings are from static reading of `src/{memory,governor,router,crypto,
  guard,knowledge,core-wasm}.ts` + their `.test.ts`.
- Re-ran the full suite before the handoff request to confirm baseline is green:
  `npm test` → 181 passed / 0 fail; `verify-doc-claims.mjs` → exit 0; `cargo test -p bebop-core` → 7
  passed. (These are the pre-investigation numbers; the investigation added no regressions because it
  added no code.)
- `bebop` Rust crate (`crates/bebop`) has PRE-EXISTING compile errors (missing `pricing::PriceInputs`,
  `OwnerCatalogStates` fields) — unrelated to this work; not touched.

## 4. Recommended next session
1. Implement §A (guard parity) — highest leverage for "resilient seam."
2. Implement §B (deterministic identity) — verify `@noble/post-quantum` seed API first.
3. Implement §C (name floor + falsifiable test).
4. Implement §D (thermo unit fix) + update governor.test.ts.
5. Fix §F5 (content-addressed tmp name) — trivial.
6. Re-run `npm test` + `verify-doc-claims.mjs`; update VERIFICATION-MATRIX + CHANGELOG.

## 5. File pointers (absolute)
- /root/bebop-repo/src/guard.ts (F1: lines 12-19, 77-111, 136-164)
- /root/bebop-repo/src/crypto.ts (F2: lines 32-39, 42-49)
- /root/bebop-repo/src/knowledge.ts (F3: lines 53; F5: line 100)
- /root/bebop-repo/src/memory.ts (F3: lines 27, 55-59, 61-88)
- /root/bebop-repo/src/governor.ts (F4: lines 13-19, 124-133, 224-226)
- /root/bebop-repo/src/core-wasm.ts (context: lines 33-92)
