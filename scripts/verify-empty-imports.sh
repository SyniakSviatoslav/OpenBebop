#!/usr/bin/env bash
# verify-empty-imports.sh — the sovereign-core ("machine-code / bare-metal") gate.
#
# ROOT-CAUSE THIS FIXES: the 2026-07-11 audit claimed `bebop2-core` wasm32/no_std/
# empty-import gate FAILS (~94 errors). That was STALE — the crate already gates
# `#![no_std]` on `not(feature = "std")`, ships a bump `#[global_allocator]` +
# `#[panic_handler]`, and (with `host` off) builds to wasm32 with ZERO imports.
#
# This script turns that claim into a FALSIFIABLE CI gate instead of prose:
# it builds bebop2-core for wasm32 with --no-default-features (host off, std off)
# and RED-fails if the produced module has ANY import (clock/RNG/socket/syscall).
# An empty import section = nothing outside the module is reachable = sovereign.
#
# Used by `bebop docs check` and CI. Not a pre-commit hook (slow wasm build).

set -euo pipefail
cd "$(dirname "$0")/.."

TARGET=wasm32-unknown-unknown
WASM="target/$TARGET/debug/bebop2_core.wasm"

echo "== sovereign-core gate: bebop2-core wasm32 (no_std, host off) =="
cargo build -p bebop2-core --target "$TARGET" --no-default-features 2>&1 | tail -1

if [[ ! -f "$WASM" ]]; then
  echo "✗ gate FAILED: $WASM not produced"
  exit 1
fi

# Parse the wasm binary: section id 2 == Import. Count entries (LEB128).
python3 - "$WASM" <<'PY'
import sys, struct
data = open(sys.argv[1],'rb').read()
assert data[:4]==b'\x00asm', 'not wasm'
def leb(b,p):
    r=0;s=0
    while True:
        x=b[p];p+=1;r|=(x&0x7f)<<s
        if x&0x80==0:break
        s+=7
    return r,p
pos=8;n=0
while pos<len(data):
    sid=data[pos];pos+=1
    size,pos=leb(data,pos);end=pos+size
    if sid==2:
        n,_=leb(data,pos)
    pos=end
print(f"import section entries: {n}")
if n!=0:
    print("✗ gate FAILED: module has imports (not sovereign -> clock/RNG/socket reachable)")
    sys.exit(1)
PY

echo "✓ sovereign-core gate PASSED: empty import section (0 imports) — bare-metal-safe."
