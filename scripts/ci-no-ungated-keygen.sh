#!/usr/bin/env bash
# C3 guard (2026-07-14): constant-seed keygen must be GATED off in production.
# `pq_dsa::keygen` and `pq_kem::keygen_internal` (arbitrary-seed minting) must be
# preceded by the `#[cfg(any(test, feature = "dangerous_deterministic",
# feature = "test_keygen"))]` gate — the same gate `sign::keygen` already carries.
# A normal (feature-off, non-test) build must NOT be able to mint a key from an
# arbitrary 32-byte seed. The legitimate prod path uses `keygen_derivable` /
# `keygen_internal_prod` instead.
set -euo pipefail

REPO_ROOT="${GUARD_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
DSA="$REPO_ROOT/bebop2/core/src/pq_dsa.rs"
KEM="$REPO_ROOT/bebop2/core/src/pq_kem.rs"

GATE='#\[cfg\(any\(test, feature = "dangerous_deterministic", feature = "test_keygen"\)\)\]'

# pq_dsa::keygen must be gated.
if ! grep -Pzo "(?s)$GATE\npub fn keygen\(" "$DSA" >/dev/null 2>&1; then
  echo "✗ C3 VIOLATION: pq_dsa::keygen is not gated behind dangerous_deterministic/test_keygen"
  exit 1
fi

# pq_kem::keygen_internal must be gated.
if ! grep -Pzo "(?s)$GATE\npub fn keygen_internal\(" "$KEM" >/dev/null 2>&1; then
  echo "✗ C3 VIOLATION: pq_kem::keygen_internal is not gated behind dangerous_deterministic/test_keygen"
  exit 1
fi

echo "✓ C3 gate present: constant-seed keygen gated off in production (pq_dsa::keygen + pq_kem::keygen_internal)"
