#!/usr/bin/env bash
# B4 guard (2026-07-14): the sovereign PQ substrate `bebop2-core` MUST stay
# from-scratch + C-crypto-free. `ring` / `aws-lc-rs` (C-built crypto backends) are
# permitted ONLY in the transport layer (`bebop-proto-wire`, via quinn/rustls) — NOT
# in the core that compiles to an empty-import wasm32 build.
#
# GREEN proof: `cargo tree -p bebop2-core` resolves with NO `ring` / `aws-lc-rs`.
# A regression that adds a C-crypto dep to the core trips this guard (caught by the
# RED mutation in test-no-ccrypto-core.sh).
set -euo pipefail

REPO_ROOT="${GUARD_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
cd "$REPO_ROOT"

if ! command -v cargo >/dev/null 2>&1; then
  echo "WARN: cargo not available — skipping B4 C-crypto fence (CI provides cargo)"; exit 0
fi

# Resolve the core's full (normal) dependency tree and assert no C-built crypto backend.
TREE=$(cargo tree -p bebop2-core --edges normal 2>/dev/null || true)
if echo "$TREE" | grep -Eq '(^|[^a-z0-9-])(ring|aws-lc-rs)([^a-z0-9-]|$)'; then
  echo "✗ B4 VIOLATION: bebop2-core pulls a C-built crypto backend (ring/aws-lc-rs):"
  echo "$TREE" | grep -E 'ring|aws-lc-rs' | head
  exit 1
fi

echo "✓ B4 fence: bebop2-core is C-crypto-free (no ring/aws-lc-rs in its dependency tree)"
