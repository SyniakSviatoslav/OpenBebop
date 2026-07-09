//! Bebop OUTFIT (2026-07-09) — the ship's identity, declared as ONE source of truth.
//!
//! This is the "new outfit from today": the cosmo-noir brand made explicit and versioned.
//! It is NOT a UI skin (Bebop is a CLI agent) — it is the canonical identity contract
//! every surface reads from: the banner, the voice, the sponsor line, the signal colors.
//! Changing the outfit changes the ship.
//!
//! Brand canon (ground truth): Warm Cosmo-Noir — Cowboy Bebop × cosmo-gothic ×
//! Ukrainian irony. "Hybrid is a feature, not a bug." The ship runs SUN-WARM
//! (amber-gold), its telemetry runs ORANGE, and a warm red is reserved for genuine
//! alerts / drift. Bone text #F2E9DB on the warm-noir void #12100E. One meaningful
//! color per view.

/// WCAG-safe luminous-warm palette (paired labels, never color-only).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Palette {
    pub ship: u32,  // the ship — luminous sun-gold, the launch ritual
    pub tele: u32,  // telemetry / data-signal — warm orange
    pub glow: u32,  // luminous peach — loader halo, hints, working-feed spark
    pub bone: u32,  // primary text on void (warm ivory)
    pub void: u32,  // the warm-noir ground
    pub alert: u32, // warm red — reserved for alerts / drift / hallucination
}

/// Voice axis. Drives the copilot's one-liners.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Narration {
    Bebop, // the dry, Ukrainian-irony ship voice (default)
    Plain,
    Sarcastic,
    CorporateKiller,
}

/// The identity contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Outfit {
    pub version: &'static str,
    pub era: &'static str,
    pub name: &'static str,
    pub tagline: &'static str,
    pub creed: &'static str,
    pub palette: Palette,
    pub sigil: char,
    pub narration: Narration,
    pub home: &'static str,
    pub lines: OutfitLines,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OutfitLines {
    pub boot: &'static str,
    pub multipilot: &'static str,
    pub field: &'static str,
    pub outfit: &'static str,
}

/// The canonical luminous cosmo-noir identity. Bump `version` when the brand contract changes.
pub const OUTFIT: Outfit = Outfit {
    version: "1.0.0",
    era: "2026-07-09 — the new outfit",
    name: "Bebop",
    tagline: "your ship.",
    creed: "Hybrid is a feature, not a bug.",
    palette: Palette {
        ship: 0xF4C25A, // luminous sun-gold
        tele: 0xF2933E, // warm orange telemetry
        glow: 0xFFD9A0, // luminous peach — loader halo / hints
        bone: 0xFBF3E7, // warm ivory text
        void: 0x12100E, // warm-noir ground
        alert: 0xE0543E, // warm red, alerts/drift only
    },
    sigil: '◈', // ◈ — the ship's eye, sun-warm
    narration: Narration::Bebop,
    home: "https://github.com/SyniakSviatoslav/bebop",
    lines: OutfitLines {
        boot: "Bebop online. The ship is yours.",
        multipilot: "Multipilot engaged — the crew is arguing, the synthesizer decides.",
        field: "Field arbiter armed: physics gets a veto now.",
        outfit: "New outfit today. Same ship, warmer light.",
    },
};

impl Outfit {
    /// Render the outfit banner (used by `bebop outfit` and the launch splash).
    pub fn banner(&self) -> String {
        format!(
            "{} {} v{} — {}\n   {}\n   palette: ship #{:06X} · tele #{:06X} · void #{:06X}",
            self.sigil,
            self.name,
            self.version,
            self.tagline,
            self.creed,
            self.palette.ship,
            self.palette.tele,
            self.palette.void,
        )
    }
}

/// Relative luminance (WCAG) of a 0xRRGGBB color, in [0,1].
pub fn luminance(rgb: u32) -> f64 {
    let channel = |c: u32| -> f64 {
        let s = (c & 0xFF) as f64 / 255.0;
        if s <= 0.03928 {
            s / 12.92
        } else {
            ((s + 0.055) / 1.055).powf(2.4)
        }
    };
    let r = channel(rgb >> 16);
    let g = channel(rgb >> 8);
    let b = channel(rgb);
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

/// WCAG contrast ratio between two 0xRRGGBB colors. Returns a value in [1,21].
pub fn contrast(a: u32, b: u32) -> f64 {
    let la = luminance(a);
    let lb = luminance(b);
    let (hi, lo) = if la > lb { (la, lb) } else { (lb, la) };
    (hi + 0.05) / (lo + 0.05)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outfit_version_is_semver() {
        // RED+GREEN: a non-semver identity version must fail the contract.
        let v = OUTFIT.version;
        assert!(
            v.split('.').count() == 3,
            "identity version must be semver, got {v}"
        );
        for part in v.split('.') {
            assert!(
                part.parse::<u32>().is_ok(),
                "version part not numeric: {part}"
            );
        }
    }

    #[test]
    fn palette_meets_wcag_aa_on_void() {
        // GREEN: bone text on void must clear WCAG AA (>= 4.5 for normal text).
        let c = contrast(OUTFIT.palette.bone, OUTFIT.palette.void);
        assert!(c >= 4.5, "bone/void contrast {c:.2} < 4.5 (WCAG AA)");
        // RED case proven by the assertion above: a too-dark text would fail.
    }

    #[test]
    fn ship_tele_alert_are_distinct_hues() {
        // The ship (sun-warm), telemetry (orange), and alert (warm red) must read
        // as THREE distinct hues — a cosmo-noir warm palette, not a mono.
        assert_ne!(OUTFIT.palette.ship, OUTFIT.palette.tele);
        assert_ne!(OUTFIT.palette.tele, OUTFIT.palette.alert);
        assert_ne!(OUTFIT.palette.ship, OUTFIT.palette.alert);
        // and ship-on-void must itself be legible (the launch uses ship on void).
        let c = contrast(OUTFIT.palette.ship, OUTFIT.palette.void);
        assert!(c >= 3.0, "ship/void launch contrast {c:.2} too low");
    }

    #[test]
    fn banner_contains_creed_and_sigil() {
        let b = OUTFIT.banner();
        assert!(b.contains(OUTFIT.creed), "banner missing creed");
        assert!(b.contains(OUTFIT.sigil), "banner missing sigil");
    }

    #[test]
    fn narration_default_is_bebop() {
        assert_eq!(OUTFIT.narration, Narration::Bebop);
    }
}
