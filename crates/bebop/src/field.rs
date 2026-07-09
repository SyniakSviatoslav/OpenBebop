//! Field core re-export (native target). The deterministic graph-PDE field lives
//! in `rust-core/` (dependency-free, air-gapped). For the native CLI we link it
//! as a path dependency so the SAME code runs on the metal and in WASM.
//! (The wasm crate stays `#[no_std]`-free-of-deps; this re-export is the
//! host-side handle.)

pub use bebop_core::*;
