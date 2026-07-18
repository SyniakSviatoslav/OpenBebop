# V1 — Unauthenticated, irreversible revocation via unsigned gossip/merge

**File:** `bebop2/proto-cap/src/revocation.rs` (244 lines, cited @ `b87b7e2`)
**Known-finding #1.** Verdict: **REPRODUCES — sharpened.**
**Severity: HIGH** (mesh-wide griefing DoS, monotonic/irreversible, no origin authentication).

## Claim tested

> `revoke_key`/`drop_anchor` are unauthenticated (no signature check), irreversible once
> triggered.

## What the code actually does

`RevocationSet` is an append-only, monotonic invalidate set with two namespaces
(`revoked_keys: HashSet<[u8;32]>`, `revoked_cap_hash: HashSet<[u8;32]>`). It is consumed by
`HybridGate::check` — a revoked key or capability hash is rejected as `CapError::Revoked` even
when signature, chain, expiry are all valid (`hybrid_gate.rs:159-168`). That part is sound and
well-tested.

The authorization on **who may add a revocation** is the finding:

- `revoke_key(&mut self, key)` (`revocation.rs:69-71`) and `revoke_capability(&mut self, ...)`
  (`:76-78`) insert unconditionally. Their only gate is Rust ownership of `&mut self` — there
  is **no signature, no capability, no operator check** on the *content* being revoked.
- `drop_anchor(roster: &mut AnchorRoster, key)` (`:105-107`) likewise removes an anchor purely
  on `&mut` access.
- The teeth are in the **gossip / anti-entropy path**: `merge(&mut self, other)` (`:94-98`)
  folds *another peer's entire revocation set* in by set-union, and `gossip_payload()`
  (`:114-120`) serialises the namespaces as bare sorted 32-byte id lists. **Neither carries or
  checks any signature, origin proof, or issuer authority.** The module doc itself says: "A
  real mesh would gossip this set between peers so every node converges" and (`:109-113`)
  "consensus-grade propagation … is a future upgrade; this is the anti-entropy primitive every
  node can run today."

## Exploit (red-team)

Because revocation is (a) content-unauthenticated and (b) monotonic with **no `unrevoke`**
(module doc `:46-47`: "there is deliberately no `unrevoke`"), a peer that participates in the
gossip mesh can inject `revoke_key(victim_pubkey)` for **any** 32-byte key — including a
competitor's or an anchor's — and every node that runs `merge` on that payload will thereafter
**permanently** reject the victim's capabilities. There is no recovery path short of re-keying
the victim and re-enrolling. One malicious (or compromised) gossip participant can therefore
grief arbitrary identities off the mesh irreversibly.

`drop_anchor` is the same class against the *roster*: an unauthenticated `&mut` caller (or a
future wire path that exposes it) can strip a legitimate anchor's vouch power.

## Plans-vs-implementation

The module frames revocation as "closing the single biggest authz hole in the line". It does
close the *expiry-only* hole for the **local** operator. But the **mesh** story it advertises
(gossip convergence) has the inverse hole: it converges on **whatever any peer asserts**, with
no proof the asserter was entitled to revoke that target. The plan implies trust-anchored
authority everywhere; the revocation-propagation primitive has none.

## Severity rationale

HIGH but **latent on this branch**: the destructive reach depends on the gossip transport
actually being wired peer-to-peer. On `b87b7e2` `merge`/`gossip_payload` are in-tree primitives;
MESH-07/09 (this very commit) is building the sync/transport that would carry them. So this is a
*design-incomplete* HIGH: harmless in a single-node build, mesh-wide irreversible DoS the moment
gossip is live — which is the stated direction.

## Remediation sketch

Require every gossiped revocation entry to be a **signed statement** whose issuer is authorised
to revoke that target (e.g. the anchor that rooted the target's delegation chain, or the target
itself for self-revocation). `merge` must verify each entry's signature/authority before union;
reject or quarantine unsigned entries. Consider a bounded, signed **revocation certificate**
(subject, revoker, reason, monotonic counter) rather than a bare 32-byte id. This is exactly the
"Vouchsafe / Lingering-Authority" upgrade the code defers.
