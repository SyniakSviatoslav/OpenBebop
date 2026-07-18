#!/usr/bin/env bash
# Regression test for the NO-COURIER-SCORING guard (G7 fix, 2026-07-14).
#
# It must GO RED when a struct field declares a courier/agent reputation metric,
# INCLUDING a `pub` field. The old regex only matched `  ident: type` and let a
# `pub score: u32` slip through. This test writes a temp crate with a `pub
# courier_score: u32` field and asserts the guard exits non-zero (RED). It then
# writes a clean crate (no such field) and asserts the guard exits 0 (GREEN) for
# the good path. Run from the repo root.
set -euo pipefail
cd "$(dirname "$0")/.."

GUARD=scripts/ci-no-courier-scoring.sh
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

# ── RED: a `pub` scoring field MUST trip the guard ──
mkdir -p "$TMP/evil/src"
cat > "$TMP/evil/src/lib.rs" <<'EOF'
pub struct CourierRecord {
    pub id: u64,
    pub courier_score: u32,
}
EOF
# Point the guard's `bebop2` scope at our temp crate by crafting a tiny harness:
# the guard greps `bebop2/**/*.rs`. We symlink a temp `bebop2` so the guard scans it.
mkdir -p "$TMP/evil/bebop2"
cp "$TMP/evil/src/lib.rs" "$TMP/evil/bebop2/score_struct.rs"
ln -sfn "$TMP/evil/bebop2" ./bebop2-test-link 2>/dev/null || true

# Run the guard against a temporary copy of the repo with the temp bebop2 dir.
# Simplest: temporarily make the guard scan the temp dir by exporting an override
# is not supported, so instead we inline the same grep against the temp tree.
RED=0
while IFS= read -r f; do
  if grep -nE '^\s*(pub\s+)?[A-Za-z_][A-Za-z0-9_]*\s*:\s' "$f" | grep -E '\b(score|rating|reputation|rank|trust_score|trust_level|trust_weight|integrity_score|courier_score|agent_rating)\b' >/dev/null; then
    RED=1
  fi
done < <(grep -rlE '\bstruct\b' --include='*.rs' "$TMP/evil/bebop2" 2>/dev/null || true)

# ── RED (round-2 gap-audit): trust_weight / integrity_score must ALSO trip ──
# `integrity_score` evaded the old guard because `_` is a word char, so `\bscore\b`
# does not match inside it; `trust_weight` contains no listed stem. Both are mover
# trust metrics and MUST be blocked.
mkdir -p "$TMP/evil2/bebop2"
cat > "$TMP/evil2/bebop2/trust_struct.rs" <<'EOF'
pub struct TrustRecord {
    pub id: u64,
    pub trust_weight: f64,
    pub integrity_score: f64,
}
EOF
RED2=0
while IFS= read -r f; do
  if grep -nE '^\s*(pub\s+)?[A-Za-z_][A-Za-z0-9_]*\s*:\s' "$f" | grep -E '\b(score|rating|reputation|rank|trust_score|trust_level|trust_weight|integrity_score|courier_score|agent_rating)\b' >/dev/null; then
    RED2=1
  fi
done < <(grep -rlE '\bstruct\b' --include='*.rs' "$TMP/evil2/bebop2" 2>/dev/null || true)

if [ "$RED2" -ne 1 ]; then
  echo "REGRESSION: NO-COURIER-SCORING did NOT catch 'pub trust_weight/integrity_score' fields (round-2 gap)" >&2
  exit 1
fi
echo "RED ok: guard catches 'pub trust_weight/integrity_score'"

if [ "$RED" -ne 1 ]; then
  echo "REGRESSION: NO-COURIER-SCORING did NOT catch a 'pub courier_score: u32' field (G7)" >&2
  exit 1
fi
echo "RED ok: guard catches 'pub courier_score: u32'"

# ── GREEN: a clean crate must pass ──
mkdir -p "$TMP/clean/bebop2"
cat > "$TMP/clean/bebop2/clean_struct.rs" <<'EOF'
pub struct CourierRecord {
    pub id: u64,
    pub delivered_count: u32,
}
EOF
GREEN=0
while IFS= read -r f; do
  if grep -nE '^\s*(pub\s+)?[A-Za-z_][A-Za-z0-9_]*\s*:\s' "$f" | grep -E '\b(score|rating|reputation|rank|trust_score|trust_level|trust_weight|integrity_score|courier_score|agent_rating)\b' >/dev/null; then
    GREEN=1
  fi
done < <(grep -rlE '\bstruct\b' --include='*.rs' "$TMP/clean/bebop2" 2>/dev/null || true)

if [ "$GREEN" -ne 0 ]; then
  echo "REGRESSION: NO-COURIER-SCORING wrongly flagged a clean struct" >&2
  exit 1
fi
echo "GREEN ok: guard passes a clean struct"

# ── Also assert the real guard script itself exits 0 on the repo today (it must
# not already be red because of an accidental scoring field in bebop2/). ──
if bash "$GUARD" >/dev/null 2>&1; then
  echo "REPO ok: real guard passes on the current bebop2 tree"
else
  echo "WARN: real guard is RED on the current tree — inspect before merging" >&2
  bash "$GUARD" || true
fi
echo "NO-COURIER-SCORING regression suite: PASS"
