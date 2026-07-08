#!/usr/bin/env bash
# Bebop Sovereign Node — Phase 2: build the core for WASI (wasm32-wasi) to run under WasmEdge.
#
# WHY: the deterministic kernel (decide/embed/similarity) is the part that must be bit-identical and
# tamper-evident. Running it under WasmEdge (AOT-compiled, MB-sized, sub-ms cold start, proven
# sandbox) gives the Sovereign Node a hardened runtime for the core WITHOUT touching the LLM periphery
# (Ollama stays native — WASI-NN is an optional later upgrade). This matches the project's own rule:
# the cheapest token is the one you never send; the deterministic core must never depend on an LLM.
#
# Requires: rustup target add wasm32-wasi
set -euo pipefail
cd "$(dirname "$0")"

TARGET="${1:-wasm32-wasi}"
rustup target add "$TARGET" 2>/dev/null || true

# Build the kernel for WASI. The crate is already wasm32-clean (no OS sockets/fs/threads/entropy in
# the core — same invariant that makes the wasm32-unknown-unknown build pass).
cargo build --release --target "$TARGET"

mkdir -p ../../dist
SRC="target/$TARGET/release/bebop_core.wasm"
[ -f "$SRC" ] || SRC="target/$TARGET/release/bebop_core.wasm"
cp "$SRC" ../../dist/bebop_core.wasi.wasm

# AOT-compile with WasmEdge (if available) for near-native speed + tiny footprint.
if command -v wasmedge >/dev/null 2>&1; then
  wasmedge compile ../../dist/bebop_core.wasi.wasm ../../dist/bebop_core.wasi.aot.wasm \
    && echo "✓ AOT-compiled: ../../dist/bebop_core.wasi.aot.wasm"
else
  echo "⚠ wasmedge not installed; shipping uncompiled .wasi.wasm (run with: wasmedge dist/bebop_core.wasi.wasm)"
fi
echo "✓ wrote ../../dist/bebop_core.wasi.wasm ($(wc -c < ../../dist/bebop_core.wasi.wasm) bytes)"
