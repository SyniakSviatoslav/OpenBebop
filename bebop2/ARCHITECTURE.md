# bebop2 — vector → tensor → wave (first-principles representation)

> Operator directive (2026-07-10): "wherever there are vectors, think about whether it is
> possible and how to replace [them] with tensors and waves." Below is the resolved design.
> It is NOT prose — it is the API contract the agents implement against.

## The physical distinction (don't store abstractions, store the irreducible object)

A `Vec<f64>` in code is almost always one of three physically-distinct things. Confusing
them is why software burns 1000× the silicon:

1. **State point** (drone pose, embedding) — genuinely a vector = coordinates in a basis.
   BUT the *natural* basis is usually spectral (eigenmodes), not canonical. Store the
   **coefficients in the eigenbasis**, not the sample list.
2. **Linear operator / tensor** (matrix, Hessian, Jacobian, covariance `P`) — a dense
   O(n²) tensor is a CRIME against the AGC envelope. Almost every operator we use is
   *local* (Laplacian, diffusion, convolution). Its eigenmodes are **waves**. Store the
   operator as its **spectrum** (O(n) eigenvalues + a few modes), never the dense tensor.
3. **Field** (spatial/temporal signal) — a sampled field IS a wave decomposition. Store
   **spectral coefficients** (Fourier/Chebyshev), not grid samples. Physics lives in
   frequency domain: heat `∂u/∂t = -λu`, waves oscillate at eigenvalue `ω`.

## Unifying primitive set (the "machine code")

Replace every dense buffer with these spectral primitives. The "tensor" becomes a spectrum;
the "vector" becomes spectral coefficients; both ARE waves.

| Old (dense) | New (spectral / wave) | Module |
|--------------|----------------------|--------|
| `Vec<f64>` state | `project(state, basis)` → coeffs; `reconstruct(coeffs, basis)` | `algebra` |
| dense matrix operator | `spectral(op)` → (eigenvalues, modes); modes = waves | `field` |
| tensor contraction | pointwise in spectrum: `propagate(spectrum, t)` = `exp(-λt)` / `exp(iωt)` | `chebyshev`, `fft` |
| VSA hypervector (dense) | `bind(a,b)` = circular convolution = **pointwise multiply in Fourier** = wave interference | `vsa` |
| Kalman covariance `P` (dense matrix) | `P` as spectrum / low-rank factors; integrate the resolvent — never form the full tensor | `kalman` |
| cost / distance tensor | resolvent of weighted adjacency = spectral; Bellman fixed-point via few spectral iterations | `field` |
| free-energy / precision (dense) | Laplacian of generative model = spectral; beliefs diffuse | `active` |

## Why this survives the AGC envelope
- O(n) storage instead of O(n²) for every operator → fits 2K core RAM per primitive.
- Operations are **pointwise multiplies** + small matrix-vector products — exactly what a
  2.048 MHz machine does. No dense matmul, no allocator thrash at hot path.
- Deterministic, no RNG/clock/network → empty wasm import section (core-RE-loop v2 gate).

## Falsifiability (every primitive ships RED+GREEN)
- `propagate` on a known Laplacian == analytic heat kernel to 1e-9.
- `bind`/`unbind` round-trip: `‖unbind(bind(a,b), a) - b‖ ≈ 0` (symmetry gap).
- spectral Kalman `P` == brute-force dense `P` to 1e-9 on a reference system.
- C8: `fexp` symmetric reduction correct for x<0 (ALREADY fixed in `lib.rs`).
- crypto: FIPS 203/204 + RFC KAT vectors pass bit-exact (committed in `kat/`).

## Carry-forward bug patterns (from fable + audit — do NOT repeat)
- **C8** fixed in `lib.rs::fexp` (negative-arg range reduction). Don't regress.
- **B4** route used LIFO `Vec` → use BinaryHeap + admissible heuristic (i→dst).
- **B8** vault keystream reuse → `rng.rs` must be per-nonce, never reused.
- **B11** hardcoded `dt` → stable corridor `dt=0.02`.
- **C2** stabilizer gate checked raw value → saturate FIRST, then gate.
- **Fable meta-fallacy**: verify PROPERTIES (empty import, named-test greps, bit-exact
  execution), never LABELS (grep symbol presence ≠ correct function).

## Middleware / proxies / transpilers → direct communication (operator directive 2026-07-10)

> "wherever there is middleware, proxies or transpilers — think and analyze whether it is
> possible to replace [them] with direct communication."

Same first-principles move as vector→wave, one layer up the stack. A middleware/proxy/transpiler
is a *relay + translation* hop between a producer and a consumer that could, in principle, talk
directly. The test: **does the hop carry real physics, or only accidental indirection?**

| Layer in old `crates/bebop` | What it is | Fundamental or accidental? | bebop2 replacement |
|------|------|------|------|
| **wasm-bindgen** (wasm target) | rust-core ↔ JS/TS translator | **Accidental** on native path. Dead weight; only needed for web build. | Native CLI calls core via raw `cdylib` C-ABI over linear memory — like AGC read core-rope directly. No bindgen shim in hot path. |
| **MCP stdio server** | proxy: agent ↔ engine over JSON-RPC | **Accidental in-process.** Exists only so external tools reach the engine. | In-tree: direct `decide()` calls. MCP stays OPTIONAL external boundary (flag-OFF), never on the execution path. |
| **`loop.ts` GUARD GATE / `guard.ts`** | middleware intercepting every command | **Mislabeled.** Not a proxy — it's a *verifier* (as-above-so-below checker). | Keep the check as a direct `apply_command_checked()` predicate, not a separate process. |
| **dual-track / advisor→kernel** | proxy: stochastic proposes, kernel decides | **Fundamental (safety).** The indirection IS the air-gap (propose-don't-execute). | Keep — it's a trust boundary, not middleware. Direct predicate call, not a transport. |
| **serde / serde_json / toml** | transpilers: struct ↔ bytes | **Accidental for kernel.** Pull 4 crates + alloc into deterministic core. | Hand-written **fixed-layout** (de)serializer in `core` — O(n) linear scan, no reflection, no alloc at hot path. Envelope = content-addressed fixed bytes (like core-rope). |
| **ratatui / crossterm** | TUI middleware over terminal | **Accidental for core; keep at edge.** Agent logic must not depend on a TUI lib. | Core emits structured state; a *thin* in-tree terminal writer (no ratatui dep) renders it. Leaf, not spine. |

### Unifying principle
> Every hop that *relays or translates without changing the physics* is accidental → delete it.
> Every hop that *verifies or enforces a boundary* (guard gate, advisor→kernel air-gap) is
> fundamental → keep, but as a **direct predicate call, not a process/transport**.

AGC precedent: no middleware between the IMU and the guidance computer. The IMU wrote directly
into fixed memory; the Executive read it directly. The only indirection that survived was LVDC's
TMR voting — and that's a *verifier* (3× compute, majority vote), not a proxy.

### Concrete bebop2 rules (enforced by reloop v2 + agent briefs)
1. Native CLI → core: **direct `cdylib` C-ABI over linear memory.** No wasm-bindgen, no JSON, no MCP in hot path.
2. Serialization: **fixed-layout direct codec** in `core`, no serde.
3. Guard / dual-track: **inline predicates** (`apply_command_checked`, `dual_track_gate`), not servers.
4. MCP / web-bindgen: **flag-OFF external-only boundary**, never on deterministic execution path.
5. TUI: **leaf renderer**, in-tree, no ratatui/crossterm dependency in the core crate.

### Falsifiability
- `bebop2-core` wasm artifact: **empty import section** (no proxy/transport reachable) AND **zero
  `extern` calls to any transport** — verified by reloop v2.
- Benchmark: in-tree `decide()` call latency vs old MCP-stdio round-trip must show the middleware
  was pure overhead (RED+GREEN: removing it speeds up, not breaks).

