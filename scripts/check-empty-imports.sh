#!/usr/bin/env bash
# check-empty-imports.sh — REAL empty-import property gate for bebop2-core.
#
# Per REMEDIATION-BLUEPRINT-2026-07-12.md §3G, the empty-import integrity gate
# must (a) build bebop2-core for wasm32-unknown-unknown in RELEASE, (b) parse the
# produced .wasm IMPORT SECTION with `wasm-tools` (not grep), and (c) FAIL-CLOSED:
# if the tool / target is missing the script ERRORS — it never silently exits 0.
# The old `bebop2/core/scripts/check-wasm32.sh` had the hole of `exit 0` when
# wasm-tools was missing and grepped the *debug* artifact; this replaces it.
#
# GREEN: the real bebop2-core wasm is built with --no-default-features (host off,
#        std off) and has ZERO imports -> exits 0.
# RED:   scripts/fixtures/bad-imports.wat is compiled and MUST be rejected
#        (non-zero exit). This proves the gate is property-complete, not a
#        label-gate: it can actually fail on a module that imports a host fn.
set -euo pipefail

cd "$(dirname "$0")/.."
ROOT="$(pwd)"
TARGET=wasm32-unknown-unknown
CRATE=bebop2-core
WASM="target/$TARGET/release/bebop2_core.wasm"
RED_WAT="$ROOT/scripts/fixtures/bad-imports.wat"
RED_WASM="$ROOT/scripts/fixtures/bad-imports.wasm"

# ── 0. Tooling preconditions (FAIL-CLOSED) ────────────────────────────────────
if ! command -v wasm-tools >/dev/null 2>&1; then
    echo "FAIL-CLOSED: wasm-tools not installed. Install with: cargo install wasm-tools" >&2
    exit 2
fi
if ! rustup target list --installed 2>/dev/null | grep -q "$TARGET"; then
    echo "wasm32-unknown-unknown target not installed — attempting install..." >&2
    rustup target add "$TARGET" || {
        echo "FAIL-CLOSED: could not install $TARGET target" >&2
        exit 2
    }
fi

# ── helper: count imports in a wasm module by parsing the binary ──────────────
# Walks sections; an Import section (id == 2) leads with a u32 LEB128 count of
# import entries. This is byte-exact and does not depend on wasm-tools' textual
# dump format. (Mirrors the approach in scripts/verify-empty-imports.sh.)
count_imports() {
    python3 - "$1" <<'PY'
import sys
def leb(b, p):
    r = 0; s = 0
    while True:
        x = b[p]; p += 1; r |= (x & 0x7f) << s
        if x & 0x80 == 0: break
        s += 7
    return r, p
data = open(sys.argv[1], 'rb').read()
assert data[:4] == b'\x00asm', 'not a wasm module'
pos = 8; n = 0
while pos < len(data):
    sid = data[pos]; pos += 1
    size, pos = leb(data, pos); end = pos + size
    if sid == 2:  # Import section
        n, _ = leb(data, pos)
    pos = end
print(n)
PY
}

# ── 1. GREEN: build the real core for wasm32 release, assert 0 imports ─────────
echo "==> [empty-import] build $CRATE for $TARGET (release, --no-default-features)"
cargo build -p "$CRATE" --target "$TARGET" --release --no-default-features 2>&1 | tail -2

if [[ ! -f "$WASM" ]]; then
    echo "FAIL-CLOSED: $WASM was not produced by the build" >&2
    exit 1
fi

GREEN_IMPORTS=$(count_imports "$WASM")
if [[ "$GREEN_IMPORTS" -ne 0 ]]; then
    echo "FAIL: $CRATE wasm has $GREEN_IMPORTS import(s) — not sovereign:" >&2
    wasm-tools dump "$WASM" | grep '^ *import ' >&2 || true
    exit 1
fi
echo "    $CRATE wasm imports: 0 (empty import section) ✓"

# ── 2. RED: compile the bad-imports fixture and assert the gate REJECTS it ─────
echo "==> [empty-import] RED leg: build fixtures/bad-imports.wat and expect rejection"
wasm-tools parse "$RED_WAT" -o "$RED_WASM" 2>/dev/null
RED_IMPORTS=$(count_imports "$RED_WASM")
if [[ "$RED_IMPORTS" -eq 0 ]]; then
    echo "RED LEG FAILED: fixture unexpectedly has 0 imports — gate is broken!" >&2
    exit 1
fi
echo "    RED fixture imports: $RED_IMPORTS (host fn) — correctly rejected by gate ✓"

echo "==> [empty-import] GATE PASSED (real core = 0 imports, RED fixture rejected)"
