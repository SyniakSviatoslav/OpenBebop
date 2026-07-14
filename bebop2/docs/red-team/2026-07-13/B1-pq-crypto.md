# B1 — PQ Crypto Red-Team (bebop2 `feat/logic-governance`, 2026-07-13)

**Target:** `/root/bebop-repo/bebop2` @ branch `feat/logic-governance` (HEAD `d94f013`).
**Scope:** `core/src/pq_dsa.rs` (ML-DSA-65 / FIPS 204), `core/src/pq_kem.rs` (ML-KEM-768 / FIPS 203),
`core/src/rng.rs`, `core/src/pq_dsa/acvp_tests.rs`, `proto-cap/src/hybrid_gate.rs`.
**Method:** read the `.rs` source line-by-line; ran `cargo test -p bebop2-core --release` (160 passed / 0 failed);
inspected the vendored ACVP JSON (`core/kat/acvp/`); traced provenance; every claim below is `file:line`.
**Prior art re-verified:** `RED-TEAM-REVIEW-2026-07-12.md` §3B claimed the PQ layer was "broken and unvalidated —
ML-DSA security destroyed; no external KAT; not FIPS/interoperable." **That review is now materially STALE for
ML-DSA and I refute it below; it remains substantially TRUE for ML-KEM.**

---

## 1. Bottom line — is the "post-quantum" claim true right now?

**Split verdict. It is now TRUE for the ML-DSA-65 signature primitive, and FALSE for everything else that the
word "post-quantum" is supposed to buy you.**

- **ML-DSA-65 (signatures): GENUINELY FIPS-204-correct and interoperable — this is a real, verified fix.**
  The current branch vendors the **authentic NIST ACVP FIPS204 vectors** (`core/kat/acvp/{key-gen,sig-gen,sig-ver}.json`,
  vsId 42) and asserts **byte-exact** agreement. All 60 ML-DSA-65 cases pass (25 keyGen pk+sk, 20 sigGen sig,
  15 sigVer valid/invalid) plus 3 count-guards — `cargo test` shows `160 passed; 0 failed`. The prior review's
  central break (`expand_a` sampling A from a centered-binomial instead of uniform) is **fixed**: `poly_uniform`
  (`pq_dsa.rs:306`) now does uniform rejection sampling (`rej_uniform`, `:287`). Sizes are FIPS-exact
  (pk 1952, sk 4032, sig 3309, c̃ 48). **This green is real, not a false-green** (provenance proof in §4).

- **ML-KEM-768 (key encapsulation): NOT FIPS-203-interoperable, NOT externally validated, NOT side-channel-safe.**
  The KEM stores key material in the **coefficient domain**, not the FIPS-203-mandated NTT domain
  (`pq_kem.rs:473-474, 604, 616, 622`) — its `ek`/`dk`/`ct` bytes cannot match any conformant ML-KEM. There is
  **no external KEM KAT at all** (`core/kat/acvp/` holds only ML-DSA files); the only tests are self-consistency
  (roundtrip) and a **circular** "dual-impl" check. The "ML-KEM-768 (FIPS 203)" label is currently inaccurate.

- **At the protocol layer there is ZERO post-quantum protection in force.** The ML-DSA-65 primitive, though now
  correct, is **not wired into any verification path**. `proto-cap`'s hybrid gate verifies only classical Ed25519;
  the PQ leg is an explicit TODO (`hybrid_gate.rs:72-89`, `signed_frame` `sign_pq/verify_pq` unimplemented). A frame
  is accepted on the classical signature alone. So a quantum adversary who breaks Ed25519 has full capability forgery
  today — the "hybrid" is classical-only.

**One line:** the *signature primitive* is now trustworthy and FIPS-conformant; the *KEM* is a plausibly-designed but
unvalidated, non-interoperable, non-constant-time MLWE KEM; and the *protocol* enforces no PQ at all. "Post-quantum"
is true in the crate, false on the wire.

---

## 2. Scorecard per primitive

| Primitive | FIPS-conformant | External KAT | Constant-time | Wired into protocol | Verdict |
|---|---|---|---|---|---|
| **ML-DSA-65** (`pq_dsa.rs`) | **YES — bit-exact** | **YES — NIST ACVP vsId 42, 60/60** | Arithmetic branchless; rejection loop data-dependent (standard) | **NO** (TODO) | **Real & correct primitive; unused** |
| **ML-KEM-768** (`pq_kem.rs`) | **NO** — coeff-domain packing (`:473-474,604`) | **NONE** — roundtrip + circular dual-impl only | **NO** — secret-dep `continue` + var-time `%` (`:299-307`); non-CT FO compare (`:708`) | NO | **Non-interoperable, unvalidated, leaky** |
| **RNG / entropy** (`rng.rs`) | N/A | N/A (ChaCha20 is DRBG) | fill is CT | production = `EntropyRng` (getrandom/RDRAND), fail-closed | **Much improved vs prior; sound** |
| **Hybrid gate** (`hybrid_gate.rs`) | N/A | N/A | N/A | classical-only; PQ = `HybridIncomplete` | **No PQ defense-in-depth yet** |
| **Zeroization** | — | — | — | **absent everywhere** | **Secrets linger in memory** |

---

## 3. Findings

### F1 · ML-KEM-768 is NOT FIPS-203-interoperable (coefficient-domain packing) · **HIGH**
- **Location:** `core/src/pq_kem.rs:473-474` (the admission), `:604` (`t = A·s + e` in coeff domain), `:616/:622`
  (`byte_encode(12, &t[i]…)` / `byte_encode(12, &s[i]…)` pack coeff-domain), `:329-335` (NTT removed).
- **Evidence:** the code comment states verbatim: *"encoding is identical whether t or NTT(t) is stored, as long as
  both sides agree. We use the coefficient domain (no NTT) for correctness-by-construction."* FIPS-203 mandates that
  `ek` carries `t̂ = NTT(t)` (Alg 13/16, ByteEncode₁₂ over the **NTT-domain** vector) and `dk` carries `ŝ`. bebop
  packs the plain coefficient vectors. `A[i][j] = SampleNTT(...)` (`:454-463`) is sampled in NTT domain, but `t` is
  then formed by a schoolbook convolution and stored coefficient-domain — so the serialized bytes diverge from any
  conformant implementation.
- **Exploit / impact for a rival:** the interoperability guarantee — the entire point of standardizing on ML-KEM —
  is void. A real FIPS-203 peer (or `liboqs`, RustCrypto `ml-kem`, BoringSSL) **cannot** encapsulate to bebop's `ek`
  or decapsulate bebop's `ct`; shared secrets never agree. The "both sides agree" caveat is only true bebop-to-bebop,
  which reduces the "standard KEM" to a bespoke, unreviewed scheme. A rival marketing against this says truthfully:
  "their ML-KEM is not ML-KEM."
- **Fix:** re-derive a verified NTT (with `intt(ntt(a))==a` **and** `intt(mul_ntts(ntt(a),ntt(b)))==schoolbook`
  gates — the code even names this requirement at `:334-335`), compute and **store `t̂`/`ŝ` in NTT domain**, then
  prove conformance against NIST ACVP ML-KEM `encapDecap`/`keyGen` vectors (F2).

### F2 · ML-KEM-768 has NO external KAT — validation is self-consistency + a circular "dual-impl" · **HIGH**
- **Location:** `core/src/pq_kem.rs:897` (`dual_impl_bit_exact`), `:920` (`kem_roundtrip_and_corruption`),
  `:28-40` (comment admitting official vectors "could" be wired but are not). `core/kat/acvp/` contains only
  `key-gen/sig-gen/sig-ver.json` = **ML-DSA**; there is no `encapDecap`/ML-KEM vector file.
- **Evidence:** `kem_roundtrip` only proves `decaps(encaps(x)) == x` — self-consistency. `dual_impl_bit_exact`
  (`:819` "used as the independent implementation") re-runs the **same** coefficient-domain algorithm twice and
  compares — a **circular** test: a self-consistent-but-wrong (or trapdoored) KEM passes its own tests. The only
  real KAT touching this file is the **FIPS-202 Keccak/SHAKE** vector (`fips202_kat`, `:739`), which validates the
  hash, not the KEM.
- **Exploit / impact:** exactly the backdoor playbook the prior review named — you can alter the KEM's internal
  encoding, packing slack, or `A`-sampling and every test stays green because nothing external contradicts it. No
  assurance of correctness *or* interoperability. Self-consistency ≠ correctness.
- **Fix:** vendor NIST ACVP ML-KEM-768 `keyGen` + `encapDecap` vectors and assert byte-exact `ek/dk/ct/K`
  (the same pattern already done well for ML-DSA in `acvp_tests.rs`). Until then, stop labeling it "FIPS 203."

### F3 · Hybrid is classical-only — the (now-correct) ML-DSA leg is unwired; no PQ enforced on the wire · **HIGH**
- **Location:** `proto-cap/src/hybrid_gate.rs:68-89`; `proto-cap/src/lib.rs:12-17`; `hybrid_gate.rs:5-8`
  (PQ leg is "a TODO pending the ML-DSA pack/unpack API").
- **Evidence:** `check()` verifies `frame.verify_classical()` (real Ed25519, `:69`). For the PQ leg: `pq_sig=Some`
  → `Err(HybridIncomplete)` (can't verify yet), `pq_sig=None` under the default `ClassicalUntilPqAudit` policy
  → `Ok(())` (`:82-88`). So the operative policy accepts a frame on the **classical signature alone**; ML-DSA-65 is
  never called anywhere in a verification path (the `pq_dsa` pack/unpack exists and is ACVP-tested, but no protocol
  consumer invokes it).
- **Exploit / impact:** the product's headline property — "post-quantum-secure delivery protocol" — does not hold in
  operation. Any adversary with a CRQC (or who breaks/steals the Ed25519 leg) forges capabilities, reroutes
  deliveries, forges ledger appends. The hybrid provides **no** post-quantum defense-in-depth today.
  *(Note: the prior review's "PQ-strip is rewarded with a spoofable Ok" is partly remediated — a present-but-bogus
  `pq_sig` now yields `HybridIncomplete` rather than success — but the net effect, PQ-absent-is-accepted, is
  unchanged under the default policy.)*
- **Fix:** implement `signed_frame::{sign_pq,verify_pq}` over `pq_dsa`, make `RequireBoth` the production policy,
  and treat a missing PQ signature as rejection once identities carry ML-DSA keys.

### F4 · ML-KEM secret-dependent, variable-time `poly_mul` (timing side-channel on the secret key) · **MEDIUM**
- **Location:** `core/src/pq_kem.rs:296-327`, specifically `:299` `if a[i]==0 { continue; }`, `:304`
  `if b[j]==0 { continue; }`, and `:307/:310/:313/:324` modular reduction via `% (Q as i64)`.
- **Evidence:** `poly_mul` is a schoolbook convolution that **short-circuits on zero coefficients**. In `decaps`
  → `kpke_decrypt` the secret vector `s` is one operand, so the number of skipped iterations (and total runtime)
  depends on how many secret coefficients are zero and where. Rust's `%` on `i64` is not guaranteed constant-time.
- **Exploit / impact:** a remote/co-located timing (or power) adversary who can trigger repeated decapsulations and
  measure latency can learn the sparsity/structure of `s`, leaking secret-key information over many samples —
  progressive key recovery. The FO transform (F5) does not protect against a timing oracle in the arithmetic itself.
- **Fix:** replace the schoolbook path with a constant-time NTT (needed for F1 anyway); remove all data-dependent
  `continue`; use Montgomery/Barrett reduction (constant-time), never `%` on secrets.

### F5 · Non-constant-time Fujisaki-Okamoto ciphertext comparison + data-dependent select in `decaps` · **MEDIUM**
- **Location:** `core/src/pq_kem.rs:706-713`, esp. `:708` `if cprime == *ct { … kbar } else { … kbar2 }`.
- **Evidence:** the FO re-encryption + implicit rejection is present (good — IND-CCA2 structure, `:706-712`), but the
  comparison `cprime == *ct` uses the derived `PartialEq` on `[u8; 1088]`, which **short-circuits on the first
  differing byte** (variable-time), and the shared secret is chosen by a **data-dependent `if/else` branch** rather
  than a constant-time conditional move.
- **Exploit / impact:** this is the classic decapsulation timing oracle. The timing difference between the
  "re-encryption matches" and "mismatch" paths (and the early-exit position of the byte compare) reveals whether a
  chosen ciphertext decrypts consistently — a plaintext-checking oracle that undermines the very IND-CCA2 property
  the FO transform is there to provide.
- **Fix:** constant-time compare (`subtle`-style / the crate's own `hash.rs` CT-eq used for AEAD tags) and a
  constant-time conditional select of `kbar` vs `kbar2`; never branch on the comparison result.

### F6 · No zeroization of any secret material · **MEDIUM**
- **Location:** whole tree — `grep -niE 'zeroize|Drop for|\.zero\(\)|clear\(\)'` over `pq_dsa.rs`, `pq_kem.rs`,
  `rng.rs`, `sign.rs` returns **nothing**. Secret keys, seeds, expanded `s1/s2`, ChaCha20 DRBG state, KEM `mprime/r`
  (`pq_kem.rs:691-699`) are dropped without wiping.
- **Evidence:** no `Drop` impls, no explicit clears; `EntropyRng`/`ChaCha20Rng` keep the key in a plain `[u8;32]`.
- **Exploit / impact:** long-lived secret bytes remain in heap/stack and can be lifted from a core dump, a swapped
  page, a `/proc/<pid>/mem` read, or a post-free reuse. Lowers the bar for local/forensic key recovery.
- **Fix:** wrap secret buffers in a zero-on-drop type (volatile write + compiler fence) — a ~20-line zero-dep
  `Zeroizing<[u8;N]>` keeps the "no external deps" posture.

### F7 · KEM abandons the NTT for schoolbook O(n²) — availability/DoS + standards drift · **LOW→MED**
- **Location:** `core/src/pq_kem.rs:329-335` (NTT removed), `:296-327` (schoolbook), matrix ops `k×k`.
- **Evidence:** each `poly_mul` is 256×256 = 65 536 `i64` mul+`%` operations; a `k×k=9`-element matrix-vector
  product per keygen/encaps/decaps multiplies that cost by an order of magnitude vs the NTT's `O(n log n)`.
- **Impact:** decapsulation is far more expensive than a conformant KEM; under load this is an asymmetric CPU-DoS
  lever (cheap ciphertext → expensive decaps). It also entrenches the non-interoperable coeff-domain design (F1).
- **Fix:** the verified NTT from F1 resolves this too.

### F8 · `test_keygen` feature compiles the constant-seed keygen into `proto-cap`'s production graph · **LOW**
- **Location:** `proto-cap/Cargo.toml:11` (`features = ["std", "test_keygen"]`), `core/src/rng.rs:162`
  (`from_seed` gated on `test_keygen`), `core/Cargo.toml` feature docs ("Production code must NOT enable this").
- **Evidence:** `test_keygen` unlocks `pub` deterministic `keygen(seed)` / `ChaCha20Rng::from_seed`. `proto-cap`
  enables it as a **normal** (not dev-) dependency feature, so those predictable-key symbols are reachable in any
  build that links `proto-cap`, not just its `#[cfg(test)]` code. No production caller uses them today (the
  `[5u8;32]`/`[42u8;32]` seeds at `hybrid_gate.rs:101`, `signed_frame.rs:245+` are inside `#[cfg(test)]` modules).
- **Impact:** latent — a future refactor or a downstream crate can call `keygen([const;32])` and mint predictable
  keys with nothing to stop it. Feature-unification means it cannot be assumed off.
- **Fix:** move `test_keygen` to `[dev-dependencies]`/`[features]` used only under `cfg(test)`, or split the
  deterministic entry points behind a `#[cfg(test)]`-only helper crate.

### F9 · "Anu QRNG native default + fallback" (HEAD commit) is vaporware — overclaim · **LOW (honesty)**
- **Location:** HEAD commit `d94f013` message: *"Anu QRNG native default+fallback"*. `grep -rniE 'anu|qrng'`
  over the whole tree (rs + docs) returns **nothing**.
- **Evidence:** the real RNG is `EntropyRng` (getrandom(2) raw syscall `rng.rs:250-258`, RDRAND fallback, WASM
  `crypto.getRandomValues` `:429`) — a sound, fail-closed CSPRNG. There is no network/quantum RNG and no "fallback"
  logic anywhere. (This is *good* for security — a network QRNG default would be a third-party-trust and
  availability hazard — but the commit claims a feature that does not exist.)
- **Impact:** documentation/marketing claims outrun the code (repeat of the prior review's honesty-audit theme).
  If the "Anu QRNG default with insecure fallback" is ever actually built, re-audit it.
- **Fix:** delete the claim, or (preferably) never make a remote QRNG the default; keep `EntropyRng`.

### F10 · ML-DSA data-dependent timing in rejection loop / `decompose` / hint packing · **LOW (informational)**
- **Location:** `pq_dsa.rs:850/866/882` (`continue` = restart signing on norm-check failure), `:263` (`decompose`
  branch), `:639/:678` (hint packing branches on `h[i][j]!=0`).
- **Evidence:** these branch on values derived from the secret nonce `y`/`z`. This matches the FIPS-204 **reference**
  behavior (the rejection-sampling loop count and hint layout are the standard construction), and the field
  arithmetic itself is branchless (`montgomery_reduce:70`, `reduce32:77`, `caddq:83` use arithmetic masks).
- **Impact:** the iteration count leaks only the (weak) fact that a rejection occurred, as in every reference
  ML-DSA; not a novel break, but worth noting for a hardened deployment.
- **Fix:** optional — masked/constant-time hint packing if a power-analysis threat model applies.

---

## 4. False-greens (tests that lie) — and one green that does NOT

**Genuine green (refutes prior review) — `pq_dsa::acvp_tests` is REAL:**
- The vectors are **authentic NIST ACVP**, proven three ways: (a) `key-gen.json` contains **ML-DSA-44** (pk 1312 B),
  **ML-DSA-65** (pk 1952 B) **and ML-DSA-87** (pk 2592 B) groups — the from-scratch impl only implements ML-DSA-65
  (`K=6,L=5`), so it **physically cannot have generated** the 44/87 vectors → the file is an external export, not
  self-captured output; (b) all sizes are FIPS-204-exact (sig 3309 B); (c) vsId 42 / revision FIPS204 / testType AFT
  is the canonical NIST ACVP-Server format. The test asserts **byte-exact pk AND sk** (`acvp_tests.rs:218-227`),
  byte-exact **signature** (`:252-257`), and `verify == testPassed` (`:276-281`, taken from the JSON, never
  hand-hardcoded). Count-guards assert exactly 25/20/15 (`:301-312`) so an empty-vector vacuous pass is impossible.
  The sigVer set has both valid (3) and invalid (12) ML-DSA-65 cases, so an always-true or always-false verifier
  fails. **Conclusion: byte-exact ML-DSA cannot be faked by a broken `expand_a`; this green is earned.** Give the
  prior review credit — its finding drove a correct fix.

**Actual false-greens (ML-KEM):**
1. `pq_kem.rs:920 kem_roundtrip_and_corruption` — proves only `decaps(encaps(x))==x`; self-consistency, says nothing
   about FIPS-203 correctness or interoperability.
2. `pq_kem.rs:897 dual_impl_bit_exact` — **circular**: the "independent" impl (`:819-839`) is the same
   coefficient-domain algorithm duplicated. Two copies of the same possibly-wrong code agreeing proves nothing.
3. `pq_kem.rs:764 poly_mul_matches_schoolbook` — proves the (removed) NTT once equaled schoolbook; the shipped path
   *is* schoolbook, so this now compares schoolbook to schoolbook. It does **not** anchor to FIPS-203 bytes.
4. The `core/kat/README.md` promise ("Commit FIPS 203/204 KAT here") is **half-kept**: FIPS-204 ML-DSA vectors are
   present and asserted; **FIPS-203 ML-KEM vectors are absent** — so "ML-KEM-768 (FIPS 203)" rests on false-greens.

---

## 5. What would make it trustworthy

1. **Wire NIST ACVP ML-KEM-768 vectors** (`keyGen` + `encapDecap`) and assert byte-exact `ek/dk/ct/K`, mirroring
   the ML-DSA `acvp_tests.rs` gate. This single step exposes F1/F2 as red and forces the fix. (Highest leverage.)
2. **Re-derive a verified constant-time NTT** and store `t̂/ŝ` in NTT domain, so the KEM is FIPS-203-interoperable
   (fixes F1, F4, F7 together). Gate it with the `intt∘ntt==id` **and** `mul_ntts==schoolbook` proofs the code
   already specifies (`pq_kem.rs:334-335`).
3. **Constant-time the KEM:** remove secret-dependent `continue` (F4); replace the FO compare/select with
   constant-time equality + conditional move (F5).
4. **Wire the ML-DSA-65 leg into the hybrid gate** (`signed_frame::verify_pq`) and make `RequireBoth` the production
   policy so "post-quantum" is true on the wire, not just in the crate (F3).
5. **Zeroize** all secret buffers with a zero-dep zero-on-drop wrapper (F6).
6. **Fix the docs/commit overclaims:** drop the "Anu QRNG" claim (F9); label ML-KEM "non-conformant / experimental"
   until F1–F2 land. Move `test_keygen` out of `proto-cap`'s production feature set (F8).
7. **Keep the ML-DSA discipline as the template:** the ACVP byte-exact gate is exactly right — apply the identical
   rigor to ML-KEM before either primitive is called "FIPS."

---

## Appendix — commands run
- `cargo test -p bebop2-core --release` → `test result: ok. 160 passed; 0 failed` (incl. 60 ML-DSA-65 ACVP cases).
- `python3` structural parse of `core/kat/acvp/*.json` → three parameter sets present; ML-DSA-65 sig = 3309 B;
  sigVer testPassed distribution [3 true / 12 false] for the ML-DSA-65 group.
- `grep` provenance/CT/zeroize sweeps cited inline as `file:line`.
- Provenance of ML-DSA vectors: genuine NIST ACVP (proof: 44/87 groups the impl cannot produce). Not independently
  re-fetched from nvlpubs in this (possibly air-gapped) environment — but the 44/87-presence argument is
  self-contained and decisive.
</content>
</invoke>
