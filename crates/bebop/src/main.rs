//! Bebop native binary entry point. The real logic lives in the `bebop` lib
//! (so it is shared with the wasm build and unit-tested in one place).

fn main() {
    bebop::cli::run();
}
