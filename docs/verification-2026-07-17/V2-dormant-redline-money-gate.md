# V2 — Red-line money/settlement gate is dormant at the only production seam

**Files:** `bebop2/proto-cap/src/hybrid_gate.rs`, `bebop2/proto-cap/src/facade.rs`
(cited @ `b87b7e2`)
**Known-finding #2.** Verdict: **REPRODUCES — cited path corrected.**
**Severity: HIGH** (a validly-signed money/settlement/secrets/migrations capability passes the
production boundary unbraked).

## Path correction (verify-fresh)

The known finding cited `bebop2/core/src/hybrid_gate.rs`. **No such file exists.** Both
`hybrid_gate.rs` and `facade.rs` live in **`bebop2/proto-cap/src/`**, not `core/src/`. The
substance reproduces; the path was imprecise.

## Claim tested

> `HybridGate::new` sets `redline: None`; the production facade never arms it — a dormant
> money/settlement safety gate.

## What the code actually does

`HybridGate` carries `redline: Option<RedLinePolicy>` (`hybrid_gate.rs:65`). Two constructors:

- `HybridGate::new(policy)` (`:75-81`) → `redline: None` (**unarmed**).
- `HybridGate::new_redlined(policy, redline)` (`:89-95`) → `redline: Some(..)` (**armed**).

When armed, `check` enforces it: `RedLineGate::check(&frame.capability.scope, rl)` →
`CapError::RedLineViolation` on a red-line scope (`:150-154`). This is **real and tested** —
`g5_deny_by_default_rejects_red_line_capability` (`:637-662`) proves a fully-valid, anchored,
classical+PQ-signed `Ledger / SettlementRecorded` frame that passes crypto is nonetheless
**rejected** once the gate is armed. So the brake works.

**The problem is the only production entry point never arms it.** `KernelFacade` is documented
(`facade.rs:1-16`) as "the *only* boundary `proto-cap` exposes to the host kernel" — the single
seam through which money semantics are reached. Its constructor builds the gate with the
**unarmed** form:

```rust
// facade.rs:96-104  (KernelFacade::new)
KernelFacade {
    gate: HybridGate::new(policy),   // ← line 97: UNARMED, redline: None
    ...
}
```

There is **no** `with_redline`-style builder on the facade (it has `with_allowed_reads`
(`:109-112`) but nothing for the red-line). A fresh grep confirms `RedLinePolicy` /
`new_redlined` are referenced **only** in `redline.rs` and `hybrid_gate.rs` (defs + unit
tests) — `facade.rs` references neither. So a host wiring `proto-cap` through the facade has
**no API path to arm the red-line at all**.

## Exploit / impact

Any capability whose scope touches money/auth/secrets/migrations — e.g.
`Ledger / SettlementRecorded` — that is validly signed and anchor-rooted flows straight through
`KernelFacade::submit_intent` to the host kernel's `EventSink`. The `hybrid_gate.rs:83-88` and
`:88` doc says "Production MUST arm it via `HybridGate::new_redlined` … the red-line gate is the
missing brake on validly-signed money/claim mutations (blueprint gap G5)" — but the facade
offers no way to comply. The brake exists in the codebase and is proven by tests, yet is
**structurally unreachable** from production.

## Plans-vs-implementation

Blueprint gap **G5** explicitly names this brake as required for money/settlement safety, and
the tests demonstrate it. The gap between plan and implementation is not the gate logic — it is
the **wiring**: the facade, the sole prod seam, hard-codes the unarmed constructor. A reader
auditing only the unit tests would conclude the red-line is enforced; a reader auditing the prod
path finds it dormant.

## Remediation sketch

Add `KernelFacade::new_redlined(policy, roster, revocations, redline, sink, clock)` (or a
`with_redline(RedLinePolicy)` builder mirroring `with_allowed_reads`) that constructs the gate
via `HybridGate::new_redlined`, and make the production wiring use it with
`RedLinePolicy::DenyByDefault`. Add a facade-level test (parallel to
`production_facade_rejects_absent_pq_require_both`) asserting a `Ledger/SettlementRecorded`
frame is `RedLineViolation` through `submit_intent`, pinning the armed prod path so a silent
revert is caught.
