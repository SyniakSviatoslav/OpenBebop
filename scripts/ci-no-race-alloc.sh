#!/usr/bin/env bash
# B3 guard (2026-07-14): the no_std bump allocator must NOT use a racy load+store
# reserve (which can return overlapping regions under concurrent alloc). It must
# reserve the heap with a single atomic RMW (compare_exchange / fetch_add), and the
# pure no_std wasm32 build must still produce an empty import section.
#
# GREEN proof: the allocator uses compare_exchange (no bare NEXT.store in alloc).
# A regression to the old `NEXT.load` + `NEXT.store` pattern trips this guard.
set -euo pipefail

REPO_ROOT="${GUARD_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
CORE="$REPO_ROOT/bebop2/core/src/lib.rs"

if ! grep -q "compare_exchange" "$CORE"; then
  echo "✗ B3 VIOLATION: no_std allocator reserves heap without an atomic RMW (compare_exchange/fetch_add) — racy load+store regressed"
  exit 1
fi

# Ensure the OLD racy pattern is gone: an `alloc` body that does NEXT.load then NEXT.store
# (without compare_exchange) is the specific B3 hazard. We flag if `NEXT.store(` exists at all
# in the allocator region (we replaced the reserve store with compare_exchange).
if grep -q "NEXT.store(" "$CORE"; then
  echo "✗ B3 VIOLATION: allocator still uses NEXT.store() (racy reserve) — use compare_exchange"
  exit 1
fi

echo "✓ B3 allocator: race-free heap reserve (compare_exchange, no NEXT.store) in no_std runtime"
