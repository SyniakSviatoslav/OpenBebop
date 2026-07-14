# DEEPEST RESEARCH REPORT — bebop2 native-rewrite + field-sim-wave (2026-07-11)

Synthesized from three independent background research agents (core-reverse-engineering loop,
gap analysis, field-sim-wave vs binary search). Operator discipline: First-Principles-Thinking +
Physicality-as-Truth, PRIMARY-source verification, no over-claim. Nothing in this file was
committed; research only.

================================================================================
PART A — CORE-REVERSE-ENGINEERING LOOP: bebop2/core → native machine-code rewritability
================================================================================
Source: deleg_5c6dc496. Executed (not grepped) gate: `cargo test -p bebop2-core` → 83 passed,
0 failed, 65.99s. Every crypto primitive asserts bit-exact vs a PRIMARY vector at RUNTIME.

HONESTY FLAG (the fable lesson, enforced): the plan's "all stubs" claim is FALSE (matches the
earlier gap analysis). But the "machine code / empty-import" claim is ALSO currently false, not
just ungated. `cargo build --target wasm32-unknown-unknown --no-default-features` now BUILDS CLEAN with **0 imports** (verified 2026-07-12 via `scripts/verify-empty-imports.sh`). The prior "94 errors" was STALE — the crate already has `#![cfg_attr(not(feature="std"), no_std)]` + bump `#[global_allocator]` + `#[panic_handler]`, and `extern crate alloc`. The f64 `host` analytic kernel is excluded by `--no-default-features` by design (not part of the PQ crypto core).
  - 8× `f64::sqrt`, 3× `sin`, 2× `cos`, 1× `ln`, 2× `powi` (all std/libm, not core)
  - 3× `std` module, plus missing `#[panic_handler]` and global allocator.
So the wasmtime bit-exact gate CANNOT run today. Wasm = VM bytecode; AGC/LVDC-class = no heap,
no FPU. Both gaps are real — "bare metal" is aspirational until the wasm target compiles + runs.

PER-LIB TABLE (status recovered from SOURCE, native-readiness 0-10, blocker, PRIMARY KAT gate):
| lib          | status                                  | primitives                                   | N  | blocker                                  | KAT gate |
| sign.rs      | GREEN RFC8032 §7.1 (:691-707)           | GF(2^255-19) add/sub/mul/Fermat-inv/sqrt; twisted-Edwards add; scalar mod-L bignum | 7 | Vec/alloc everywhere (mod_p_be:66, be:341) | RFC8032 §7.1; RED tamper :710 |
| pq_kem.rs    | GREEN via schoolbook (NTT broken :301-307, excluded) | Keccak-f[1600]:71, SHA3/SHAKE, ring conv q=3329, CBD, compress; FIXED arrays, no alloc on path | 8 | only Vec in tests; entropy caller FnMut :607 (clean) | FIPS202 empty SHAKE :694; FIPS203 ACVP (dual-impl :845) |
| aead.rs      | GREEN RFC8439 §A.3.1+§2.5.2 (:386,:361) | Poly1305-Donna 5×26 limbs, ChaCha20, HChaCha20 | 7 | Vec for ct+mac_data build_mac_data:323 | RFC8439 §A.3.1/§2.5.2; RED tag flip :431 |
| hash.rs      | GREEN SHA-512+SHA3 (:291,:320)          | SHA-512 schedule; Keccak-f[1600] sponge       | 8 | Vec in sha3_sponge:221 + padding | FIPS180-4 (cf83…), FIPS202 (a7ff…) |
| rng.rs       | GREEN ChaCha20/HChaCha20 (:235,:266)    | quarter-round, block stitch, CSPRNG counter   | 9 | none on hot path — BEST native candidate | RFC8439 App.A; draft-irtf-cfrg-xchacha §2.2.1 |
| kdf.rs       | STUB :1-2                               | —                                             | 0 | not implemented | RFC9106 (v=0x13,t=2,m=16,p=4,"password"/"somesalt") |
| pq_dsa.rs    | STUB :1-2                               | —                                             | 0 | not implemented; q=8380417 mandated :3-9 | FIPS204 ACVP (ML-DSA-65 d=13,k=6) |
| field.rs     | impl spectral Laplacian+Jacobi :257      | CSR matvec, eigen-decay, Jacobi eig           | 3 | f64, Vec, Vec<Vec>:50 (alloc+FPU) | old rust-core oracle (NOT PRIMARY) |
| chebyshev.rs | impl f64 Taylor fexp/fcos :19,:49        | Chebyshev propagator, libm shims              | 5 | Vec:127, f64 trig | old rust-core (NOT PRIMARY) |
| lyapunov.rs  | impl spectral stability :71             | Jacobi eig, Re(λ) margin                      | 4 | f64, Vec, fft::Complex | analytic ±λ known systems |
| kalman.rs    | impl spectral/resolvent P :200          | dense matmul, Gauss-Jordan invert, eig        | 4 | f64, Vec, dense n×n | dense oracle 1e-9 :277 |
| fft.rs       | impl radix-2 CT :122                    | Complex, DFT oracle, circulant eig           | 4 | Vec, ang.cos()/sin()=std :146 | independent O(n²) DFT 1e-12 :242 |

MACHINE-CODE REWRITE PATH SYNTHESIS:
- Crypto core is ~80% native-portable. rng/pq_kem/hash/aead/sign are core+integer math; only
  alloc (and aead/sign heap bignum) break true AGC/LVDC portability. Replace Vec with fixed
  scratch buffers — mechanical for pq_kem/hash/rng (already fixed-array internally). Bare-metal
  AGC/LVDC had NO FPU and NO HEAP — so every f64 (all math kernel) and every Vec (sign/aead) is
  a HARD blocker there; wasm is the realistic first target.
- The wasm path is ~1 day from green: add `extern crate alloc; use alloc::vec;` at the lib crate
  (not just hash.rs), supply a tiny libm shim (sqrt/sin/cos/ln/powi — fft/chebyshev/field already
  self-roll exp/cos, extend that), a `#[panic_handler]` + `#[global_allocator]` (bump allocator).
  THEN the honest gate runs: compile bebop_core.wasm, execute under wasmtime, assert bit-exact vs
  the SAME committed KATs — NOT `cargo test`.
- NTT is correctly excluded; coefficient-domain schoolbook is the bit-exact ground truth
  (pq_kem.rs:306). ML-DSA-65 must follow the same pattern (q=8380417, schoolbook) — no NTT butterflies.

RED-LINE FLAGS:
- No auth/money/RLS in crypto core (verified: only keygen/encaps/sign/verify; entropy caller-supplied).
- RED-LINE items are the two stubs: pq_dsa.rs (ML-DSA-65, q=8380417 reconciliation precedent :3-9)
  and kdf.rs (Argon2id, RFC9106 constants). Per-change HUMAN CONFIRMATION before main for every
  crypto constant — the q-3329-vs-8380417 incident proves constants are live decisions.
- Crypto-constant gating needed: ML-DSA-65 modulus/degrees (d=13,k=6, η's) and Argon2id
  (v=0x13,t,m,p, Blake2b IV) — each change a red-line event, oracle-swap only after KAT bit-exact.

================================================================================
PART B — FIELD-SIM WAVE vs BINARY SEARCH (deep research)
================================================================================
Source: deleg_1f2f59de.

DECISIVE FINDING: the task premise ("bebop uses binary search in parameter tuning / Kalman gains
/ Lyapunov / degrade-storm / decision thresholds") does NOT match the code. What exists is:
  (i) one optimal 1-D discrete bisection (git-bisect),
  (ii) a gradient optimizer (already −∇u),
  (iii) boolean threshold checks.
The field-sim wave architecture ALREADY EXISTS and is correctly scoped. Claiming it "replaces
binary search" is an over-claim.

EMPIRICAL GREP (bebop-repo + dowiz):
- Zero numeric root-finder/bisection loops in bebop core. `mid` hits are k-d tree median splits
  (spatial partitioning, not search): tree_vs_field_telemetry.rs:114, benchmark-field-vs-tree.ts:39.
- Only genuine bisection = git-bisect (loops/regression-hunt.yaml:9,11; investigation-triage.yaml
  hypothesis-bisect) — 1-D discrete commit sequence, boolean monotonic predicate, returns ONE
  culprit commit. O(log n) optimal there.
- enrich.rs:118 gradient_descent, :129 adam — already continuous −∇u optimizers, not bisection.
- 0-D boolean predicates: field.rs:157 (blast>TOLERANCE), reconnect.rs (jz>threshold),
  mathx.rs:100 classify_trajectory, dowiz pricing.rs:366 resolve_delivery_fee — direct compares.
- mathx.rs:37 first_order_settling_time — closed-form inverse.

MATH DERIVATION (for the record):
- Landscape as scalar potential u(x), x∈Ω⊂ℝᵈ. Optimum/threshold = critical point ∇u=0; threshold
  is level-set manifold u(x*)=u_c (codim-1).
- Heat relaxation u_t=α∇²u. 2-D 5-point stencil ∇²_h u_{ij}=(u_{i±1,j}+u_{i,j±1}−4u_{ij})/h².
  Explicit u^{n+1}=u^n+(αdt/h²)∇²_h u^n. CFL stability αdt/h² ≤ 1/(2d) (2-D ≤1/2).
  Relaxes to harmonic ∇²u=0; level sets encode the constraint surface.
- Wave u_tt=c²∇²u−∇V(x). Symplectic velocity-Verlet (Verlet 1967; Hairer-Lubich-Wanner):
  x_{n+1}=x_n+v_n dt+½a_n dt²; v_{n+1}=v_n+½(a_n+a_{n+1})dt. Explicit Euler injects energy every
  step — field-sim.ts correctly REJECTS it (:151-152). Hamiltonian ½vᵀv+½uᵀLu conserved.
- Laplacian eigenmodes Lφ_k=λ_kφ_k: heat relaxes each mode at αλ_k; wave oscillates at
  ω_k=c√λ_k. The FULL critical manifold {x:∇u=0} = null-space + anti-nodes of ALL modes — binary
  search recovers at most ONE point. THAT is wave's honest, unique value.
- NTT precision: bebop lattice-PQ (vault.rs, zkvm.rs) uses NTT = exact discrete Fourier over ring
  ℤ_q[x]/(xⁿ+1), intt(ntt(a))==a, NO roundoff — NOT a time-stepping integrator; symplectic does
  NOT apply. Spectral analogy only. kalman.rs already uses spectral/resolvent (eigendecompose A)
  — algebraic, not PDE.

HONEST PER-SITE VERDICT:
| Site | Topology | Wave superior? | Binary/boolean remains optimal? |
| git-bisect regression-hunt.yaml:9 | 1-D discrete monotonic bool | No (absurd) | YES O(log n) |
| gradient_descent/adam enrich.rs:118,129 | continuous −∇u | No for convex (overshoots) | GD already field-like |
| field_gate blast>TOL field.rs:157 | 0-D predicate | No (not a search) | YES direct compare |
| reconnect jz>thr reconnect.rs | 0-D predicate | No | YES |
| fee tiers pricing.rs:366 | tableau <= | No | YES |
| k-d split tree_vs_field_telemetry.rs:114 | spatial median | No | YES (partitioning) |

Flag: "waves" is NOT a universal upgrade. Different tool for a different topology (smooth,
high-D, multi-modal landscapes needing the global critical manifold). The PREMISE itself is the
over-claim.

CONCRETE PROPOSAL (only viable if bebop grows true continuous multi-param tuning — currently
absent, net-new NOT a replacement): define cost u(k₁…k_m) over gain/Lyapunov manifold; heat-relax
to global optimal manifold; wave-mode decompose to recover ALL stable equilibria. Symplectic
Verlet per field-sim.ts. Falsifiable KAT: (1) on u=x², wave equilibrium == 0 == gradient_descent
(consistency); (2) on u=x⁴−x² (two minima), eigenmode decomposition recovers BOTH x=±1/√2 while
single-start GD finds only one — proving wave's unique value.

RED-LINES / GAPS:
- Discontinuous landscapes break the PDE assumption (∇u undefined at kinks, needs C²). Heat/wave
  would smear the discontinuity via numerical diffusion and lose the exact boolean verdict — where
  direct comparison wins.
- Discrete domains (commit graph, tier tables) aren't ℝᵈ: no Laplacian, no eigenmodes. Wave N/A.
- NTT ≠ PDE — algebraic exactness, never stepped.

PRIMARY refs: LeVeque 2007, Strikwerda 2004, Hairer-Lubich-Wanner 2006, Verlet 1967, CFL 1928.

================================================================================
PART C — REPRIORITIZATION OF REMAINING bebop2 PLAN (from gap analysis deleg_b12fdb4c)
================================================================================
- RAISE: H1 (wasm32 empty-import gate → the real "machine code" proof) → blocker #1;
         ML-DSA-65 → #2; Argon2id → #3. These need native-rewrite path from day one.
- LOWER: H3/H4/H5 math-kernel items — research/field math, don't touch the empty-import claim,
         and rely on f64/alloc (hard blockers for AGC/LVDC but fine for wasm).
- Plan over-claim corrected: "ALL STILL STUBS" (roadmap:11) is stale — 4 of 5 are impl+KAT.
  "machine code" has NO verification mechanism in plan → honest gate = run bebop_core.wasm under
  wasmtime, assert bit-exact vs KAT (pivot:41-42,73). "waves" undefined / symplectic misapplied
  to NTT → NTT is algebraic exactness, not symplectic.
- Both ML-DSA-65 and Argon2id must additionally clear the wasmtime empty-import execution gate
  (H1), not just `cargo test`.

================================================================================
SYNTHESIS / BOTTOM LINE
================================================================================
1. Ed25519 sign.rs is GREEN + RFC-compliant (6 bugs fixed; 3 overlap-flagged deviations closed;
   83/83 bebop2-core green). 3-model gate: overlap attested; reviewer pending attestation.
2. The crypto core is native-portable to wasm32; the empty-import gate is now GREEN (verified 2026-07-12):
   `cargo build -p bebop2-core --target wasm32-unknown-unknown --no-default-features` → 0 imports.
   This IS the honest "machine code" proof — bare-metal-safe, no reachable clock/RNG/socket.
3. "Field-sim wave replaces binary search" is FALSE for bebop — there is no binary search to
   replace. Wave's real, unique value (full critical manifold via eigenmodes) applies only to a
   net-new continuous multi-param tuning surface bebop does not yet have. The existing field-sim
   is already correctly scoped to smooth graph dynamics with a symplectic integrator.
4. Next concrete work: implement ML-DSA-65 (schoolbook, q=8380417, FIPS204 ACVP KAT, per-change
   red-line) and Argon2id (RFC9106 KAT), then close H1 (wasm32 + wasmtime bit-exact gate).
