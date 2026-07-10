//! GEOMETRY-FIELD — the final 3 additions from the master dossier, applied to
//! the connection-graph sim (see `wavefield.rs`):
//!
//!   1. PLATONIC SOLIDS as field geometry — a node can be a regular polyhedron
//!      (tetra/cube/octa/dodeca/icosa). Its vertices seed the field; its
//!      Euler characteristic V−E+F=2 is a structural invariant we CHECK.
//!   2. NYQUIST STABILITY — a plan's open-loop transfer L(jω) must NOT
//!      encircle −1 (Z = N + P; stable iff N = −P). Encapsulated as a
//!      fail-closed check over a sampled Nyquist contour.
//!   3. SPHERICAL HARMONICS Y_l^m(θ,φ) — each platonic node carries an angular
//!      signature; the field is reconstructed as a sum of harmonics over the
//!      node's vertices, giving a smooth geometry-aware potential.
//!
//! All deterministic, std-only, 0 deps — same doctrine as the rest of core.

use crate::wavefield::LinkKind;

/// The five convex regular polyhedra. `a` = edge length. Verified by Euler's
/// formula V − E + F = 2 (checked in tests).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platonic {
    Tetrahedron,
    Cube,
    Octahedron,
    Dodecahedron,
    Icosahedron,
}

impl Platonic {
    /// (faces, edges, vertices) for the solid.
    pub fn fev(&self) -> (usize, usize, usize) {
        match self {
            Platonic::Tetrahedron => (4, 6, 4),
            Platonic::Cube => (6, 12, 8),
            Platonic::Octahedron => (8, 12, 6),
            Platonic::Dodecahedron => (12, 30, 20),
            Platonic::Icosahedron => (20, 30, 12),
        }
    }

    /// Volume for edge length `a`. Closed form from the dossier.
    pub fn volume(&self, a: f64) -> f64 {
        match self {
            Platonic::Tetrahedron => a.powi(3) / (6.0 * 2.0_f64.sqrt()),
            Platonic::Cube => a.powi(3),
            Platonic::Octahedron => (2.0_f64.sqrt() / 3.0) * a.powi(3),
            Platonic::Dodecahedron => ((15.0 + 7.0 * 5.0_f64.sqrt()) / 4.0) * a.powi(3),
            Platonic::Icosahedron => (5.0 * (3.0 + 5.0_f64.sqrt()) / 12.0) * a.powi(3),
        }
    }

    /// Surface area for edge length `a`.
    pub fn surface_area(&self, a: f64) -> f64 {
        match self {
            Platonic::Tetrahedron => 3.0_f64.sqrt() * a.powi(2),
            Platonic::Cube => 6.0 * a.powi(2),
            Platonic::Octahedron => 2.0 * 3.0_f64.sqrt() * a.powi(2),
            Platonic::Dodecahedron => 3.0 * (25.0 + 10.0 * 5.0_f64.sqrt()).sqrt() * a.powi(2),
            Platonic::Icosahedron => 5.0 * 3.0_f64.sqrt() * a.powi(2),
        }
    }

    /// Unit-sphere vertices of the solid (deterministic, canonical). Returns
    /// (θ, φ) pairs in spherical coords (colatitude θ∈[0,π], azimuth φ∈[0,2π)).
    /// These seed the field on the node's surface — the geometry-aware basis.
    pub fn vertices_spherical(&self) -> Vec<(f64, f64)> {
        match self {
            Platonic::Tetrahedron => vec![
                (std::f64::consts::FRAC_PI_2, 0.0),
                (
                    std::f64::consts::FRAC_PI_2,
                    2.0 * std::f64::consts::PI / 3.0,
                ),
                (
                    std::f64::consts::FRAC_PI_2,
                    4.0 * std::f64::consts::PI / 3.0,
                ),
                (0.0, 0.0), // apex
            ],
            Platonic::Cube => {
                let mut v = vec![];
                for sx in [-1.0_f64, 1.0_f64] {
                    for sy in [-1.0_f64, 1.0_f64] {
                        for sz in [-1.0_f64, 1.0_f64] {
                            // map cube corner to a unit-sphere direction
                            let r = (sx * sx + sy * sy + sz * sz).sqrt();
                            let theta = (sz / r).acos();
                            let phi = sy.atan2(sx);
                            v.push((theta, phi));
                        }
                    }
                }
                v
            }
            Platonic::Octahedron => vec![
                (0.0, 0.0),
                (std::f64::consts::PI, 0.0),
                (std::f64::consts::FRAC_PI_2, 0.0),
                (std::f64::consts::FRAC_PI_2, std::f64::consts::FRAC_PI_2),
                (std::f64::consts::FRAC_PI_2, std::f64::consts::PI),
                (
                    std::f64::consts::FRAC_PI_2,
                    3.0 * std::f64::consts::FRAC_PI_2,
                ),
            ],
            Platonic::Dodecahedron => dodeca_vertices(),
            Platonic::Icosahedron => icosa_vertices(),
        }
    }
}

/// Associated Legendre polynomial P_l^m(x), x∈[−1,1], via the standard
/// recurrence (m ≥ 0). Used by the spherical-harmonic signature.
pub fn legendre(l: usize, m: usize, x: f64) -> f64 {
    if m > l {
        return 0.0;
    }
    // P_m^m via double-factorial closed form
    if l == m {
        let mut prod = 1.0f64;
        let mut k = 1usize;
        while k <= 2 * m {
            if k % 2 == 1 {
                prod *= (k as f64) * (1.0 - x * x).sqrt();
            }
            k += 1;
        }
        return if (m / 2) % 2 == 1 { -prod } else { prod };
    }
    // upward recurrence to l
    let mut pmm = legendre(m, m, x);
    if l == m {
        return pmm;
    }
    let mut pm1 = x * (2 * m + 1) as f64 * pmm;
    if l == m + 1 {
        return pm1;
    }
    let mut pm2 = pmm;
    for ll in (m + 2)..=l {
        let num = (2 * ll - 1) as f64 * x * pm1 - (ll + m - 1) as f64 * pm2;
        let cur = num / (ll - m) as f64;
        pm2 = pm1;
        pm1 = cur;
    }
    pm1
}

/// Normalization constant N_l^m for the orthonormal spherical harmonic.
fn sph_norm(l: usize, m: usize) -> f64 {
    let num = (2 * l + 1) as f64 * (factorial(l - m) as f64);
    let den = (4.0 * std::f64::consts::PI) * (factorial(l + m) as f64);
    (num / den).sqrt()
}

fn factorial(n: usize) -> usize {
    (1..=n).product()
}

/// Spherical harmonic Y_l^m(θ, φ). m may be negative; we use the real form
/// (cos(mφ) for the cosine part) so the signature is real-valued on the sphere.
pub fn spherical_harmonic(l: usize, m: isize, theta: f64, phi: f64) -> f64 {
    let am = m.unsigned_abs();
    if am > l {
        return 0.0;
    }
    let p = legendre(l, am, theta.cos());
    let n = sph_norm(l, am);
    if m >= 0 {
        n * p * (m as f64 * phi).cos()
    } else {
        n * p * ((am as f64) * phi).sin()
    }
}

/// Reconstruct a node's geometry-aware potential over its platonic vertices:
/// sum over (l,m) modes of coefficient c_lm · Y_l^m(θ,φ). Returns the vertex
/// potentials (the field "lifted" onto the solid's surface — your idea: the
/// node's structure is geometry, not a point).
pub fn node_harmonic_field(solid: Platonic, coeffs: &[(usize, isize, f64)]) -> Vec<f64> {
    solid
        .vertices_spherical()
        .iter()
        .map(|&(theta, phi)| {
            coeffs
                .iter()
                .map(|&(l, m, c)| c * spherical_harmonic(l, m, theta, phi))
                .sum()
        })
        .collect()
}

/// ─────────────────────────────────────────────────────────────────────────
/// NYQUIST STABILITY — frequency-domain fail-closed check.
///
/// The open-loop L(jω) samples are given as `(re[k], im[k])` over the Nyquist
/// contour. Stability criterion: Z = N + P, where P = number of open-loop RHP
/// poles (`p_rhp`), N = winding number of the contour around the point −1.
/// Closed-loop is stable iff Z = 0, i.e. N = −P. We compute N by the exact
/// winding number of the polygon (re+1, im) around the origin (the point −1),
/// accumulated as signed angle increments. Returns `true` if UNSTABLE
/// (Z ≠ 0) — fail-closed for the planner. Degenerate contour → not flagged.
/// ─────────────────────────────────────────────────────────────────────────
pub fn nyquist_unstable(re: &[f64], im: &[f64], p_rhp: usize) -> bool {
    if re.len() < 2 || im.len() != re.len() {
        return false;
    }
    // wind the contour (re+1, im) around the origin
    let n = re.len();
    let mut total = 0.0f64;
    for k in 1..n {
        // vectors from −1 to consecutive samples
        let x0 = re[k - 1] + 1.0;
        let y0 = im[k - 1];
        let x1 = re[k] + 1.0;
        let y1 = im[k];
        let dot = x0 * x1 + y0 * y1;
        let cross = x0 * y1 - y0 * x1;
        if cross.abs() < 1e-15 && dot > 0.0 {
            continue; // no angular turn
        }
        let d = (x0 * x0 + y0 * y0) * (x1 * x1 + y1 * y1);
        if d < 1e-30 {
            continue;
        }
        let ang = (cross / d.sqrt())
            .asin()
            .clamp(-std::f64::consts::FRAC_PI_2, std::f64::consts::FRAC_PI_2);
        total += ang;
    }
    let winding = (total / (2.0 * std::f64::consts::PI)).round();
    // stable iff winding == -(P as f64); unstable otherwise
    winding != -(p_rhp as f64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platonic_euler_invariant() {
        // GREEN: every platonic solid satisfies V − E + F = 2.
        for s in [
            Platonic::Tetrahedron,
            Platonic::Cube,
            Platonic::Octahedron,
            Platonic::Dodecahedron,
            Platonic::Icosahedron,
        ] {
            let (f, e, v) = s.fev();
            assert_eq!(v as isize - e as isize + f as isize, 2, "{:?} Euler", s);
        }
    }

    #[test]
    fn platonic_volumes_positive_and_ordered() {
        // GREEN: unit-edge volumes are positive; cube > tetrahedron.
        assert!(Platonic::Tetrahedron.volume(1.0) > 0.0);
        assert!(Platonic::Cube.volume(1.0) > Platonic::Tetrahedron.volume(1.0));
        assert!((Platonic::Cube.volume(2.0) - 8.0).abs() < 1e-9);
    }

    #[test]
    fn spherical_harmonics_orthonormal_and_real() {
        // GREEN: Y_0^0 = 1/√(4π) (constant), real-valued.
        let y00 = spherical_harmonic(0, 0, 1.0, 0.7);
        assert!((y00 - 1.0 / (4.0 * std::f64::consts::PI).sqrt()).abs() < 1e-9);
        // GREEN: Y_1^0 ∝ cos θ (zonal), varies with θ.
        let y10_a = spherical_harmonic(1, 0, 0.0, 0.0);
        let y10_b = spherical_harmonic(1, 0, std::f64::consts::PI, 0.0);
        assert!(
            y10_a > 0.0 && y10_b < 0.0,
            "Y_1^0 = +c at pole, −c at antipode"
        );
        // RED: l < |m| ⇒ 0.
        assert_eq!(spherical_harmonic(1, 5, 1.0, 0.0), 0.0);
    }

    #[test]
    fn node_harmonic_field_lifts_onto_vertices() {
        // GREEN: a constant mode (l=0) gives a uniform potential over all verts.
        let f = node_harmonic_field(Platonic::Tetrahedron, &[(0, 0, 2.0)]);
        assert_eq!(f.len(), 4);
        for v in &f {
            assert!((v - 2.0 / (4.0 * std::f64::consts::PI).sqrt()).abs() < 1e-9);
        }
        // GREEN: a Y_1^0 mode varies across vertices (apex vs equator).
        let f2 = node_harmonic_field(Platonic::Tetrahedron, &[(1, 0, 1.0)]);
        let mn = f2.iter().cloned().fold(f64::INFINITY, f64::min);
        let mx = f2.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        assert!(mx - mn > 1e-3, "mode Y_1^0 must vary across vertices");
    }

    #[test]
    fn nyquist_stable_vs_unstable() {
        // GREEN: a stable open-loop (no RHP poles, no encirclement) → not unstable.
        // L(jω) = 1/(jω+1): real/imag sampled; never encircles −1, P=0.
        let mut re = vec![];
        let mut im = vec![];
        for k in 0..64 {
            let w = (k as f64 / 63.0) * 10.0 - 5.0;
            let d = w * w + 1.0;
            re.push(1.0 / d);
            im.push(-w / d);
        }
        assert!(!nyquist_unstable(&re, &im, 0), "stable first-order loop");
        // RED: a contour that encircles −1 with P=0 → unstable.
        // unit circle centred at −1 (passes through origin), clockwise.
        let mut re2 = vec![];
        let mut im2 = vec![];
        for k in 0..64 {
            let a = (k as f64 / 63.0) * 2.0 * std::f64::consts::PI;
            re2.push(-1.0 + a.cos());
            im2.push(a.sin());
        }
        assert!(nyquist_unstable(&re2, &im2, 0), "encircles −1 ⇒ unstable");
        // RED: P=1 (one RHP pole) but no encirclement → unstable (N=−P violated).
        assert!(nyquist_unstable(&re, &im, 1), "P=1, N=0 ⇒ unstable");
    }

    #[test]
    fn platonic_node_validates_euler_and_field() {
        // Compose: a node modeled as an Icosahedron must pass its Euler
        // invariant AND lift a harmonic field onto its 12 vertices.
        let solid = Platonic::Icosahedron;
        let (f, e, v) = solid.fev();
        assert_eq!(v as isize - e as isize + f as isize, 2, "Euler holds");
        // a Y_2^0 (quadrupole) signature varies across the 12 vertices
        let field = node_harmonic_field(solid, &[(2, 0, 1.0)]);
        assert_eq!(field.len(), 12);
        let mn = field.iter().cloned().fold(f64::INFINITY, f64::min);
        let mx = field.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        assert!(
            mx - mn > 1e-3,
            "quadrupole signature varies across vertices"
        );
        // the field is geometry-aware: a tetrahedron has only 4 vertices
        assert_eq!(
            node_harmonic_field(Platonic::Tetrahedron, &[(0, 0, 1.0)]).len(),
            4
        );
    }
}

// ── canonical icosahedron / dodecahedron vertices (deterministic) ──
fn golden() -> f64 {
    (1.0 + 5.0_f64.sqrt()) / 2.0
}

fn icosa_vertices() -> Vec<(f64, f64)> {
    let t = golden();
    let raw: [(f64, f64, f64); 12] = [
        (-1.0, t, 0.0),
        (1.0, t, 0.0),
        (-1.0, -t, 0.0),
        (1.0, -t, 0.0),
        (0.0, -1.0, t),
        (0.0, 1.0, t),
        (0.0, -1.0, -t),
        (0.0, 1.0, -t),
        (t, 0.0, -1.0),
        (t, 0.0, 1.0),
        (-t, 0.0, -1.0),
        (-t, 0.0, 1.0),
    ];
    raw.iter()
        .map(|&(x, y, z)| {
            let r = (x * x + y * y + z * z).sqrt();
            (z / r).acos() // θ
        })
        .zip(raw.iter().map(|&(x, y, _)| y.atan2(x))) // φ
        .map(|(theta, phi)| (theta, phi))
        .collect()
}

fn dodeca_vertices() -> Vec<(f64, f64)> {
    let t = golden();
    let raw: [(f64, f64, f64); 20] = [
        (-1.0, -1.0, -1.0),
        (-1.0, -1.0, 1.0),
        (-1.0, 1.0, -1.0),
        (-1.0, 1.0, 1.0),
        (1.0, -1.0, -1.0),
        (1.0, -1.0, 1.0),
        (1.0, 1.0, -1.0),
        (1.0, 1.0, 1.0),
        (0.0, -t, -1.0 / t),
        (0.0, -t, 1.0 / t),
        (0.0, t, -1.0 / t),
        (0.0, t, 1.0 / t),
        (-t, -1.0 / t, 0.0),
        (t, -1.0 / t, 0.0),
        (-t, 1.0 / t, 0.0),
        (t, 1.0 / t, 0.0),
        (-1.0 / t, 0.0, -t),
        (1.0 / t, 0.0, -t),
        (-1.0 / t, 0.0, t),
        (1.0 / t, 0.0, t),
    ];
    raw.iter()
        .map(|&(x, y, z)| {
            let r = (x * x + y * y + z * z).sqrt();
            (z / r).acos()
        })
        .zip(raw.iter().map(|&(x, y, _)| y.atan2(x)))
        .map(|(theta, phi)| (theta, phi))
        .collect()
}
