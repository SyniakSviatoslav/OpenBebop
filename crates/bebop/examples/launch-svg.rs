//! Example: render the deterministic sun-warm ship launch to an SVG strip.
//! Proof-of-design artifact for the CLI recording. Run:
//!   cargo run --example launch-svg --release > docs/design/bebop-launch.svg
//! The frames are the SAME `render_launch` output the TUI uses, so what you
//! see is bit-identical to the live TUI (one source of truth).
use bebop::launch::{render_launch, Frame};
use bebop::outfit::OUTFIT;
use std::env;

fn main() {
    // args: [cell] [frames_to_show]  (defaults: cell=8, show=all)
    let cell = env::args()
        .nth(1)
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(8);
    let max_show = env::args().nth(2).and_then(|s| s.parse::<usize>().ok());

    let w = 41;
    let h = 20;
    let steps = 18;
    let frames = render_launch(w, h, 0xC0FFEE, steps);
    let show: Vec<&Frame> = match max_show {
        Some(n) => frames
            .iter()
            .step_by(((steps + 1) / n.max(1)).max(1))
            .collect(),
        None => frames.iter().collect(),
    };

    // lay frames out horizontally with a gap
    let gap = 4u32;
    let cell = 8u32; // px per cell
    let fw = w as u32 * cell;
    let fh = h as u32 * cell;
    let total_w = (fw + gap) * (steps as u32 + 1) - gap;
    let total_h = fh + 24;

    let void = OUTFIT.palette.void;
    let ship = OUTFIT.palette.ship;

    println!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{total_w}\" height=\"{total_h}\" viewBox=\"0 0 {total_w} {total_h}\">");
    println!(
        "<rect width=\"{total_w}\" height=\"{total_h}\" fill=\"#{:06X}\"/>",
        void
    );
    println!(
        "<text x=\"4\" y=\"16\" fill=\"#{:06X}\" font-family=\"monospace\" font-size=\"12\">Bebop v{} — sun-warm ship launch (deterministic, ship #E8A544 on void #12100E)</text>",
        ship, OUTFIT.version
    );

    for (i, f) in show.iter().enumerate() {
        let ox = (i as u32) * (fw + gap);
        let oy = 24u32;
        for y in 0..h {
            for x in 0..w {
                let c = f.cells[y * w + x];
                let rgb = c & 0xFFFFFF;
                if rgb == void {
                    continue;
                }
                // brighten alpha a touch for the dark bg
                let r = (rgb >> 16) as u8;
                let g = (rgb >> 8) as u8;
                let b = rgb as u8;
                println!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"rgb({},{},{})\"/>",
                    ox + x as u32 * cell,
                    oy + y as u32 * cell,
                    cell,
                    cell,
                    r,
                    g,
                    b
                );
            }
        }
    }
    println!("</svg>");
}
