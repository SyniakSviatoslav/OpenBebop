# V4 — Unbounded delegation issuance + no chain-depth cap (Sybil + verify-cost DoS)

**File:** `bebop2/proto-cap/src/roster.rs` (654 lines, cited @ `b87b7e2`)
**Known-finding #4.** Verdict: **REPRODUCES — sharpened with a fresh DoS angle; two nuances
corrected after review.**
**Severity: MEDIUM–HIGH** (adds an algorithmic-complexity DoS on every gate check; the
Sybil half is a policy-assumption gap, not a falsified plan claim — see review note).

> **Review note (3-model overlap), verified against the tree:**
> 1. Chain length is **not truly "unbounded"** — `proto-wire/src/framing.rs:18` caps an envelope
>    at **1 MiB** (lowered from 8 MiB, 2026-07-14), and a signed delegation link is ~3.3 KB
>    (ML-DSA-65 sig), so a chain is bounded at **~hundreds of links** per frame. That is still a
>    real O(N) verify-cost amplification (hundreds of Ed25519 verifies forced per submitted
>    frame), but it is *frame-cap-bounded*, not infinite. Wording corrected below.
> 2. The planning docs' **Sybil story is at the wire layer** (roster enrollment + S/Kademlia
>    node-id puzzles), not "bounded delegation issuance". So the issuance-count half attacks a
>    property the plans don't actually claim in code terms — it is a genuine *gap* (nothing bounds
>    issuance) but not a *contradiction of a stated code guarantee*. Severity reflects that.

## Claim tested

> `AnchorRoster`/`verify_chain` allow unbounded per-anchor delegation issuance — a real
> Sybil-resistance weakness, since the whole capability-based (not reputation-based) Sybil
> defense depends on issuance being costly/bounded.

## What the code actually does

`verify_chain(roster, chain, cap, now)` (`roster.rs:252-316`) walks the delegation chain and
enforces a genuinely sound UCAN-subset lattice:

1. root issuer ∈ roster (`:260-263`) — kills self-issue;
2. per-link signature verifies against the issuer (`:274-275`);
3. `effect ⊆ scope` per link (`:277-279`);
4. chain alignment `child.issued_by == parent.subject` (`:281-285`);
5. **narrow-only** attenuation `link.scope ⊆ parent.scope` (`:288-292`, a real set-subset since
   the G4 fix);
6. tail subject binds to `cap.subject_key` (`:299-302`); requested effect ⊆ tail scope
   (`:304-308`); expiry per-link and per-cap (`:271-273`, `:310-313`).

This is correct as *lattice* enforcement. **What it does not have is any bound on issuance:**

- **No breadth bound.** An enrolled anchor may sign an unlimited number of `Delegation`s to
  unlimited child keys, each granting the same (or narrower) scope. Nothing counts or rate-limits
  issuance.
- **No depth bound.** A child may sub-delegate (as long as scope narrows and chain aligns), and
  `verify_chain` iterates `for link in chain` (`:269`) over a slice of **any length**. A fresh
  grep for `MAX*DEPTH` / `MAX*CHAIN` / `depth` in `proto-cap/src/` returns **nothing** — there is
  no `MAX_CHAIN_DEPTH`.

The module's "Honest bound" section (`:29-34`) only asserts that the **roster** is frozen at
genesis. It says nothing about delegation issuance being bounded — because it isn't.

## Two distinct weaknesses

### (a) Sybil-resistance is asserted, not enforced

The whole design premise (stated across the crate) is "capability-based, not reputation-based"
Sybil resistance: identities are cheap, but *authority* is supposed to be bounded because it must
be delegated from a scarce anchor. In practice, once an anchor delegates any scope to attacker
key A, A can mint an **unbounded** fan-out/fan-depth of further delegations within that scope —
each a fresh "identity" holding real authority. The scarcity that Sybil-resistance depends on
(costly/bounded issuance) is **not present in code**. The defense reduces to "trust that anchors
delegate sparingly", which is a policy assumption, not an enforced invariant.

### (b) Fresh: no chain-depth cap ⇒ frame-cap-bounded algorithmic-complexity DoS

Every `HybridGate::check` (and thus every `KernelFacade::submit_intent`) calls `verify_chain`,
which performs **one Ed25519 signature verification per link** (`:275`). There is no
`MAX_CHAIN_DEPTH` in `proto-cap`, so the only ceiling on chain length is the **1 MiB wire frame
cap** (`proto-wire/src/framing.rs:18`) — at ~3.3 KB per signed link that permits **hundreds of
links per frame**. An authorized-but-malicious delegate can construct a validly-signed chain that
long (A→B→C→…→leaf, all keys under its control, each link correctly signed) and attach it to a
frame; the verifier is then forced to perform hundreds of expensive verifications per submitted
frame — an asymmetric CPU-exhaustion vector (attacker pays O(N) signing once, victim pays O(N)
per replay attempt). It is *bounded* by the frame cap, not infinite, but there is no early
depth-check to reject an over-long chain cheaply before the verify walk.

## Plans-vs-implementation

Plans claim a Sybil-resistant, capability-scoped mesh with bounded authority. The implementation
enforces the *lattice* (attenuation, anchoring) rigorously but leaves *issuance quantity* and
*chain length* completely unbounded — so the "bounded authority" property is aspirational, and the
verifier has no DoS ceiling.

## Remediation sketch

1. Add `MAX_CHAIN_DEPTH` and reject longer chains **before** the verify loop (cheap length check
   first). A small constant (e.g. 8) covers realistic anchor→…→leaf depths.
2. If bounded issuance is a real requirement (not just lattice narrowing), add an explicit,
   signed issuance budget/counter per anchor (or per delegation), or document loudly that
   issuance quantity is a governance/policy concern outside cryptographic enforcement — so the
   Sybil claim is not read as code-enforced when it is not.
