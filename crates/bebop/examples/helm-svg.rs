//! Example: render the cosmo-noir helm to SVG (design proof) or text (inspect).
//! Honors the active profile's `looks` accent via `Profile::load().resolve_outfit()`.
//! Run:
//!   cargo run --example helm-svg --release                 > docs/design/bebop-helm.svg
//!   cargo run --example helm-svg --release text 90 30      # print all tabs as text
use bebop::customize::Profile;
use bebop::tui::{debug_helm_text, render_helm_svg};
use std::env;

fn main() {
    let o = Profile::load().resolve_outfit();
    let args: Vec<String> = env::args().collect();
    if args.get(1).map(|s| s.as_str()) == Some("text") {
        let w: u16 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(90);
        let h: u16 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(30);
        for tab in 0..4 {
            println!("===== TAB {tab} =====");
            println!("{}", debug_helm_text(w, h, tab, &o));
        }
        return;
    }
    print!("{}", render_helm_svg(90, 30, &o));
}
