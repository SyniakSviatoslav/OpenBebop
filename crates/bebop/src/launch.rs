//! The sun-warm ship launch animation — a *moment*, not a spinner.
//!
//! Brand law: the launch view uses ONE saturated accent — sun-warm `#E8A544`
//! on the warm-noir void `#12100E`. The ship mark `◈` lifts along a warm
//! thruster trail.
//!
//! DETERMINISM: the only "randomness" is a const-seeded LCG
//! (`x = x*1664525 + 1013904223`). No `std::rand`, no
//! `std::time::SystemTime`. This keeps the animation provably reproducible —
//! a `#[test]` asserts same seed → same frame.

use crate::outfit::OUTFIT;

/// A frame buffer: `w*h` cells, each an RGBA u32 (0xAARRGGBB) on the void.
pub struct Frame {
    pub w: usize,
    pub h: usize,
    pub cells: Vec<u32>,
}

impl Frame {
    pub fn new(w: usize, h: usize, void: u32) -> Self {
        Frame {
            w,
            h,
            cells: vec![void; w * h],
        }
    }
    #[inline]
    pub fn set(&mut self, x: usize, y: usize, rgba: u32) {
        if x < self.w && y < self.h {
            self.cells[y * self.w + x] = rgba;
        }
    }
    /// Pack a 0xRRGGBB into opaque RGBA.
    #[inline]
    pub fn rgb(rgb: u32) -> u32 {
        0xFF000000 | (rgb & 0xFFFFFF)
    }
    /// Alpha-blend `fg` (0xAARRGGBB) over `bg` (0xRRGGBB) → 0xAARRGGBB.
    pub fn blend(fg: u32, bg: u32) -> u32 {
        let a = ((fg >> 24) & 0xFF) as f64 / 255.0;
        let fr = (fg >> 16) & 0xFF;
        let fg_ = (fg >> 8) & 0xFF;
        let fb = fg & 0xFF;
        let br = (bg >> 16) & 0xFF;
        let bg_ = (bg >> 8) & 0xFF;
        let bb = bg & 0xFF;
        let r = (fr as f64 * a + br as f64 * (1.0 - a)).round() as u32;
        let g = (fg_ as f64 * a + bg_ as f64 * (1.0 - a)).round() as u32;
        let b = (fb as f64 * a + bb as f64 * (1.0 - a)).round() as u32;
        0xFF000000 | (r << 16) | (g << 8) | b
    }
}

/// Linear-congruential generator (const-seeded). Reproducible by construction.
pub struct Lcg {
    state: u64,
}
impl Lcg {
    pub const fn new(seed: u64) -> Self {
        Lcg { state: seed }
    }
    /// Next value in [0, u32::MAX].
    pub fn next_u32(&mut self) -> u32 {
        // Numerical Recipes LCG constants.
        self.state = self.state.wrapping_mul(1664525).wrapping_add(1013904223);
        (self.state >> 32) as u32
    }
    /// Next float in [0,1).
    pub fn next_f(&mut self) -> f64 {
        (self.next_u32() as f64) / (u32::MAX as f64 + 1.0)
    }
}

/// The launch animation as a sequence of frames. Pure — given `seed`, the output is
/// bit-identical every run. `steps` controls the lift progress (0..=steps).
pub fn render_launch(w: usize, h: usize, _seed: u64, steps: usize) -> Vec<Frame> {
    render_launch_accent(w, h, _seed, steps, OUTFIT.palette.ship, OUTFIT.palette.void)
}

/// Like `render_launch`, but the launch accent (the hull/trail color) and the
/// void ground are parameters — so a customized `Outfit` recolors the ship.
/// Keeps the canonical brand launch when called with `OUTFIT.palette` values.
pub fn render_launch_accent(
    w: usize,
    h: usize,
    _seed: u64,
    steps: usize,
    ship_rgb: u32,
    void_rgb: u32,
) -> Vec<Frame> {
    let ship = Frame::rgb(ship_rgb);
    let void = void_rgb;
    let spark = Frame::rgb(OUTFIT.palette.ship); // warm ignition spark
    let mut frames = Vec::with_capacity(steps + 1);

    let cx = w / 2;
    let spark_y = h - 3; // thruster at bottom-center

    for s in 0..=steps {
        let mut f = Frame::new(w, h, void);
        let prog = s as f64 / steps as f64; // 0..1 lift progress

        // Phase 1+2: warm thruster trail rising from the spark, mirrored L/R.
        let trail_top = spark_y.saturating_sub((prog * (spark_y as f64)) as usize);
        for y in trail_top..=spark_y {
            let half = 1usize; // trail half-width (cells), mirrored L/R
            for dx in 0..=half {
                let lx = (cx as i32 - dx as i32).max(0) as usize;
                let rx = (cx + dx).min(w - 1);
                // brighter near the spark, fading up the trail
                let dist = (spark_y - y) as f64 / (spark_y.max(1) as f64);
                let alpha = ((1.0 - dist) * 200.0 + 40.0) as u32;
                let col = (alpha << 24) | (ship & 0xFFFFFF);
                f.set(lx, y, col);
                if rx != lx {
                    f.set(rx, y, col);
                }
            }
        }
        // Phase 0/3: sun-warm ignition spark blinks at the thruster.
        if s % 2 == 0 {
            f.set(cx, spark_y, spark);
        }
        // Phase 3: the ship (chevron + fins) ascends along the trail.
        let ship_y = spark_y.saturating_sub((prog * (spark_y as f64 - 2.0)) as usize);
        // hull: a small upward chevron around (cx, ship_y)
        for dx in 0..3u32 {
            let x = (cx as i32 + (dx as i32 - 1)).max(0) as usize;
            f.set(x, ship_y, Frame::blend(ship, void)); // ▲ nose
        }
        if ship_y + 1 < h {
            f.set(cx, ship_y + 1, ship); // body
            f.set((cx as i32 - 2).max(0) as usize, ship_y + 1, ship); // ◀ fin
            f.set((cx + 2).min(w - 1), ship_y + 1, ship); // ▶ fin
        }

        frames.push(f);
    }
    frames
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn launch_is_deterministic() {
        // RED+GREEN: same seed → bit-identical frames. A non-deterministic
        // source (e.g. Date/rand) would break this.
        let a = render_launch(40, 20, 0xC0FFEE, 10);
        let b = render_launch(40, 20, 0xC0FFEE, 10);
        assert_eq!(a.len(), b.len());
        for (fa, fb) in a.iter().zip(b.iter()) {
            assert_eq!(
                fa.cells, fb.cells,
                "frames diverged — launch is not deterministic"
            );
        }
    }

    #[test]
    fn launch_uses_ship_on_void() {
        // The launch view MUST carry the sun-warm ship accent on the void ground.
        let frames = render_launch(40, 20, 0xC0FFEE, 10);
        let ship = Frame::rgb(OUTFIT.palette.ship);
        let mut saw_ship = false;
        for f in &frames {
            if f.cells.iter().any(|&c| (c & 0xFFFFFF) == (ship & 0xFFFFFF)) {
                saw_ship = true;
                break;
            }
        }
        assert!(
            saw_ship,
            "launch shows no sun-warm ship/trail — violates brand law"
        );
    }

    #[test]
    fn lcgg_seeded_reproducible() {
        let mut a = Lcg::new(42);
        let mut b = Lcg::new(42);
        for _ in 0..100 {
            assert_eq!(a.next_u32(), b.next_u32());
        }
    }

    #[test]
    fn blend_over_void_keeps_fg_hue() {
        let ship = Frame::rgb(OUTFIT.palette.ship);
        let blended = Frame::blend(ship, OUTFIT.palette.void);
        // With full alpha, the result hue equals the fg hue.
        assert_eq!((blended & 0xFFFFFF), (ship & 0xFFFFFF));
    }

    #[test]
    fn ship_is_axially_symmetric() {
        // Brand law: the ship silhouette must be a clean left-right mirror.
        // Use an ODD width so there is a single center column; reflect about it.
        let w = 41;
        let frames = render_launch(w, 20, 0xC0FFEE, 18);
        let f = frames.last().unwrap();
        let h = f.h;
        let cx = w / 2; // integer center column (w odd → single column)
        let mut asym = 0usize;
        for y in 0..h {
            for x in 0..cx {
                let left = f.cells[y * w + x];
                let partner = (2 * cx - x) as usize; // reflect about center column
                if partner >= w {
                    continue;
                }
                let right = f.cells[y * w + partner];
                let l_on = (left & 0xFFFFFF) != (OUTFIT.palette.void & 0xFFFFFF);
                let r_on = (right & 0xFFFFFF) != (OUTFIT.palette.void & 0xFFFFFF);
                if l_on != r_on {
                    asym += 1;
                }
            }
        }
        assert_eq!(
            asym, 0,
            "ship hull is not axially symmetric ({asym} cells break mirror)"
        );
    }
}
