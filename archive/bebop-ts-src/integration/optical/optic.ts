/**
 * optic.ts — Deterministic optical "physical compute" kernel for the Sovereign Node.
 *
 * Reverse-engineered from SVETlANNa (CompPhysLab/SVETlANNa, PyTorch Fourier-optics library)
 * and Meep (MIT FDTD). The core operation of a free-space optical computer is:
 *
 *   light through a phase/amplitude mask, then a thin lens, performs a 2D LINEAR TRANSFORM.
 *   A thin lens applies  t(x,y) = exp(-i·k·(x²+y²)/(2f))   (paraxial phase curvature)
 *   and the lens + free-space pair performs the DISCRETE FOURIER TRANSFORM of the masked field:
 *
 *       G(fx,fy) = FFT₂D{ t(x,y) · E_in(x,y) }
 *
 *   This is "compute with light, ~zero energy on the logic": the matrix product is performed
 *   by diffraction/propagation, not by switching transistors.
 *
 * The implementation is DETERMINISTIC and physical-law enforcing:
 *   - fixed Float64, no RNG, no Date
 *   - a passive optical mask must satisfy |t(x,y)| <= 1 everywhere (energy cannot be gained
 *     by a passive element) — physically-impossible masks are REJECTED.
 *
 * SVETlANNa forward-pass evidence (captured, /root/bebop-repo/.venv2/run_svet.py):
 *   INPUT  shape (64,64) complex64, sum |ΣE| = 144.0
 *   OUTPUT shape (64,64), Σ|E|² = 143.99995  ≈ 144  → power conserved by passive mask
 *   ThinLens transmission at center: exp(-i·k/(2f)·0) = (1 - 2.9e-6j) ≈ 1.
 *
 * Meep (MIT FDTD) Yee-lattice update equations (from meep.readthedocs.io FAQ/reference):
 *   1D along z for Hx, Hy fields:
 *     Hx^{n+1/2}(i) = Hx^{n-1/2}(i) - (dt/μ)·[ Ez^{n}(i) - Ez^{n}(i-1) ] / dz
 *     Hy^{n+1/2}(i) = Hy^{n-1/2}(i) + (dt/μ)·[ Ex^{n}(i) - Ex^{n}(i-1) ] / dz
 *   and the E-field update (with ε):
 *     Ex^{n+1}(i) = Ex^{n}(i) + (dt/ε)·[ Hy^{n+1/2}(i+1) - Hy^{n+1/2}(i) ] / dz
 *   (Yee grid: E and H are staggered in space & time; Courant limit dt ≤ dz/c/√D.)
 *   FDTD solves the SAME Maxwell wave equation that the monochromatic FFT propagation
 *   approximates in the far/paraxial regime — both are the optical linear transform.
 */

export type Complex = [number, number]; // [re, im], Float64

export interface OpticalMask {
  /** n×n complex transmission matrix t(x,y) = [re, im]. Must satisfy |t| <= 1 (passive). */
  transmission: Complex[][];
}

export class OpticalLawError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'OpticalLawError';
  }
}

// ── Complex arithmetic helpers (Float64) ──

function cmul(a: Complex, b: Complex): Complex {
  return [a[0] * b[0] - a[1] * b[1], a[0] * b[1] + a[1] * b[0]];
}

function cabs2(c: Complex): number {
  return c[0] * c[0] + c[1] * c[1];
}

// ── Radix-2 Cooley–Tukey 1D FFT (deterministic, Float64) ──
// Sign = +1 for forward (exp(-i2πk/N)), -1 for inverse.
function fft1d(a: Complex[], sign: number): Complex[] {
  const n = a.length;
  if (n <= 1) return a.slice();
  // iterative bit-reversal permutation
  const rev = new Array<number>(n);
  let bits = 0;
  while (1 << bits < n) bits++;
  for (let i = 0; i < n; i++) {
    let r = 0;
    for (let b = 0; b < bits; b++) r |= ((i >> b) & 1) << (bits - 1 - b);
    rev[i] = r;
  }
  const out = a.map((_, i) => a[rev[i]]);
  for (let len = 2; len <= n; len <<= 1) {
    const ang = (-sign * 2 * Math.PI) / len;
    const wlen: Complex = [Math.cos(ang), Math.sin(ang)];
    for (let i = 0; i < n; i += len) {
      let w: Complex = [1, 0];
      for (let k = 0; k < len >> 1; k++) {
        const u = out[i + k];
        const v = cmul(w, out[i + k + (len >> 1)]);
        out[i + k] = [u[0] + v[0], u[1] + v[1]];
        out[i + k + (len >> 1)] = [u[0] - v[0], u[1] - v[1]];
        w = cmul(w, wlen);
      }
    }
  }
  return out;
}

/** Forward 2D FFT matching numpy.fft.fft2 convention: FFT over rows, then over columns. */
function fft2d(m: Complex[][]): Complex[][] {
  const n = m.length;
  // rows
  const rows = m.map((r) => fft1d(r, +1));
  // columns
  const cols: Complex[][] = Array.from({ length: n }, () => new Array<Complex>(n));
  for (let j = 0; j < n; j++) {
    const col: Complex[] = [];
    for (let i = 0; i < n; i++) col.push(rows[i][j]);
    const fc = fft1d(col, +1);
    for (let i = 0; i < n; i++) cols[i][j] = fc[i];
  }
  return cols;
}

/**
 * opticalMatmul — the optical "compute-with-light" primitive.
 *
 * @param x    n×n real input field (the encoded input vector/matrix).
 * @param mask OpticalMask with n×n complex transmission t(x,y).
 * @returns n×n complex field = FFT₂D{ t(x,y) · x(x,y) }  (lens+propagation = Fourier transform).
 *
 * Throws OpticalLawError if the mask is physically impossible:
 *   - any |t(x,y)| > 1  (passive element cannot amplify)  → gain violation
 */
export function opticalMatmul(x: number[][], mask: OpticalMask): Complex[][] {
  const n = x.length;
  if (n === 0 || x[0].length !== n) {
    throw new OpticalLawError('opticalMatmul: x must be a square n×n matrix');
  }
  const t = mask.transmission;
  if (t.length !== n || (n > 0 && t[0].length !== n)) {
    throw new OpticalLawError('opticalMatmul: mask transmission must be n×n matching x');
  }

  // Enforce physical law: a passive optical element cannot AMPLIFY light, so every
  // transmission coefficient must satisfy |t(x,y)| <= 1. (A pure phase shift, e.g.
  // t = -1 + 0i from a π lens, has |t| = 1 and is perfectly passive — it must NOT
  // be rejected.) Only gain (|t| > 1) is unphysical for a passive element.
  let gainViolation = false;
  for (let i = 0; i < n; i++) {
    for (let j = 0; j < n; j++) {
      const ti = t[i][j];
      if (!Number.isFinite(ti[0]) || !Number.isFinite(ti[1])) {
        throw new OpticalLawError('opticalMatmul: non-finite mask transmission');
      }
      if (cabs2(ti) > 1 + 1e-12) gainViolation = true;
    }
  }
  if (gainViolation) {
    throw new OpticalLawError(
      'opticalMatmul: mask violates passivity |t(x,y)| > 1 (light cannot be amplified by a passive element)',
    );
  }

  // Modulate the input field by the mask: E = t · x  (elementwise complex-scalar product).
  const modulated: Complex[][] = Array.from({ length: n }, () => new Array<Complex>(n));
  for (let i = 0; i < n; i++) {
    for (let j = 0; j < n; j++) {
      modulated[i][j] = cmul(t[i][j], [x[i][j], 0]);
    }
  }

  // Lens + free-space propagation = 2D Fourier transform (the optical matrix multiply).
  return fft2d(modulated);
}

/**
 * Build a physically-valid passive thin-lens phase mask:
 *   t(x,y) = exp(-i·k·(x²+y²)/(2f)),  |t| = 1 everywhere.
 * @param n grid size, f focal length (any positive), k wave number (2π/λ).
 */
export function thinLensMask(n: number, f: number, k: number): OpticalMask {
  if (f <= 0) throw new OpticalLawError('thinLensMask: focal length must be positive');
  if (k <= 0) throw new OpticalLawError('thinLensMask: wave number must be positive');
  const t: Complex[][] = Array.from({ length: n }, () => new Array<Complex>(n));
  for (let i = 0; i < n; i++) {
    for (let j = 0; j < n; j++) {
      const r2 = (i - n / 2) ** 2 + (j - n / 2) ** 2;
      const phase = (-k * r2) / (2 * f);
      t[i][j] = [Math.cos(phase), Math.sin(phase)]; // |t| = 1 → passive, valid
    }
  }
  return { transmission: t };
}
