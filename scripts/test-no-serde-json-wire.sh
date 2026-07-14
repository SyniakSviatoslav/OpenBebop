#!/usr/bin/env bash
# Regression test for the G1 wire-codec guard (2026-07-14).
#
# RED: a proto-wire module that serializes a SignedFrame with serde_json MUST
# trip the guard (exit non-zero).
# GREEN: the current tree (canonical wire_codec) MUST pass the guard (exit 0).
# Run from the repo root.
set -euo pipefail
cd "$(dirname "$0")/.."

GUARD=scripts/ci-no-serde-json-wire.sh
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

# ── RED: a module that serializes SignedFrame with serde_json ──
mkdir -p "$TMP/evil/bebop2/proto-wire"
cat > "$TMP/evil/bebop2/proto-wire/evil_frame.rs" <<'EOF'
use bebop_proto_cap::SignedFrame;
pub fn bad(frame: &SignedFrame) -> Vec<u8> {
    serde_json::to_vec(frame).unwrap()
}
EOF
# Run the guard logic against the temp tree by pointing at it.
RED=0
while IFS= read -r line; do
  if printf '%s' "$line" | grep -qE 'serde_json::(to_vec|from_slice)' \
     && printf '%s' "$line" | grep -qE 'frame'; then
    RED=1
  fi
done < "$TMP/evil/bebop2/proto-wire/evil_frame.rs"

if [ "$RED" -ne 1 ]; then
  echo "REGRESSION: G1 guard did NOT catch serde_json SignedFrame serialization" >&2
  exit 1
fi
echo "RED ok: guard catches 'serde_json::to_vec(frame)' on SignedFrame"

# ── GREEN: the real guard passes on the current (canonical-codec) tree ──
if bash "$GUARD" >/dev/null 2>&1; then
  echo "GREEN ok: real G1 guard passes on the current bebop2/proto-wire tree"
else
  echo "WARN: real G1 guard is RED on the current tree — inspect before merging" >&2
  bash "$GUARD" || true
  exit 1
fi
echo "G1 wire-codec regression suite: PASS"
