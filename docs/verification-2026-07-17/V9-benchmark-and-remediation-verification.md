# V9 — Benchmark / remediation verification (charter: "correspondence to plans")

The charter asked specifically for **benchmark verification** and **correspondence to what is
claimed in plans**. This doc records verification *results* (including a positive one) surfaced by
the 3-model overlap review that the first pass had not covered. Verified fresh against `b87b7e2`.

---

## V9.1 — RED-TEAM §3B "#1 CRITICAL: ML-DSA samples A from CBD" is GENUINELY REMEDIATED (confirmed-holds)

**Files:** `bebop2/core/src/pq_dsa.rs`, `bebop2/core/src/pq_dsa/acvp_tests.rs`,
`bebop2/core/kat/acvp/` · Verdict: **CLAIM HELD / remediation confirmed.**

`RED-TEAM-REVIEW-2026-07-12.md` named as its **single most important finding** that the
post-quantum claim was *false*: ML-DSA sampled the public matrix **A from CBD** (center-binomial)
instead of the FIPS-204 uniform/rejection sampler, which would make the lattice trivially
breakable. The charter's "benchmark verification / correspondence to plans" demand makes checking
this the highest-value item — and the first pass missed it.

**It is fixed, and the fix is real (verified byte-level):**
- `pq_dsa.rs:7` documents the *old* bug in a comment ("…secrets s1/s2 with center-binomial CBD
  instead of the FIPS uniform/rejection samplers…").
- ExpandA now uses **rejection sampling to uniform** in the NTT domain: `poly_uniform(rho, nonce)`
  (`:308-320`) calls `rej_uniform` (`:290`) — RejNTTPoly, the correct FIPS-204 §Algorithm. ExpandS
  uses `poly_uniform_eta` (RejBoundedPoly, `:349-359`); ExpandMask uses `poly_uniform_gamma1`
  (`:363`). This is the FIPS sampler set, not CBD-for-A.
- **ACVP provenance is real**, not hand-rolled: `pq_dsa/acvp_tests.rs` parses the **vendored
  official NIST ACVP known-answer vectors** (`bebop2/core/kat/acvp/{key-gen,sig-gen,sig-ver}.json`,
  **vsId 42, revision FIPS204, isSample=false**) — the canonical ACVP-Server export — with **one
  discrete `#[test]` per `tcId`** so a single vector failing is a pinpoint failure. This is a
  genuine external-KAT benchmark, and it is committed and part of the 858-test green suite.

**Verdict:** the project's own #1 CRITICAL is closed with the correct algorithm *and* an
independent NIST-vector benchmark. The corpus credits it here. (This is the kind of positive
"claim held" result an honest red-team owes alongside the breaks.)

---

## V9.2 — Transport confidentiality (RED-TEAM §3D): `WssStream::Plain` still present (LOW, low-confidence)

**File:** `bebop2/proto-wire/src/wss_transport.rs:47-48` · Verdict: **RESIDUAL — needs the
owner's call.**

`REMEDIATION-BLUEPRINT §3B` (transport row) prescribes: *"mandatory rustls TLS 1.3 on `accept()`
(**delete** the `MaybeTlsStream::Plain` path)"*. On `b87b7e2` a `WssStream::Plain(Box<dyn
AsyncReadWrite + Unpin + Send>)` variant still exists (`:47-48`, used at `:431`). The doc comment
(`:42-45`) frames it as the `ws://` / **loopback-tests** path and notes it wraps a boxed IO object
"so that tests can drive the stream", i.e. it reads as test/loopback plumbing rather than a
production `accept()` path — and `iroh_transport.rs` uses `ring` and bans `native-tls`.

**Why LOW / low-confidence:** I did not trace whether any *production* `accept()` can select
`Plain` (the newer iroh/QUIC transport may have superseded the wss path entirely). So this is
flagged as a **residual to confirm**, not an asserted break: the blueprint said "delete Plain",
the variant is still in the tree, and whether it is prod-reachable or test-only is the open
question. Recommend the transport owner either delete it or gate it `#[cfg(test)]` to make the
"no plaintext in production" property structural rather than convention.

---

## Note on scope

Transport (proto-wire TLS/QUIC) was otherwise outside this pass's depth; V9.2 is included only
because the overlap review surfaced it as a comparable-severity area left unexamined. A dedicated
transport-confidentiality audit is the right follow-up if the `Plain` path turns out prod-reachable.
