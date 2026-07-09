//! Mission summary — the sign-off.
//!
//! At the end of a session / task / loop, bebop lights a cigar on the station
//! dock and files the report. The voice is *clear-but-ironic*: it tells you
//! exactly what happened, then undercuts it with a smoke and a shrug — the
//! Cowboy Bebop × cosmo-gothic × Ukrainian-irony canon that is project-wide law.
//!
//! Deterministic: the cigar-smoke animation is a precomputed frame ring (no RNG,
//! no Date). The same mission always paints the same dock. TTY-gated — in a pipe
//! or CI it prints a single static frame, so output is still reproducible.

use crate::outfit::OUTFIT;
use std::time::Duration;

/// Cigar-smoke frames, rising one notch each tick. Same every run.
const SMOKE: [&str; 4] = [
    "                 ≈≈≈",
    "              ≈≈≈ ♢",
    "           ≈≈≈ ♢",
    "        ≈≈≈ ♢    ☆",
];

/// The station + bebop at the dock, with a glowing cigar. Deterministic scene.
fn station() -> String {
    format!(
        "\x1b[38;5;180m\
   ☆   ╔══════════════════╗   ☆\n\
       ║  ⊂(•‿•)⊃≈  bebop  ║\n\
   ·   ║    ◈ ▶ ◈   dock   ║   ·\n\
       ╚══════════════════╝\x1b[0m"
    )
}

/// Full scene for a given smoke frame (pure — used by tests + animation).
pub fn scene(smoke_idx: usize) -> String {
    let smoke = SMOKE[smoke_idx % SMOKE.len()];
    format!("{}\n{}", smoke, station())
}

/// The sign-off. `title` is the mission name; `lines` are the clear-but-ironic
/// bullets. Animates the cigar in a TTY, prints one frame otherwise.
pub fn mission_summary(title: &str, lines: &[&str]) {
    let is_tty = crossterm::tty::IsTty::is_tty(&std::io::stdout());
    let animated = is_tty && std::env::var("BEBOP_NO_ANIM").is_err();
    let frames = if animated { SMOKE.len() } else { 1 };

    for f in 0..frames {
        if f > 0 {
            // rewind 5 lines (smoke + 4 station) and clear each, then redraw
            for _ in 0..5 {
                print!("\x1b[1A\x1b[2K");
            }
            std::thread::sleep(Duration::from_millis(220));
        }
        print!("{}\x1b[K", scene(f));
        use std::io::Write;
        let _ = std::io::stdout().flush();
    }
    println!();

    let o = &OUTFIT;
    println!("{}", o.banner());
    println!("  ◈ mission: {}", title);
    for l in lines {
        println!("    · {}", l);
    }
    println!(
        "  ◈ signed — bebop. still flying. (cigar not shipped in the build)"
    );
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mission_scene_shows_dock_and_cigar() {
        // RED+GREEN: the dock scene must contain bebop, the station box, and smoke.
        let s = scene(0);
        assert!(s.contains("bebop"), "scene must name the ship");
        assert!(s.contains("╔"), "scene must draw the station frame");
        assert!(s.contains("≈"), "scene must show the cigar smoke");
        assert!(s.contains("⊃≈"), "cigar must be at bebop's mouth");
    }

    #[test]
    fn mission_scene_is_deterministic() {
        // RED+GREEN: same frame index → byte-identical scene (air-gapped moat).
        assert_eq!(scene(1), scene(1));
        assert_eq!(scene(3), scene(3));
        // and the ring wraps cleanly
        assert_eq!(scene(4), scene(0));
    }
}
