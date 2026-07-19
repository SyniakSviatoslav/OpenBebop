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

# A gate counts ONLY when a `#[cfg(any(...))]` carrying all three required predicates
# (test, dangerous_deterministic, test_keygen) attaches DIRECTLY to the target `pub fn` —
# i.e. nothing but further attributes / doc-comments / blank lines separate the cfg's
# closing `))]` from `pub fn NAME(`. This defeats the token-scatter bypass (a nearby but
# UNrelated gated item cannot launder an actually-ungated keygen). `ceremony` etc. are
# allowed as extra predicates. REJECTED: a bare ungated `pub fn`, or a cfg separated from
# the fn by real code.
check_gated() {
  local file="$1" fn="$2" label="$3"
  if ! FN="$fn" perl -0777 -ne '
      my $fn = quotemeta($ENV{FN});
      # cfg block (single- or multi-line) → only attrs/doc-comments/blank → pub fn $fn(
      while (/\#\[cfg\(any\((.*?)\)\)\]\s*(?:(?:\#\[.*?\]|\/{2,3}[^\n]*)\s*)*pub\s+fn\s+'"$fn"'\s*\(/gs) {
        my $g = $1;
        exit 0 if $g =~ /\btest\b/ && $g =~ /dangerous_deterministic/ && $g =~ /test_keygen/;
      }
      exit 1;
    ' "$file"; then
    echo "✗ C3 VIOLATION: $label is not gated behind dangerous_deterministic/test_keygen"
    exit 1
  fi
}

# pq_dsa::keygen must be gated (accepts the stricter multi-line +ceremony form).
check_gated "$DSA" "keygen" "pq_dsa::keygen"
# pq_kem::keygen_internal must be gated.
check_gated "$KEM" "keygen_internal" "pq_kem::keygen_internal"

echo "✓ C3 gate present: constant-seed keygen gated off in production (pq_dsa::keygen + pq_kem::keygen_internal)"
