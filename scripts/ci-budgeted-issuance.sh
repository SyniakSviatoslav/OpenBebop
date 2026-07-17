#!/usr/bin/env bash
# CI GUARD — BUDGETED-ISSUANCE SEAM (Layer D / P-D consensus-capability, Option A).
#
# The budgeted-issuance production code in bebop2/proto-cap/src/node_id.rs MUST
# route ALL Ed25519 delegation signing through the single seam
# `sign_delegation_budgeted`. A bare `Delegation::sign(` call OUTSIDE that seam
# (and outside #[cfg(test)] modules) is a budget bypass — it mints authority
# with no per-epoch cap enforcement. This guard locks that bulkhead.
#
# RED-proven: inserting a bare `Delegation::sign(` in non-test, non-seam
# production code makes this script exit non-zero (FAIL). The seam is delimited
# by the comments `BUDGETED-ISSUANCE-SEAM-BEGIN` / `BUDGETED-ISSUANCE-SEAM-END`
# around `sign_delegation_budgeted`; #[cfg(test)] modules are exempt.
set -uo pipefail
cd "$(dirname "$0")/.."

TARGET=bebop2/proto-cap/src/node_id.rs
if [ ! -f "$TARGET" ]; then
  echo "SKIP: $TARGET not present"
  exit 0
fi

if awk '
  # Seam / test markers are checked on the RAW line (they live in comments).
  /^[[:space:]]*#\[cfg\(test\)\]/ { in_test = 1 }
  in_test == 1 { next }                              # test modules are exempt
  /^[[:space:]]*\/\/[[:space:]]*BUDGETED-ISSUANCE-SEAM-BEGIN/ { in_seam = 1; next }
  /^[[:space:]]*\/\/[[:space:]]*BUDGETED-ISSUANCE-SEAM-END/   { in_seam = 0; next }
  # Now strip line comments so prose like "bare `Delegation::sign(`" in docs is
  # never treated as a code call site (the seam/anchor markers above already
  # matched against the raw line).
  { sub(/[ \t]*\/\/.*/, "", $0) }
  in_seam == 1 { next }                              # the seam may sign
  /Delegation::sign\(/ {
    print FILENAME ":" FNR ": bare Delegation::sign outside budgeted-issuance seam / test module"
    bad = 1
  }
  END { if (bad) exit 1; exit 0 }
' "$TARGET"; then
  echo "PASS: budgeted-issuance — all signing routes through sign_delegation_budgeted seam."
else
  echo "FAIL: budgeted-issuance seam gate red — non-seam Delegation::sign in production code."
  exit 1
fi
