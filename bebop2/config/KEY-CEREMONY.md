# bebop2 HUB — KEY CEREMONY (P10 §4)

This document is the operational runbook for producing the **genesis** trust
roots the hub boots from: delegation-chain **anchors** and unilateral
**kill anchors** (M9). Boot is **fail-closed** — a missing/empty/malformed
`config/genesis.txt` refuses to start.

## Threat model recap

- **Anchors** are the roots of the proto-cap delegation chain. Anything they (or
  their attenuated delegates) sign is authoritative. Compromise = full authority
  compromise.
- **Kill anchors** hold **unilateral** M9 halt authority. A *single* valid
  kill-anchor signature HALTS the hub (after a confirmed COLD backup). There is
  **no quorum** on this path — this is deliberate and is the OPPOSITE of the
  legacy `crates/bebop/src/guard.rs::KillSwitch` (a ≥2/3 consensus vote registry,
  which is NOT on the M9 kill path).

Because a kill anchor can stop the whole hub, treat its private key with the
same or greater care than an anchor key.

## Ceremony steps

1. **Air-gapped generation.** On an offline machine, generate each keypair with
   the from-scratch Ed25519 in `bebop2-core::sign::keygen(&seed)`. Derive `seed`
   from a hardware RNG / dice; never from a network source.
2. **Split custody.** Store each private seed under M-of-N custody (e.g. Shamir
   split across separate hardware tokens held by distinct operators). The public
   key (64-hex) is what goes in genesis; the seed NEVER touches the hub host.
3. **Record the public keys.** Write each 64-hex Ed25519 public key into
   `config/genesis.txt`:
   - `anchor <64-hex>` for a delegation-chain root,
   - `kill_anchor <64-hex>` for a unilateral kill authority.
4. **Separate the roles.** Do NOT reuse an `anchor` key as a `kill_anchor`. A
   kill signing operation should require pulling the kill custody shares
   specifically, so an accidental delegation signature can never also halt.
5. **Verify before deploy.** Confirm the hub boots against the genesis on a
   staging host, then confirm a test `KillOrder` signed by a kill anchor halts
   only AFTER a COLD snapshot receipt (never before).

## Signing a KillOrder

A `KillOrder` is signed over the canonical domain
`BEBOP2/OPERATOR-KILL/v1 || anchor || nonce || len(reason) || reason`
(see `mesh-node::kill_switch::KillOrder::signing_domain` — no serde on the signed
path). Each order carries a fresh `nonce`; the hub's replay ledger rejects any
nonce already seen. Reuse of a previously-accepted order is refused.

## Rotation & revocation

- To rotate an anchor: enroll the new key in genesis, redeploy, then drop the old
  key. Outstanding delegations under the old key stop verifying once it is gone.
- To revoke a compromised subject key at runtime, use the operator revocation
  verb (F5): it enforces locally and hands a delta to the P9 gossip seam so peers
  converge.

## What NEVER to do

- Never commit a real private seed to the repo or any host filesystem.
- Never ship the placeholder keys in `config/genesis.example.txt`.
- Never bypass the COLD-backup-then-halt ordering — the hub must snapshot
  durably before it halts.
