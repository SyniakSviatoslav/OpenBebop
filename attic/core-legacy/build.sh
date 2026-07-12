#!/usr/bin/env bash
# Build the bebop-core WASM kernel and copy the artifact next to the TS loader.
# Requires: rustup target add wasm32-unknown-unknown  (already present on CI/dev).
set -euo pipefail
cd "$(dirname "$0")"
cargo build --release --target wasm32-unknown-unknown
mkdir -p ../../src
cp target/wasm32-unknown-unknown/release/bebop_core.wasm ../../src/bebop_core.wasm
echo "✓ wrote ../../src/bebop_core.wasm ($(wc -c < ../../src/bebop_core.wasm) bytes)"
