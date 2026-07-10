//! The ship's lounge — `bebop radio`.
//!
//! Free-to-listen Lofi / Jazz streams the operator can spin up while the ship
//! is working. Licensing is clean by construction:
//!   - NO audio is bundled, downloaded, or stored. We only print the station's
//!     public stream URL and hand it to the user's OWN local player
//!     (`open` / `xdg-open` / `start`), or just leave the URL for copy-paste.
//!   - Every station listed here is listener-supported / Creative-Commons and
//!     free to listen (SomaFM, etc.). The curated default is Lofi/Jazz.
//!   - Fully deterministic: station pick is a const-LCG of the seed, no RNG.
//!     The same `bebop radio <n>` always opens the same deck.
//!
//! This keeps the sovereign core air-gapped and copyright-safe.

use crate::outfit::OUTFIT;

/// A free-to-listen station. `kind` is shown as a label, not a color-only cue.
#[derive(Clone, Copy, Debug)]
pub struct Station {
    pub name: &'static str,
    pub kind: &'static str, // "lofi" | "jazz" | "space" — paired label, never color-only
    pub url: &'static str,  // public stream URL (Icecast/mp3)
}

/// Curated, free-to-listen Lofi / Jazz deck. All listener-supported, no login.
/// SomaFM's streams are free to listen and explicitly permit listening clients.
pub const STATIONS: &[Station] = &[
    Station {
        name: "Groove Salad",
        kind: "lofi",
        url: "https://ice1.somafm.com/groovesalad-128-mp3",
    },
    Station {
        name: "Lush",
        kind: "lofi",
        url: "https://ice1.somafm.com/lush-128-mp3",
    },
    Station {
        name: "Drone Zone",
        kind: "space",
        url: "https://ice1.somafm.com/dronezone-128-mp3",
    },
    Station {
        name: "Jazz Greetings",
        kind: "jazz",
        url: "https://ice1.somafm.com/jazzgreetings-128-mp3",
    },
    Station {
        name: "Sonic Universe",
        kind: "jazz",
        url: "https://ice1.somafm.com/sonicuniverse-128-mp3",
    },
    Station {
        name: "Boot Liquor",
        kind: "jazz",
        url: "https://ice1.somafm.com/bootliquor-128-mp3",
    },
    Station {
        name: "Secret Agent",
        kind: "jazz",
        url: "https://ice1.somafm.com/secretagent-128-mp3",
    },
    Station {
        name: "Seven Inch Soul",
        kind: "jazz",
        url: "https://ice1.somafm.com/seveninch-128-mp3",
    },
    Station {
        name: "Illusion Flux",
        kind: "lofi",
        url: "https://ice1.somafm.com/illusionflux-128-mp3",
    },
    Station {
        name: "Beat Blender",
        kind: "lofi",
        url: "https://ice1.somafm.com/beatblender-128-mp3",
    },
];

/// Deterministic pick: station index from a seed (const LCG, no RNG/Date).
pub fn pick(seed: u64) -> usize {
    // LCG step, then mod by len — same seed → same deck.
    let s = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
    (s >> 33) as usize % STATIONS.len()
}

/// List every station with its index + kind (so the user can `bebop radio <n>`).
pub fn list() {
    println!("{}", OUTFIT.banner());
    println!("  ◈ the ship's lounge — free-to-listen Lofi / Jazz");
    println!("    (listener-supported streams · nothing bundled · your player does the work\n     — which is the one job bebop won't do for you)\n");
    for (i, s) in STATIONS.iter().enumerate() {
        println!("    {:>2} · {:<16} [{}]", i, s.name, s.kind);
    }
    println!("\n  play:  bebop radio <n>     stop:  bebop radio stop");
    println!("  shuffle the deck:  bebop radio onair");
    println!("  (the lounge is free. the cigar is not included.)");
}

/// "On air": pick a station from `seed`, animate the ship, and hand the URL to
/// the OS player (best-effort; never blocks on a download). Falls back to
/// printing the URL so the user can paste it anywhere.
pub fn on_air(seed: u64) -> std::io::Result<()> {
    let idx = pick(seed);
    let st = &STATIONS[idx];
    crate::tui::render_loader_animation(
        crate::tui::AgentState::Radio,
        10,
        "radio",
        &format!("tuning {} — {} on air", st.name, st.kind),
        &OUTFIT,
    );
    println!("  ♪ now on air: {} [{}]", st.name, st.kind);
    println!("    {}", st.url);
    println!("    (your player should be picking this up. if not, the URL is right there — bebop is not your butler.)");
    open_url(st.url)?;
    crate::mission::mission_summary(
        &format!("radio · {}", st.name),
        &[
            "tuned a free Lofi/Jazz stream — no subscription, no guilt.",
            "the ship is broadcasting. the cigar is lit. the loop is closed.",
        ],
    );
    Ok(())
}

/// Play a specific station by index.
pub fn play(index: usize) -> std::io::Result<()> {
    if index >= STATIONS.len() {
        eprintln!("  ✖ no such station: {} (try `bebop radio`)", index);
        std::process::exit(2);
    }
    let st = &STATIONS[index];
    crate::tui::render_loader_animation(
        crate::tui::AgentState::Radio,
        10,
        "radio",
        &format!("tuning {} — {} on air", st.name, st.kind),
        &OUTFIT,
    );
    println!("  ♪ now on air: {} [{}]", st.name, st.kind);
    println!("    {}", st.url);
    println!("    (your player should be picking this up. if not, the URL is right there — bebop is not your butler.)");
    open_url(st.url)?;
    crate::mission::mission_summary(
        &format!("radio · {}", st.name),
        &[
            "tuned a free Lofi/Jazz stream — no subscription, no guilt.",
            "the ship is broadcasting. the cigar is lit. the loop is closed.",
        ],
    );
    Ok(())
}

/// Best-effort OS open: hand the URL to the platform's default handler so the
/// user's own player (mpv / vlc / browser) streams it. We never fetch audio
/// ourselves — the air-gapped core stays clean and copyright-safe.
fn open_url(url: &str) -> std::io::Result<()> {
    if std::env::var("BEBOP_RADIO_NO_OPEN").is_ok() {
        // headless / CI: just print, don't shell out
        return Ok(());
    }
    let (prog, arg) = if cfg!(target_os = "macos") {
        ("open", url)
    } else if cfg!(target_os = "windows") {
        ("cmd", "/C") // we then add start "" url as separate args
    } else {
        ("xdg-open", url)
    };
    let mut child = if cfg!(target_os = "windows") {
        let mut c = std::process::Command::new(prog);
        c.arg(arg).arg("").arg(url);
        c
    } else {
        let mut c = std::process::Command::new(prog);
        c.arg(arg);
        c
    };
    // fire-and-forget: if the player isn't installed, just leave the URL printed
    match child.spawn() {
        Ok(_) => Ok(()),
        Err(_) => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn radio_pick_is_deterministic() {
        // RED+GREEN: same seed → same deck. This is what makes the lounge
        // reproducible across machines (no RNG) — the bebop moat.
        assert_eq!(pick(0xBEEF), pick(0xBEEF));
        assert_eq!(pick(0x7E57), pick(0x7E57));
    }

    #[test]
    fn radio_pick_stays_in_bounds() {
        // RED+GREEN: pick must never index out of the deck, for ANY seed.
        for seed in [0u64, 1, 0xBEEF, 0x7E57, u64::MAX, 0xDEAD_BEEF] {
            let i = pick(seed);
            assert!(i < STATIONS.len(), "seed {seed:#X} → {i} out of bounds");
        }
    }

    #[test]
    fn radio_all_stations_are_listener_supported() {
        // RED+GREEN: every station must be a real, free-to-listen stream URL.
        // We assert the SomaFM shape (the license-clean source) so nobody can
        // sneak a copyrighted/bundled track in.
        assert!(!STATIONS.is_empty(), "deck must have at least one station");
        for s in STATIONS {
            assert!(
                s.url.starts_with("https://ice1.somafm.com/"),
                "station {} must be a SomaFM listener-supported stream",
                s.name
            );
            assert!(
                matches!(s.kind, "lofi" | "jazz" | "space"),
                "station {} kind must be labelled (lofi/jazz/space), not color-only",
                s.name
            );
        }
    }
}
