# V5 — `RootDelegationPolicy`: three real variants are never dispatched, no rate-limiting

**File:** `bebop2/proto-cap/src/node_id.rs` (378 lines, cited @ `b87b7e2`)
**Known-finding #5.** Verdict: **REPRODUCES — refined.**
**Severity: MEDIUM** (an operator-configuration surface that implies distinct behavior it does
not deliver; no rate-limiting).

## Claim tested

> `RootDelegationPolicy` enum (OperatorSigned / WebOfTrust / FirstContactQr) is a pure marker
> with zero behavioral difference and no rate-limiting.

## What the code actually does

The enum (`node_id.rs:156-166`) now has **four** variants — `OperatorSigned`, `WebOfTrust`,
`FirstContactQr`, and a `Unspecified` (`:164-165`). `Default` is `Unspecified` (`:168-174`), and
`require_explicit_policy(p)` (`:179-184`) returns `Err(GenesisError::PolicyUnspecified)` for
`Unspecified` and `Ok(other)` for anything else.

**Refinement of the claim:** there *is* now one behavioral distinction — `Unspecified`
fails-closed vs "some policy chosen". That is a real, useful gate (the node won't bootstrap root
authority until the operator picks *something*).

**But the core claim holds for the three *real* variants:** `require_explicit_policy` treats
`OperatorSigned`, `WebOfTrust`, and `FirstContactQr` **identically** — all three fall through the
same `other => Ok(other)` arm. A fresh grep for `OperatorSigned` / `WebOfTrust` /
`FirstContactQr` across all of `bebop2/**.rs` (excluding the `node_id.rs` definition itself)
returns **zero matches**: nothing anywhere in the crate `match`es on *which* real policy was
chosen. There is:

- no operator-signature verification path for `OperatorSigned`,
- no transitive trust-seed evaluation for `WebOfTrust`,
- no out-of-band/QR handshake for `FirstContactQr`,
- and **no rate-limiting** attached to any of them.

So beyond the binary "chosen vs unspecified" gate, the three named policies are indistinguishable
markers: choosing `WebOfTrust` vs `OperatorSigned` changes **nothing** about how anchors are
admitted or how fast.

## Impact

An operator configuring `RootDelegationPolicy::OperatorSigned` may reasonably believe the node
will enforce offline operator-signed root certificates. It does not — the value is recorded and
then never consulted. This is a **false-affordance** risk: security-relevant configuration that
implies an enforcement it doesn't perform. Combined with V4 (unbounded issuance), the absence of
any per-policy rate-limit means even the *intended* stricter policies would not throttle anchor
bootstrap.

## Plans-vs-implementation

The doc comment (`:150-155`) is admirably honest that the root-delegation model is "an OPERATOR
decision" and lists all three models. The gap is that the plan/enum *enumerates* three
enforcement models while the implementation *enforces none of the three distinctly* — it only
enforces "you must pick one". The enum reads as a feature set; it is currently a selector with no
selectees wired.

## Remediation sketch

Either (a) implement per-variant behavior (operator-cert verification, WoT transitivity,
first-contact QR pinning — each with its own admission + rate-limit), or (b) collapse the enum to
the distinction that is actually enforced (`{Unspecified, Chosen}`) and move the three model names
to documentation/roadmap so the type does not advertise unenforced guarantees. If keeping the
three, add a `#[deprecated]`/`todo!`-style compile-time or test-time marker that fails until each
variant is dispatched, so the marker cannot silently masquerade as enforcement.
