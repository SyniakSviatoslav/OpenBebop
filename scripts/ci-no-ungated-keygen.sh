#!/usr/bin/env bash
# C3 guard (2026-07-14): constant-seed keygen must be GATED off in production.
# `pq_dsa::keygen` and `pq_kem::keygen_internal` (arbitrary-seed minting) must be
# preceded by a `#[cfg(any(test, feature = "dangerous_deterministic",
# feature = "test_keygen", ...))]` gate — the same gate `sign::keygen` already
# carries. A normal (feature-off, non-test) build must NOT be able to mint a key
# from an arbitrary 32-byte seed. The legitimate prod path uses `keygen_derivable`
# / `keygen_internal_prod` instead.
#
# The gate attribute is matched structurally (extract the `#[cfg(any(...))]`
# block immediately preceding the fn, then check it for the required
# predicates as substrings) rather than as one fixed-string line, because the
# real gate is pretty-printed across multiple lines and also carries an
# operator-ceremony predicate (`feature = "ceremony"`) that the guard does not
# require but must tolerate. Extra/optional predicates are fine; the three
# baseline ones below are mandatory.
set -euo pipefail

REPO_ROOT="${GUARD_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
DSA="$REPO_ROOT/bebop2/core/src/pq_dsa.rs"
KEM="$REPO_ROOT/bebop2/core/src/pq_kem.rs"

# Extract the `#[cfg(any( ... ))]` attribute block that immediately precedes
# `pub fn <fn>(` in $1, whitespace-tolerant, spanning any number of lines.
# `[^)]*?` (rather than `.*?`) cannot cross a `)` character, so it cannot skip
# over unrelated code/attributes elsewhere in the file to falsely "reach" the
# target fn — it can only match the immediate, unbroken `any(...)` predicate
# list, which contains no `)` characters of its own. Empty output means: no
# such attribute directly precedes the fn (ungated, or gated by something
# else) — the caller treats that as a violation.
extract_gate_block() {
  local file="$1" fn="$2"
  grep -Pzo "(?s)#\[cfg\(any\([^)]*?\)\)\]\s*\npub fn ${fn}\(" "$file" 2>/dev/null | tr -d '\0'
}

# Require a bare `test` predicate (not merely a substring of `test_keygen`).
has_bare_test() {
  grep -Pq '(?<![A-Za-z0-9_])test(?![A-Za-z0-9_])' <<<"$1"
}

check_gated() {
  local file="$1" fn="$2" label="$3"
  local block
  block="$(extract_gate_block "$file" "$fn")"
  if [[ -z "$block" ]]; then
    echo "✗ C3 VIOLATION: $label is not gated behind dangerous_deterministic/test_keygen"
    exit 1
  fi
  if ! has_bare_test "$block"; then
    echo "✗ C3 VIOLATION: $label gate is missing required predicate: test"
    exit 1
  fi
  local required
  for required in 'feature = "dangerous_deterministic"' 'feature = "test_keygen"'; do
    if ! grep -qF -- "$required" <<<"$block"; then
      echo "✗ C3 VIOLATION: $label gate is missing required predicate: $required"
      exit 1
    fi
  done
}

# pq_dsa::keygen must be gated.
check_gated "$DSA" "keygen" "pq_dsa::keygen"

# pq_kem::keygen_internal must be gated.
check_gated "$KEM" "keygen_internal" "pq_kem::keygen_internal"

echo "✓ C3 gate present: constant-seed keygen gated off in production (pq_dsa::keygen + pq_kem::keygen_internal)"
