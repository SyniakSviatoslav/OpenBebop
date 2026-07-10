# optical — reverse-engineering notes

## Sources (reverse-engineered)
- **SVETlANNa** (PyTorch diffractive neural network library, CompPhysLab) — **real forward pass
  run** in `/root/bebop-repo/.venv2` (torch 2.13 + svetlanna 2.0.1). Captured REAL stdout:
  - `INPUT  shape (64,64) complex64, Σ|E| = 144.0`
  - `OUTPUT shape (64,64), Σ|E|² = 143.99995 ≈ 144`  → power conserved by passive mask
  - `ThinLens TF[0,0] = (1 − 2.9e-6j) ≈ exp(-i·k·r²/2f)` at r=0  ✓
- **Meep** (MIT FDTD) — not installed (heavy C++/MPI); equations extracted from docs. FDTD solves
  the same Maxwell wave equation the paraxial FFT approximates.

## Exact physics (verified against SVETlANNa source + real run)
- A thin lens is a phase mask: `T(r) = exp(-i·k·r²/(2f))`, `k = 2π/λ` (SVETlANNa
  `ThinLens.transmission_function`). At r=0, T = 1 (captured ~1−2.9e-6j from float noise).
- Free-space propagation (angular-spectrum / ASM): `FFT2(incident) · exp(i·z·k_z)` where
  `k_z = sqrt(k² − k_x² − k_y²)` (SVETlANNa `FreeSpace._propagate_by_asm`). A lens then performs
  a 2D Fourier transform of the field.
- **Optical compute primitive**: `out = FFT2D{ t(x,y) · x }` — a mask `t` times the input field,
  followed by the lens Fourier transform. That is matrix-multiply-by-diffraction.

## Equations implemented in optic.ts
```
fft1d(a, sign=+1):  X_k = Σ_n a_n · exp(-i·2π·k·n/N)      # forward = numpy.fft.fft, SVETlANNA
fft2d(A):           rows → fft1d(+1), cols → fft1d(+1)
opticalMatmul(x, mask):
    t  = mask.transmission            # complex [re,im], passive ⇒ |t|<=1
    fm = elementwise t·x              # mask the field
    out = fft2d(fm)                   # lens → Fourier transform
```
Physical-law enforcement (passive optical element):
- `|t(x,y)| > 1`  → **rejected** (a passive element cannot amplify light; gain needs a pump).
- non-finite (NaN/Inf) transmission → **rejected**.
- Pure phase masks (e.g. thin lens at phase π → t = [-1, 0], |t|=1) are **accepted** — valid passive.

## Parseval note (why the test pins N²·input power)
`numpy.fft.fft2` / our `fft1d` use the **unnormalized** convention, so
`Σ|out|² = N² · Σ|in|²`. The GREEN test asserts `outSum ≈ N² · inSum` (NOT `outSum ≈ inSum`),
matching SVETlANNA's conserved power within float tolerance.

## Determinism / limits
- Pure Float64 complex math, no RNG, no `Date`. Same input → same output (RED: determinism test).
- This simulates the optical compute deterministically; it is the compute primitive, not a GPU/photonic
  runtime. Wire as an optional `field.ts` accelerator behind `opticalMatmul`.
