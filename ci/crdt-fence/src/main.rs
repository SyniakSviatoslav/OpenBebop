//! CLI entrypoint for the MESH-08 CRDT-periphery compile-fence.
//!
//! Usage:
//!   cargo run -p ci-crdt-fence            # runs `cargo metadata` on the cwd workspace
//!   cargo run -p ci-crdt-fence -- --metadata FILE.json   # use a pre-baked metadata file
//!
//! Exit code 0 => the order/money dependency graph is clean (no CRDT reach).
//! Exit code 1 => some guarded crate transitively depends on a CRDT-merge crate.

use std::process::Command;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let metadata = match parse_metadata_arg(&args) {
        Some(path) => std::fs::read_to_string(&path)
            .unwrap_or_else(|e| die(&format!("cannot read {path}: {e}"))),
        None => run_cargo_metadata(),
    };

    match ci_crdt_fence::find_offenses(&metadata) {
        Ok(offenses) if offenses.is_empty() => {
            println!("[crdt-fence] OK — no CRDT-merge crate reachable from guarded order/money crates.");
            std::process::exit(0);
        }
        Ok(offenses) => {
            eprintln!("[crdt-fence] FAIL — order/money crate reaches a forbidden CRDT-merge crate:");
            for o in &offenses {
                eprintln!(
                    "  {} -> {}   path: {}",
                    o.guarded,
                    o.crdt,
                    o.path.join(" -> ")
                );
            }
            eprintln!(
                "[crdt-fence] MESH-08 invariant violated: money/order state must NEVER depend on a CRDT-merge crate."
            );
            std::process::exit(1);
        }
        Err(e) => die(&format!("metadata parse error: {e}")),
    }
}

fn parse_metadata_arg(args: &[String]) -> Option<String> {
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--metadata" && i + 1 < args.len() {
            return Some(args[i + 1].clone());
        }
        i += 1;
    }
    None
}

fn run_cargo_metadata() -> String {
    let out = Command::new("cargo")
        .args(["metadata", "--format-version=1", "--all-features"])
        .output()
        .unwrap_or_else(|e| die(&format!("failed to spawn `cargo metadata`: {e}")));
    if !out.status.success() {
        die(&format!(
            "`cargo metadata` exited {}:\n{}",
            out.status,
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    // We deliberately use the FULL resolve graph (no `--no-deps`): an injected
    // registry CRDT crate would otherwise be dropped from the dependency edges
    // and the lint would silently pass. `--all-features` guarantees feature
    // unification can't hide a dependency behind an unenabled feature.
    String::from_utf8_lossy(&out.stdout).to_string()
}

fn die(msg: &str) -> ! {
    eprintln!("[crdt-fence] ERROR: {msg}");
    std::process::exit(2);
}
