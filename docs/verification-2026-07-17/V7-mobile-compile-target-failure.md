# V7 — No wired entropy provider for Android/iOS: `compile_error!` (portability gap, by-design fail-closed)

**File:** `bebop2/core/src/rng.rs` (cited @ `b87b7e2`)
**Known-finding #7 (flagged as "CRITICAL cross-repo").** Verdict: **REPRODUCES (mechanism) —
but severity CORRECTED DOWN after independent review.**
**Severity: LOW–MEDIUM** (a portability gap on not-yet-wired native targets; the fail-closed
`compile_error!` is *correct, prescribed behavior*, not a defect).

> **Review-corrected.** The first draft of this finding rated it HIGH on a "plans claim mobile
> reach" premise. The 3-model overlap reviewer showed that premise does not hold up, and I
> verified the correction against the tree. This version reflects the corrected, honest reading.
> The mechanism is real; the *severity and framing* were overstated. Recorded transparently
> rather than silently patched.

## Claim tested

> An Android/iOS compile-target failure was flagged as a CRITICAL cross-repo finding — likely a
> crate that doesn't build for a mobile target due to a raw-syscall/MMU dependency. Verify what
> it refers to and confirm current status.

## Mechanism (confirmed exactly)

`bebop2-core` supplies platform entropy with zero external crates. `entropy_provider()`:

- `#[cfg(target_arch = "wasm32")]` → `WasmCrypto` (`rng.rs:445-449`)
- `#[cfg(all(target_os = "linux"))]` → `LinuxGetrandom` (`:451-455`)
- `#[cfg(all(not(wasm32), not(linux), any(x86, x86_64)))]` → `RdRand` (`:457-465`)
- `#[cfg(all(not(wasm32), not(linux), not(any(x86, x86_64))))]` → **`compile_error!`**
  (`:467-477`).

**Android** (`target_os = "android"`, aarch64) and **iOS** (`target_os = "ios"`, aarch64) match
none of the first three arms, so both route to `compile_error!`. `bebop2-core` (and its
dependents) therefore does not compile for `aarch64-linux-android` / `aarch64-apple-ios`. The
aarch64 syscall impl (`:299-347`) is gated `target_os = "linux"`, so it is not reused for Android
even though the kernel is the same.

## Why this is NOT a HIGH defect (the corrected reading)

Three things the first draft got wrong, all verified against the tree:

1. **No plan claims native mobile reach.** A grep across `bebop2/**` + `docs/**` finds **no**
   statement claiming native Android/iOS support or "runs-everywhere/ubiquitous mobile". The
   phrase the first draft quoted ("every node can run it") is from `revocation.rs:113` and refers
   to the *anti-entropy revocation primitive*, not to platform reach — it was lifted out of
   context. The "plans-vs-implementation gap" was, in this instance, **manufactured**.
2. **The `compile_error!` is the prescribed remediation, not a bug.**
   `REMEDIATION-BLUEPRINT-2026-07-12.md §3B` states verbatim: *"A release profile without a wired
   provider **fails to compile**; production keygen returns `Err` if entropy is unavailable —
   never a constant fallback."* So refusing to compile on an unwired target is the **implemented
   §3B behavior** closing RED-TEAM §3C — i.e. a *correct safety property*, exactly as designed.
3. **The documented deployment vehicle (wasm32) IS wired.** `WasmCrypto`
   (`crypto.getRandomValues`) is present and is the one sanctioned wasm import. If a mobile build
   ships via a wasm runtime (the stated AGC-class direction), there is no gap at all. The README
   also marks the entropy layer itself as **"WS-1 — fail-closed CSPRNG hardening still in flight
   (Wave 1)"** (`README.md:89`), i.e. explicitly not-yet-complete work, not a shipped claim.

## What remains true (the LOW–MEDIUM residue)

Native aarch64 Android/iOS targets are **not yet wired** — a real portability limitation for any
future *native* mobile node, and the fix is small (widen the cfg, since Android shares the Linux
`getrandom` syscall). That is worth a tracking item, but it is a "not-yet-built native target
behind a correct fail-closed guard", not a security defect and not a falsified plan claim.

## Remediation sketch (unchanged, low priority)

Widen the aarch64 cfg to `any(target_os = "linux", target_os = "android")` for
`LinuxGetrandom`/`getrandom_syscall` and `entropy_provider()`; add an iOS arm via
`SecRandomCopyBytes` / Darwin `getentropy(3)`; add build-only CI for `aarch64-linux-android` /
`aarch64-apple-ios` so a *native* mobile target is validated when the project chooses to support
it. Until then, the README should enumerate supported targets (Linux x86_64/aarch64, wasm32) and
mark native mobile as intentionally-not-yet-wired — which largely matches what the docs already
say.
