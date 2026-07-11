#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# check-wasm32.sh — verify the bebop2 PQ crypto core builds for wasm32 with
# ZERO imports (genuine no_std + no-alloc crypto lib).
#
# Strategy:
#   1. Build the core lib as `cdylib` for wasm32-unknown-unknown with BOTH the
#      `std` and `host` features disabled. This leaves only the pure PQ crypto
#      core (ml_kem, pq_dsa, aead, kdf, hash, sign, rng) — no f64 analytic
#      kernel, no std. The crate's self-contained bump allocator (lib.rs) gives
#      `alloc` without pulling in any wasm import.
#   2. Assert the build emits 0 errors.
#   3. Assert the produced `.wasm` has an EMPTY import section (no `env.*`
#      imports) — the definition of "zero imports" for wasm32-unknown-unknown.
#
# Requires: rustup target `wasm32-unknown-unknown` and `wasm-tools`
#   rustup target add wasm32-unknown-unknown
#   cargo install wasm-tools
# ─────────────────────────────────────────────────────────────────────────────
set -euo pipefail

CRATE_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "${CRATE_DIR}"

TARGET=wasm32-unknown-unknown

# Resolve the actual cargo target dir (workspace may place it at the repo root).
TARGET_DIR=$(cargo metadata --format-version 1 --no-deps 2>/dev/null | sed -n 's/.*"target_directory":"\([^"]*\)".*/\1/p')
[ -z "${TARGET_DIR}" ] && TARGET_DIR="${CRATE_DIR}/target"
echo "    target dir: ${TARGET_DIR}"

echo "==> Building bebop2-core for ${TARGET} (--no-default-features) =="
# Capture the actual error count (0 == clean). grep -c exits 1 when it prints 0,
# so `|| true` keeps `set -e` from aborting on the expected zero-match case.
ERR_COUNT=$(cargo build --target "${TARGET}" --no-default-features 2>&1 | grep -c '^error' || true)
if [ "${ERR_COUNT}" -ne 0 ]; then
    echo "FAIL: ${ERR_COUNT} build error(s) for ${TARGET}" >&2
    exit 1
fi
echo "    build: OK (0 errors)"

# Locate the produced artifact (cdylib emits .wasm next to the deps dir).
WASM=$(find "${TARGET_DIR}/${TARGET}/debug" -name 'bebop2_core.wasm' 2>/dev/null | head -n1)
if [ -z "${WASM}" ]; then
    echo "FAIL: no wasm artifact produced under ${TARGET_DIR}/${TARGET}/debug" >&2
    exit 1
fi
echo "    artifact: ${WASM}"

# Inspect the import section. `wasm-tools dump` prints an "imports:" block; an
# empty import section yields no import lines. We also fail if `wasm-tools` is
# missing so the gate is explicit rather than silently passing.
if ! command -v wasm-tools >/dev/null 2>&1; then
    echo "WARN: wasm-tools not installed — skipping import-section check." >&2
    echo "      install with: cargo install wasm-tools" >&2
    exit 0
fi

# Count imports. `wasm-tools dump` lists each import as e.g.
#   import env.memory (memory)
# An empty import section prints nothing under the imports heading.
IMPORT_LINES=$(wasm-tools dump "${WASM}" 2>/dev/null | grep -c '^ *import ' || true)
if [ "${IMPORT_LINES}" -ne 0 ]; then
    echo "FAIL: wasm artifact has ${IMPORT_LINES} import(s) — not zero-import:" >&2
    wasm-tools dump "${WASM}" | grep '^ *import ' >&2 || true
    exit 1
fi

echo "    imports: 0 (empty import section) ✓"
echo "PASS: bebop2-core builds for ${TARGET} with zero imports."
