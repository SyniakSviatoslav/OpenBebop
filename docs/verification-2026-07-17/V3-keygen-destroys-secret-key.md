# V3 — `keygen_from_entropy` returns the public key as the "secret" and discards the real seed

**File:** `bebop2/core/src/sign.rs` (1036 lines, cited @ `b87b7e2`)
**Known-finding #3.** Verdict: **REPRODUCES.**
**Severity: HIGH — latent CRITICAL** (the entropy keygen path produces an unusable,
unrecoverable keypair; currently masked only because every caller signs from the *seed*).

> **Review note (3-model overlap).** Fairness caveat verified against the tree: `README.md:89`
> marks the entropy layer as **"WS-1 — fail-closed CSPRNG hardening still in flight (Wave 1)"**,
> so calling `keygen_from_entropy` "the declared production path" over-reads a self-declared
> *in-flight* path. The bug is real and reproduces; but it sits on Wave-1 work the project already
> flags as incomplete, and the only in-tree callers (rng tests, pq demo) correctly discard the
> bogus `sk`. Read the severity as "latent landmine on an in-flight path", not "shipped break".

## Claim tested

> `keygen_from_entropy` silently destroys the secret key via a scope-drop bug; scalar
> RFC-8032 `verify()` around line ~832; no `verify_batch` on `main`.

## What the code actually does

### The keygen bug reproduces (and is broader than "scope drop")

`keygen(seed)` (`sign.rs:748-762`) and its prod twin `keygen_from_seed_infallible(seed)`
(`:779-791`) both end with:

```rust
let pk = point_compress(&a_pt);
let mut sk = [0u8; 32];
sk.copy_from_slice(&pk);   //  ← sk := COPY OF THE PUBLIC KEY   (lines 759-760 / 788-789)
(pk, sk)
```

The returned "secret key" is a **byte-for-byte copy of the public key**. The actual secret —
the 32-byte `seed` — is never returned.

`keygen_from_entropy()` (`:767-775`), whose own doc says *"Production Ed25519 keygen … Replaces
the constant-seed `keygen` in all prod paths"*, draws a fresh random `seed`
(`rng::entropy_provider().fill(&mut seed)`), calls `keygen_from_seed_infallible(&seed)`, and
returns its `(pk, sk=pk)`. The local `seed` then **goes out of scope and is dropped** — it is
never returned, stored, or zeroised-with-a-copy-kept. So a production keypair minted via
`keygen_from_entropy` **has no retrievable private key**.

### Why it's currently masked (and why that makes it a landmine)

Every in-tree caller signs from the **seed**, not from the returned `sk`. The signing API is
`sign(seed, msg)` (`:795-829`) and `Delegation::sign(..., anchor_seed)` /
`SignedFrame::sign_classical(seed)` — all take the seed. Test helpers do
`let (pk, _) = keygen(&seed);` and then sign with `seed` directly. So the bogus `sk` is silently
discarded everywhere and nothing fails today.

The trap: `keygen_from_entropy` is the *only* path that generates a **fresh, non-constant**
seed — and it throws that seed away. A production caller who does the obvious
`let (pk, sk) = keygen_from_entropy()?;` and later tries to sign with `sk` will either (a) fail
to produce verifiable signatures, or (b) unknowingly derive a *different* keypair (since
`sign(sk_as_seed, ..)` clamps/derives from `sk==pk` and yields a key unrelated to `pk`). Either
way, correctness is broken, and the true secret is unrecoverable because the entropy was dropped.

### `verify()` — confirmed, sound; no `verify_batch`

`verify(pubkey, msg, sig)` at `sign.rs:832` is scalar RFC-8032 §5.1.7. It correctly rejects
non-canonical/malleable `S ≥ L` (`:841-844`) before decompressing points and checking
`S·B == R + k·A` (`:860-869`). No batch-verify exists on this branch (grep for `verify_batch`
in `sign.rs` → none). This matches the known note: the SSR-2020 mixed-order batch-forgery fix
lives only on the separate unmerged `feat/b4-crypto-groundtruth-bench` branch, **not here**. No
finding against `verify` itself.

## Plans-vs-implementation

The plan is "fail-closed production keygen from real entropy, never a constant fallback"
(`:764-766`). The entropy-drawing and fail-closed error path are implemented correctly. But the
*return contract* is broken: the function name and signature promise a `(public, secret)` pair,
and it returns `(public, public)`. The claim "replaces the constant-seed keygen in all prod
paths" is only safe **because no prod path actually consumes the returned secret** — the API is
a loaded gun with the safety currently on by luck of caller convention.

## Remediation sketch

Make the keygen functions return the actual signing material. RFC 8032's secret key is
`seed || pubkey` (64 bytes), and this crate's `sign` takes the 32-byte seed — so return
`(pk, seed)` (or a typed `SecretKey(seed)` newtype), never `(pk, pk)`. Then migrate callers off
the "sign from a loose seed" convention onto the returned secret, and add a round-trip test:
`let (pk, sk) = keygen_from_entropy()?; let sig = sign(&sk, m); assert!(verify(&pk, m, &sig));`
— which **fails today** and would pin the fix.
