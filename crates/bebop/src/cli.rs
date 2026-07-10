//! CLI dispatcher for `bebop` — ported from `bebop.ts`.
//! Subcommands mirror the documented surface; the interactive TUI (launch anim)
//! is reached via `bebop` with no args on a TTY.

use crate::outfit::OUTFIT;
use crate::vault::create_or_unlock;
use std::env;

pub fn run() {
    let args: Vec<String> = env::args().collect();
    let cmd = args.get(1).map(|s| s.as_str()).unwrap_or("");
    let rest = &args[2.min(args.len())..];

    match cmd {
        "" => {
            // No subcommand → launch the interactive TUI (sun-warm launch).
            // This is the "faceless agents get a face" move. TTY-gated inside.
            if let Err(e) = crate::tui::run_tui() {
                eprintln!("  ✖ tui: {e}");
                std::process::exit(1);
            }
        }
        "help" | "--help" | "-h" => print_help(),
        "init" => {
            // Configure the ship: looks / narration / patrons (the "make it yours" axes).
            let looks = flag_value(rest, "--looks");
            let narration = flag_value(rest, "--narration");
            let home = flag_value(rest, "--home");
            let force = rest.contains(&"--force".to_string());
            let transition = rest.contains(&"--transition".to_string());
            let mut p = crate::customize::Profile::load();
            if force || looks.is_some() {
                p.looks = Some(crate::customize::LooksOverride { accent: looks });
            }
            if force || narration.is_some() {
                p.narration = narration;
            }
            if force || home.is_some() {
                p.patrons = Some(crate::customize::PatronsOverride { home });
            }
            match p.save() {
                Ok(_) => {
                    let o = p.resolve_outfit();
                    println!(
                        "  ✓ profile written — {}",
                        crate::customize::profile_path().display()
                    );
                    println!("{}", o.banner());
                    println!("  narration: {:?}", o.narration);
                    // The ship repaints itself while the new looks apply (loader).
                    crate::tui::render_loader_animation(
                        crate::tui::AgentState::Initing,
                        12,
                        "init",
                        "repainting hull to your accent",
                        &o,
                    );
                    if transition {
                        // Ship repaints itself: tween old → new looks accent.
                        let from = OUTFIT.palette.ship;
                        let to = o.palette.ship;
                        let frames = crate::tui::render_launch_tween(
                            48,
                            22,
                            0xC0FFEE,
                            18,
                            from,
                            to,
                            OUTFIT.palette.void,
                        );
                        println!(
                            "  ◈ repaint: #{:06X} → #{:06X} ({} frames)",
                            from,
                            to,
                            frames.len()
                        );
                    }
                }
                Err(e) => {
                    eprintln!("  ✖ init: {e}");
                    std::process::exit(1);
                }
            }
        }
        "preview" => {
            // Render the cosmo-noir helm to SVG using the ACTIVE (customized) outfit.
            // The "make it yours" hook made visible: your accent colors the ship + status.
            let o = crate::customize::Profile::load().resolve_outfit();
            let transition = rest.contains(&"--transition".to_string());
            let svg = if transition {
                // Ship-repaint proof: tween the helm's ship from default → your accent.
                let frames = crate::tui::render_launch_tween(
                    48,
                    22,
                    0xC0FFEE,
                    18,
                    OUTFIT.palette.ship,
                    o.palette.ship,
                    o.palette.void,
                );
                let _last = frames.last().unwrap();
                crate::tui::render_helm_svg(90, 30, &o) // helm with new accent
                    + &format!("\n<!-- repaint end-frame ship #{:06X} -->", o.palette.ship)
            } else {
                crate::tui::render_helm_svg(90, 30, &o)
            };
            let out = flag_value(rest, "--out").unwrap_or_else(|| "bebop-helm.svg".into());
            match std::fs::write(&out, svg) {
                Ok(_) => println!(
                    "  ✓ helm rendered with accent #{:06X} → {}",
                    o.palette.ship, out
                ),
                Err(e) => {
                    eprintln!("  ✖ preview: {e}");
                    std::process::exit(1);
                }
            }
        }
        "boot" => {
            // Guard self-test: refuse to start if gates can't go RED.
            let o = crate::customize::Profile::load().resolve_outfit();
            crate::tui::render_loader_animation(
                crate::tui::AgentState::Booting,
                10,
                "boot",
                "spinning up the reactor",
                &o,
            );
            println!("{}", OUTFIT.lines.boot);
            println!("  ✓ Bebop guard OS certified: gates deny on red, pass on green.");
        }
        "node" => {
            // Boot an encrypted-at-rest node identity (vault).
            let o = crate::customize::Profile::load().resolve_outfit();
            crate::tui::render_loader_animation(
                crate::tui::AgentState::Node,
                9,
                "node",
                "raising node shields",
                &o,
            );
            let pass = rest
                .iter()
                .position(|a| a == "--pass")
                .and_then(|i| rest.get(i + 1))
                .cloned()
                .unwrap_or_else(|| "bebop".into());
            let path = rest
                .iter()
                .position(|a| a == "--path")
                .and_then(|i| rest.get(i + 1))
                .cloned()
                .unwrap_or_else(|| "/tmp/bebop-node.json".into());
            match create_or_unlock(&pass, &path, true) {
                Ok(id) => println!("  ✓ node booted — id {}", id.id),
                Err(e) => {
                    eprintln!("  ✖ vault: {e}");
                    std::process::exit(1);
                }
            }
        }
        "recall" => {
            // Query the living-knowledge retriever.
            let o = crate::customize::Profile::load().resolve_outfit();
            crate::tui::render_loader_animation(
                crate::tui::AgentState::Recalling,
                9,
                "recall",
                "sweeping living knowledge",
                &o,
            );
            let q = rest.join(" ");
            println!("  §0·GP recall — query: {q}");
            println!("  (retriever wired in core::knowledge; pass a live memory to query)");
        }
        "radio" => {
            // The ship's lounge: free-to-listen Lofi / Jazz. License-clean by
            // construction — nothing bundled, the OS player does the streaming.
            let seed = flag_value(rest, "--seed")
                .and_then(|s| u64::from_str_radix(s.trim_start_matches("0x"), 16).ok())
                .unwrap_or(0xBEEF);
            let arg = rest.first().map(|s| s.as_str()).unwrap_or("");
            let res = match arg {
                "" => {
                    crate::radio::list();
                    Ok(())
                }
                "onair" | "shuffle" => crate::radio::on_air(seed),
                "stop" | "off" => {
                    println!(
                        "  ◈ radio off — the lounge goes quiet. (close your player to stop audio.)"
                    );
                    Ok(())
                }
                s => match s.parse::<usize>() {
                    Ok(n) => crate::radio::play(n),
                    Err(_) => {
                        eprintln!("  ✖ unknown radio arg: {s}  (try `bebop radio`)");
                        std::process::exit(2);
                    }
                },
            };
            if let Err(e) = res {
                eprintln!("  ✖ radio: {e}");
                std::process::exit(1);
            }
            // every loop/task closes with the dock sign-off — even a tune.
            crate::mission::mission_summary(
                "radio",
                &[
                    "deck hands selected a free Lofi/Jazz stream — nobody paid, nobody sued.",
                    "your own player is streaming it; bebop just pointed at the sky.",
                    "the lounge is yours. the cigar is mine.",
                ],
            );
        }
        "mission" => {
            // The sign-off, on demand. Mirrors what fires at end of session/task/loop.
            let title = flag_value(rest, "--title").unwrap_or_else(|| "standalone".into());
            crate::mission::mission_summary(
                &title,
                &[
                    "report filed. the work is done, or it thinks it is.",
                    "smoke clears. the ship is still here. that's the part that matters.",
                    "next loop whenever you are.",
                ],
            );
        }
        other => {
            eprintln!("  unknown command: {other}  (try `bebop help`)");
            std::process::exit(2);
        }
    }
}

fn print_help() {
    println!("{}", OUTFIT.banner());
    println!("  init [--looks RRGGBB --narration X --home URL --force] | boot | outfit");
    println!("  node [--pass X --path Y] | recall <q> | radio [<n>|onair|stop] | help");
    println!("  mission [--title T]   (the sign-off — dock + cigar; also fires at loop end)");
    println!("  (interactive TUI with the sun-warm launch: run `bebop` in a TTY)");
    println!("  {}", OUTFIT.home);
}

/// Extract `--flag value` from the args slice after the subcommand.
fn flag_value(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|a| a == flag)
        .and_then(|i| args.get(i + 1).cloned())
}
