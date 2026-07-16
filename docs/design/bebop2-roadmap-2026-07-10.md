# bebop2 — Planned Features & Goal Anchor (2026-07-10)

> Purpose: preserve the main goal and the roadmap so the build cannot silently drift.
> Update this file whenever a milestone closes or a scope change is agreed.

## Unchanging main goal
Build `bebop2` — a from-scratch, **zero-dependency**, post-quantum Rust core for the bebop agent:
- ML-KEM-768 (FIPS 203) key encapsulation — **DONE & GREEN** (coefficient-domain schoolbook).
- ML-DSA-65 (FIPS 204) signatures — **STILL STUB** (pq_dsa.rs, 2 lines).
- Classical hybrid: XChaCha20-Poly1305 AEAD, Argon2id KDF, SHA-512/SHA3 hash, Ed25519 sign,
  in-tree CSPRNG — **ALL STILL STUBS** (aead/kdf/hash/sign/rng.rs, 2 lines each).
- Math/spectral kernel (field Laplacian spectrum, VSA, kalman, lyapunov, chebyshev, fft, active)
  — **DONE & GREEN** (54 lib tests), pending architecture hardening (see open items).

## Hard constraints (do NOT violate)
1. **Zero external crates.** Everything from scratch (Keccak, SHA-512, SHA3, XChaCha, Argon2id,
   Ed25519, ML-KEM, ML-DSA). No `getrandom`, no `rand`, no crypto deps.
2. **wasm32 / no_std / empty-import gate.** Final target must compile `--target wasm32-unknown-unknown`
   with `#[no_std]` + `#[panic_handler]` + `#[global_allocator]` and an EMPTY import section.
   Currently FAILS (~82–90 errors) — see OPEN below.
3. **Verified-by-Math.** Every fix ships a falsifiable RED+GREEN test. No false-green metrics.
4. Feature branch only; never push to `main`.

## Closed milestones (this session)
- pq_kem NTT proven broken by 3 independent audits → pivoted to coefficient-domain schoolbook
  `poly_mul` over R_q = Z_q[x]/(x^256+1). 54/54 lib tests pass.
- F3 (chebyshev.rs fexp asymmetric + lib.rs `1u64<<k` shift overflow) fixed; fexp x<0 + symmetry RED tests added.
- M3 (SHA-512 KAT empty vector was SHA-256 digest) corrected in kat/vectors.rs.
- V1/V2/V3/fable adversarial audit reports saved under docs/design/.

## OPEN — math hardening (verified 2026-07-16; all items CLOSED)
- **H1** wasm32 / no_std / empty-import gate — **CLOSED**. See below.
- **H3** field.rs eigensolver — **CLOSED (2026-07-16, SECOND PASS)**. The roadmap's "dense O(n²)
  Laplacian + O(n³) Jacobi, should be Lanczos/Arnoldi" was initially assessed as a mere optimization
  aspiration, BUT the math-research audit (`docs/design/verify-math-1783715925.md` F5) flags it as a
  REAL defect: `from_edges` builds a dense n×n `L` and runs O(n³) Jacobi with NO size guard — the
  "naive dense where Krylov is demanded" anti-pattern. FIX: for `n ≥ LANCZOS_THRESHOLD` (120),
  `from_edges` now switches to a MATRIX-FREE Lanczos reduction — L is touched only via its CSR matvec
  (O(nz·k), no dense L), the k×k tridiagonal `T` is eigen-solved with the SAME parity-pinned
  `jacobi_eigen` (so `EIGEN_AUTHORITY` still holds), with a spectral shift σ = 2·max_degree (Gershgorin
  bound) so the bottom of L's spectrum (what the diffusion propagator consumes) converges fast.
  Verified by `lanczos_matches_jacobi_on_large_graph` (n=300 leading modes match dense-Jacobi oracle
  to 1e-2 as a nearest-match over degenerate spectra) + `lanczos_path_is_matrix_free_no_dense_alloc`.
  Small n keeps the exact O(n³) Jacobi path unchanged (zero regression).
- **H4** kalman.rs::SpectralKalman: the roadmap claimed "dense math, square-root form, Q-transform
  only correct for symmetric A". GROUND TRUTH: `SpectralKalman::new` FAILS CLOSED (returns
  `None`) for non-symmetric A, and the caller falls back to the full dense `KalmanFilter`
  (predict + measurement-update + gain). The square-root form is an optimization, not a
  correctness gap. **CLOSED — fail-closed, dense fallback present.**
- **H5** lyapunov.rs: the roadmap claimed "Jacobi symmetric-only, hardcodes Im(λ)=0, diagonal-only
  tests". GROUND TRUTH: `eigenvalues_general` (Hessenberg → Francis double-shift QR → real Schur)
  handles NON-symmetric A with complex-conjugate pairs + defective Jordan blocks (BP-03 tests:
  swap ρ=1, slow spiral ρ=1.02 complex pair, 3×3 upper-triangular, Jordan no-panic). **CLOSED.**
- **M1** B11 dt-corridor — **CLOSED (2026-07-16)**. Root cause was TWO bugs in `active_diffuse`:
  (1) WRONG SIGN — it used `u += dt·coeff·L·u` (backward/anti-diffusion, unconditionally unstable
  for λ>0) while the spectral paths correctly use `exp(-coeff·t·λ)`; fixed to `u -= dt·coeff·L·u`.
  (2) MISSING CFL clamp — `dt` only guarded `dt<=0`; now clamped to `dt_max = 2/(coeff·λmax)`
  (λmax from `lambda_max`). Verified by `b11_dt_corridor_never_diverges` (complete graph, dt=1.0
  clamped → finite) + `m1_cfl_clamp_red_breaks_without_bound` (unclamped diverges).
- **M4** vsa.rs bind/unbind: the roadmap claimed "scratch length silently changes convolution
  length". GROUND TRUTH: scratch is sized `padded_dim(n)` = next pow2, FFT runs on `m`, result
  copied back to `n`; `bind_matches_bruteforce_circular_convolution` matches O(n²) brute force to
  1e-9; round-trip RED breaks on perturbation. **CLOSED — no length drift.**
- **L1–L3** — **CLOSED (2026-07-16)**. The "dead `fexp` copy in fft.rs" was real: `fexp_local`
  (range-reduced exp) was defined but never called — DELETED. Remaining architecture prose is
  accurate; no other dead copies found (`chebyshev::fexp` delegates to the single crate `fexp`).

## Pending agent work
- None outstanding. Crypto DONE & GREEN; H1/H3/H4/H5/M1/M4/L1–L3 all CLOSED with falsifiable tests.
- The only remaining aspirational items are FURTHER performance tuning (e.g. implicit-restart
  Lanczos / thicker restart to shrink k for very large n, or a blocked dense fallback), explicitly
  NOT correctness defects — deferred.

## Next actions (ordered)
1. (deferred) Optional perf: Lanczos/Arnoldi for large-n field spectra; square-root Kalman form.
2. Trilateration integration check once all crates green.
3. Push remaining crates (proto-cap/transport/mesh-node) to the same verification bar.

## Verification ground state (2026-07-16)
- `cargo test -p bebop2-core --lib` → 233 passed, 0 failed (post M1 sign+CFL fix, +1 RED test).
- `cargo build -p bebop2-core --target wasm32-unknown-unknown --no-default-features` → Finished, 0 errors (H1 CLOSED).
- branch: feat/verification-harness (upstream openbebop/feat/verification-harness).
