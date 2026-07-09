#!/usr/bin/env bash
# Bebop Sovereign Node — Phase 2: build the core for WASI to run under WasmEdge.
#
# WHY: the deterministic kernel (decide/embed/similarity) is the part that must be bit-identical and
# tamper-evident. Running it under WasmEdge (AOT-compiled, MB-sized, sub-ms cold start, proven
# sandbox) gives the Sovereign Node a hardened runtime for the core WITHOUT touching the LLM periphery
# (Ollama stays native — WASI-NN is an optional later upgrade). This matches the project's own rule:
# the cheapest token is the one you never send; the deterministic core must never depend on an LLM.
#
# Rust retired the "wasm32-wasi" alias in 1.87+; this script auto-selects a valid WASI target
# (wasm32-wasip1 preferred; wasm32-wasip2 if unavailable).
set -euo pipefail
cd "$(dirname "$0")"

pick_wasi() {
  for t in wasm32-wasip1 wasm32-wasip2 wasm32-wasi; do
    if rustc --print target-list 2>/dev/null | grep -qx "$t"; then echo "$t"; return; fi
  done
  echo "wasm32-wasip1"  # final fallback; rustup add will fetch it
}

TARGET="$(pick_wasi)"
rustup target add "$TARGET" 2>/dev/null || true

cargo build --release --target "$TARGET"

mkdir -p ../../dist
SRC="target/$TARGET/release/bebop_core.wasm"
if [ ! -f "$SRC" ]; then
  echo "✗ expected $SRC not found" >&2
  exit 1
fi
cp "$SRC" ../../dist/bebop_core.wasi.wasm

# AOT-compile with WasmEdge (if available) for near-native speed + tiny footprint.
if command -v wasmedge >/dev/null 2>&1; then
  wasmedge compile ../../dist/bebop_core.wasi.wasm ../../dist/bebop_core.wasi.aot.wasm \
    && echo "✓ AOT-compiled: ../../dist/bebop_core.wasi.aot.wasm"
else
  echo "⚠ wasmedge not installed; shipping uncompiled .wasi.wasm (run with: wasmedge dist/bebop_core.wasi.wasm)"
fi
echo "✓ wrote ../../dist/bebop_core.wasi.wasm ($(wc -c < ../../dist/bebop_core.wasi.wasm) bytes)"
