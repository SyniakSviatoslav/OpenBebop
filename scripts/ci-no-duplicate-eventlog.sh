#!/usr/bin/env bash
# G2 guard (2026-07-14): bebop2 must have EXACTLY ONE append-only event-log
# primitive. The duplication gap is `MerkleLog` (bebop2) vs `MeshEvent`
# (dowiz-kernel) — but `ci-kernel-fence` forbids coupling proto-cap/proto-wire to
# dowiz-kernel, so the cross-repo mirror is intentionally out-of-repo. The
# in-repo discipline this guard enforces: do NOT introduce a SECOND parallel
# event-log type inside bebop2 (no re-implementation of the log). The single
# sanctioned primitive is `MerkleLog` in proto-wire/src/sync_pull.rs.
set -euo pipefail

REPO_ROOT="${GUARD_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
SCAN="$REPO_ROOT/bebop2/proto-wire/src $REPO_ROOT/bebop2/proto-cap/src"

# Count public `*Log` struct definitions across the two bebop2 crates.
LOG_TYPES="$(grep -rEno 'pub struct ([A-Za-z0-9_]*Log)\b' $SCAN 2>/dev/null | sed -E 's/.*pub struct ([A-Za-z0-9_]*Log).*/\1/' | sort -u || true)"

count="$(printf '%s\n' "$LOG_TYPES" | grep -c . || true)"

if [ "$count" -eq 0 ]; then
  echo "G2 VIOLATION: no event-log primitive found in bebop2 (expected MerkleLog)."
  exit 1
fi
if [ "$count" -gt 1 ]; then
  echo "G2 VIOLATION: more than one event-log primitive in bebop2 (duplication):"
  printf '%s\n' "$LOG_TYPES"
  echo "Only MerkleLog may exist; the cross-repo MeshEvent mirror is out-of-scope (ci-kernel-fence)."
  exit 1
fi
if [ "$LOG_TYPES" != "MerkleLog" ]; then
  echo "G2 VIOLATION: the sole event-log primitive is '$LOG_TYPES', expected 'MerkleLog'."
  exit 1
fi
echo "G2 guard: OK (single canonical event-log primitive: MerkleLog in bebop2)."
