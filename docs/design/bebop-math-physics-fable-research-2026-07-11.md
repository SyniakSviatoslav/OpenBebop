# DEEPEST FABLE RESEARCH REPORT — math/physics foundations → bebop protocol

Synthesized from two independent deep-research agents (deleg_014e3c63 = PRIMARY-source math
verification of the pasted notes; deleg_91222529 = code-surface mapping + operational RED-lines).
Research only; no code changed except README/AGENTS test-count fix (77→82, 371→376) to pass the
doc-claim gate. All claims below cite PRIMARY sources.

═══════════════════════════════════════════════════════════════════
PART A — FORMULA AUDIT (delegate 014e3c63, verify-by-math)
═══════════════════════════════════════════════════════════════════

FABRICATED / WRONG (reject or correct):
- "Legendre transform" ∫₋₁¹ Pₙ(x)f(x)dx  — MISLABELED + missing normalization. This is a
  Legendre–Fourier spectral coefficient, NOT the Legendre transform (conjugate-variable sup_x(px−f(x))).
  Correct coefficient: aₙ = (2n+1)/2 ∫₋₁¹ Pₙ(x)f(x)dx  (orthogonality 2/(2n+1)δ_mn;
  Arfken&Weber §12). Rodrigues' Pₙ=1/(2ⁿn!)dⁿ/dxⁿ(x²−1)ⁿ is CORRECT.
- Fractional-derivative identity Σ(-1)ⁿ⁻¹[D_{1/2}(n²)]²/(n⁵C(2n,n)) = 128 ln²φ/(9π)  — FABRICATED.
  Zero primary/secondary hits; D_{1/2}(n²) undefined; "rizzy_aka_mathbae" not citable. REJECT.
- Contour-integral "network stability"  — POETRY. Cauchy-Riemann/Residue/Max-Mod/Liouville/FTA/
  Riemann-Mapping justify Laplace/Fourier signal analysis, do NOT evaluate graph stability.

CORRECT (genuine, correctly transcribed):
- Padovan P(n)=P(n−2)+P(n−3), P(0)=P(1)=P(2)=1  — OEIS A000931 (offset variant; Stewart 1996).
- Emden–Chandrasekhar (1/ξ²)d/dξ(ξ²dψ/dξ)=e^{−ψ)  — the ISOTHERMAL Lane–Emden equation
  (Chandrasekhar 1939, Ch.IV). Genuine; call it isothermal Lane–Emden, not generic polytropic.
- Wave ∂²u/∂t²=c²∇²u; vorticity ω=∇×u; Fick J=−D∇ψ; redshift z=(λ_obs−λ_emit)/λ_emit;
  u-sub/IBP/Riemann sums  — all standard, correct.
- Convergence "via log kernels" (ln²2, ln²3)  — UNSUBSTANTIATED, no explicit identity given.

PER-CONCEPT VERDICT (a=genuine math, b=applicable to protocol, c=over-claimed analogy):
- Spectral theorem / λ₂ Fiedler  → (a)(b) SOLID. Fiedler 1973: λ₂=0 iff disconnected; small λ₂
  ⇒ near-split. Legit graph-partitioning tool.
- Chebyshev/Legendre spectral  → (a)(b) legit bounded-interval approximation (Trefethen).
- Fick diffusion  → (a)(b) load-balancing as mean-field diffusion. Applicable.
- Wave eq → info wavefront  → (a)(b) WITH CAVEAT: explicit Euler injects energy; MUST use
  symplectic velocity-Verlet. Loose but defensible as hyperbolic transport.
- Vorticity→courier loops  → (a)(c) curl is 3D; graph loops detected via cycle basis/girth, not fluid.
- Emden→"demand black holes"  → (a)(c) isothermal collapse is equilibrium; demand isn't self-gravitating.
- Redshift→trust-decay  → (a)(c) rename of staleness; use max-age/TTL. Poetry.
- TDA (filtration→barcode)  → (a)(b) Carlsson 2009; noise-robust clustering. Applicable.
- Function classification→weight-type  → (a)(c) taxonomy only; no theorem forces log/exp for growth.
- Nyquist  → (a)(b) only for continuous control loops; NOT a graph-stability primitive.
- Noether  → (a)(c) applies only with conserved symmetry; mostly narrative here.
- Catalan  → (a)(b) marginal: Dyck-path routing / balanced aggregation tree counts.
- Fock space  → (a)(b formalism)(c physics) multi-agent tensor-product state.
- Cauchy–Schwarz  → (a)(b) bounds node-similarity/correlation. Real.
- Platonic / spherical harmonics  → (a)(c)/(a)(b) angular eigenbasis for directional coverage.
- Padovan temporal→TTL  → (a)(b) legit APERIODIC schedule (avoids sync resonance); design pattern,
  not physics. Current bebop memory.rs uses hash-mod-7 tick, NOT Padovan (would need KAT).
- Resource-exhaustion attacks (fork bomb / tail /dev/zero / dd)  → (a)(b) genuine Linux threat
  model for node hardening. Real engineering.

═══════════════════════════════════════════════════════════════════
PART B — CODE SURFACE + FALSIFIABLE KATs + RED-LINES (delegate 91222529)
═══════════════════════════════════════════════════════════════════

VERIFIED math → bebop/rust-core file:line (all deterministic, RED+GREEN tested):
- Laplacian eigendecomp / graph health: field.rs from_edges (CSR L=D−A) :45, jacobi_eigen :257;
  zero-mode test spectrum_has_zero_mode_for_connected :361.
- λ₂ algebraic connectivity (rupture): chebyshev.rs lambda_max :63; propagate_spectral eigen-decay.
- Chebyshev/Legendre spectral propagator: chebyshev.rs spectral_propagate :111;
  geometry_field.rs legendre :172.
- Kalman: kalman.rs SpectralKalman :130; field.rs field_kalman :174; analytics.rs kalman1d_step/
  kalman_anomaly :7,:18. KATs: kalman_p_matches_dense_oracle 1e-9 :277; kalman_red_breaks :301;
  kalman_converges_to_constant_signal field.rs:332; kalman_anomaly 10σ analytics.rs:41.
- Lyapunov: lyapunov.rs stability_margin/spectral_radius :71; stabilizer.rs stabilize_step :42,:114.
  KAT unstable_system_has_positive_margin :109.
- NTT-class FFT: fft.rs radix-2 :122, circulant_eigenvalues; KAT fft_matches_dft_oracle 1e-12 :242,
  fft_roundtrip_identity :268. (Lattice PQ ML-KEM/ML-DSA lives in vault.rs, NOT fft.rs.)
- Cauchy–Schwarz cosine bound: knowledge.rs cosine :62; KAT recall_excludes_noise_floor :174.
- Nyquist: geometry_field.rs nyquist_unstable :262; KAT nyquist_stable_vs_unstable :353.
- Platonic Euler V−E+F=2: geometry_field.rs :21,:124; spherical_harmonic :220; node_harmonic_field :238.
- Graph cycles/divergence: wavefield.rs wave_probe :239; field.rs limit_cycle_unstable :211.
- Padovan: memory.rs tick hash-mod-7 :60 (supersedes Padovan; not implemented).

FALSIFIABLE KATs (each goes RED when wrong — Verified-by-Math):
- Spectral: spectrum_has_zero_mode_for_connected (λ₀≈0 connected); matvec_f32_laplacian_zero_row_sum
  (L·1=0) :382.
- Chebyshev: propagator_matches_old_oracle_mass (heat kernel mass≈1) :205; propagator_red_breaks_on_
  coeff_change :275.
- Lyapunov: stabilize_step returns 0 when V̇>0 (freeze).
- Field veto: redline_task_is_vetoed ("rotate secrets"⇒override) field.rs:263; fail_closed_on_sim_
  degradation :303.

OPERATIONAL RED-LINES (decentralized agent protocol — operational, NOT crypto-core auth/money):
- Fork bomb → process/recursion exhaustion. mcp.rs serve() :125 loops on stdin; run_multipilot/
  batch_dispatch multipilot.rs:113 fan N pilots; recall_graph BFS knowledge.rs:103; GOAP BFS
  analytics.rs:77. GUARD: ulimit -u + cgroups pids.max per agent; cap n pilots/batches in
  run_multipilot/batch_dispatch; bound BFS depth; enforce at call_tool mcp.rs:215 — reject n/batches
  above constant.
- RAM saturation (tail /dev/zero) → OOM. field_eval field.rs:26 Vec<f64> by node count; serve holds
  persistent LivingMemory+AuditLog mcp.rs:129. GUARD: cgroups memory.max; cap LivingMemory node
  count; reject oversized JSON args in handle mcp.rs:155 (args.to_string().len() limit).
- Storage flooding (dd) → disk exhaustion. vault.rs fs::write :273; audit.rs append. GUARD: disk
  quota per agent; cap audit-log entries; sandbox disk-OFF default (currently network-OFF mcp.rs:90),
  fail-closed.
- FLAGGED triggerable tools: sandbox (cmd exec), recon, harvest, wave_probe — accept unbounded
  inputs → enforce input-size limits + cgroups at call_tool entry. All fail-closed by design.

REAL ENGINEERING vs POETRY (honesty):
REAL (implemented+tested): eigen-decomp graph health, Kalman, Chebyshev/Legendre spectral, NTT-class
FFT, Cauchy–Schwarz cosine, Lyapunov, Nyquist, Platonic Euler, spherical-harmonic field.
POETRY (analogy only, must not be cited as implemented physics): Emden–Chandrasekhar "demand black
holes" (no gravity sim), redshift "trust coefficient" (staleness rename), vorticity "courier loops"
(topological cycle finder mislabeled), Fock space / Noether "stabilized" (stabilizer.rs:668 comment,
no theorem computed), Catalan (no routing combinatorics).

═══════════════════════════════════════════════════════════════════
PART C — NET HONEST VERDICT
═══════════════════════════════════════════════════════════════════
Usable for the protocol (real + applicable): Fiedler λ₂, Chebyshev/Legendre spectral decomposition,
Fick diffusion (load balancing), TDA barcodes, Riemann/IBP integration metrics, Cauchy–Schwarz bounds,
spherical harmonics, Padovan aperiodic scheduling, resource-exhaustion threat model. Wave equation
ONLY with symplectic velocity-Verlet (no explicit Euler).

Reject: fabricated fractional-derivative identity; mislabeled "Legendre transform" (fix normalization);
contour-integral "network stability" poetry; over-claimed Emden/redshift/vorticity/noether/fock/catalan
analogies.

Repo's real math-backed core = graph spectral dynamics + Kalman + Lyapunov + Chebyshev/FFT, all with
live RED+GREEN KATs. The astrophysics/fock/catalan/noether material from the pastes is the analogy
layer — fine as narrative, must not be cited as implemented engineering.
