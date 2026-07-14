#!/usr/bin/env bash
# RED+GREEN regression for the G2 guard.
set -euo pipefail
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
GUARD="$REPO_ROOT/scripts/ci-no-duplicate-eventlog.sh"

echo "== GREEN: clean tree has exactly one event-log primitive (MerkleLog) =="
bash "$GUARD" >/dev/null 2>&1 || { echo "GREEN FAILED"; exit 1; }
echo "GREEN OK"

echo "== RED: a second parallel event-log type must trip the guard =="
TMP="$(mktemp -d)"
mkdir -p "$TMP/bebop2/proto-wire/src"
# First sanctioned log.
printf 'pub struct MerkleLog {}\n' > "$TMP/bebop2/proto-wire/src/a.rs"
# A second parallel event-log type => duplication.
printf 'pub struct EventLog {}\n' > "$TMP/bebop2/proto-wire/src/b.rs"
if GUARD_ROOT="$TMP" bash "$GUARD" >/dev/null 2>&1; then
  echo "RED FAILED: guard did not catch a second event-log primitive"
  rm -rf "$TMP"
  exit 1
fi
echo "RED OK (guard tripped on duplicate log)"
rm -rf "$TMP"
echo "G2 regression: PASS (RED+GREEN)"
