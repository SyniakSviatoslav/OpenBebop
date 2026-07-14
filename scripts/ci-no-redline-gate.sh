#!/usr/bin/env bash
# G5 guard (2026-07-14): bebop2 MUST have a real capability-scoped red-line gate.
# The deny-list guard kernel was archived TS / an unrelated physics veto that
# `bebop boot` no longer calls (blueprint gap G5). This guard fails the build if
# the red-line gate symbols are absent from proto-cap — proving the brake exists
# and is not silently removed.
set -euo pipefail

REPO_ROOT="${GUARD_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
SRC="$REPO_ROOT/bebop2/proto-cap/src"

# The red-line gate is required to live in redline.rs and expose these symbols.
REQUIRED=(
  "redline.rs:pub enum RedLinePolicy"
  "redline.rs:pub enum RedLineCategory"
  "redline.rs:pub fn is_red_line"
  "redline.rs:pub struct RedLineGate"
  "redline.rs:pub fn check"
  "error.rs:RedLineViolation"
  "hybrid_gate.rs:pub fn new_redlined"
)

found=0
for req in "${REQUIRED[@]}"; do
  f="${req%%:*}"; sym="${req#*:}"
  if ! grep -qF "$sym" "$SRC/$f" 2>/dev/null; then
    echo "G5 VIOLATION: required red-line symbol missing: $sym (expected in $f)"
    found=1
  fi
done

if [ "$found" -ne 0 ]; then
  echo "FAIL: G5 guard — red-line gate incomplete or removed (money/auth/secrets/migrations unprotected)."
  exit 1
fi
echo "G5 guard: OK (capability-scoped red-line deny gate present in proto-cap)."
