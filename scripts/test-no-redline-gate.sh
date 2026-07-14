#!/usr/bin/env bash
# RED+GREEN regression for the G5 guard.
set -euo pipefail
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
GUARD="$REPO_ROOT/scripts/ci-no-redline-gate.sh"

echo "== GREEN: clean tree must pass the G5 guard =="
bash "$GUARD" >/dev/null 2>&1 || { echo "GREEN FAILED"; exit 1; }
echo "GREEN OK"

echo "== RED: removing the red-line gate must trip the guard =="
TMP="$(mktemp -d)"
mkdir -p "$TMP/bebop2/proto-cap/src"
# A proto-cap src tree with NO redline.rs => guard fails (symbol missing).
printf 'pub mod scope;\n' > "$TMP/bebop2/proto-cap/src/lib.rs"
printf '// deliberately absent red-line gate\n' > "$TMP/bebop2/proto-cap/src/error.rs"
if GUARD_ROOT="$TMP" bash "$GUARD" >/dev/null 2>&1; then
  echo "RED FAILED: guard did not catch missing red-line gate"
  rm -rf "$TMP"
  exit 1
fi
echo "RED OK (guard tripped)"
rm -rf "$TMP"
echo "G5 regression: PASS (RED+GREEN)"
