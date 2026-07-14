#!/usr/bin/env bash
# CI GUARD — G1 (2026-07-14): NO serde_json wire (de)serialization of a SignedFrame.
#
# The external wire MUST carry SignedFrame in the canonical, fixed-layout binary
# codec (proto-wire/src/wire_codec.rs), NOT serde_json (non-canonical:
# implementation-defined field ordering, so a non-Rust node cannot reproduce the
# bytes, and there is a malleability surface). This guard fails CLOSED if any
# `serde_json::(to_vec|from_slice)` call in proto-wire actually operates on a
# SignedFrame value.
#
# Deliberately OUT OF SCOPE (and NOT flagged):
#   - doc-comment mentions of serde_json (informational only),
#   - the OUTER `Envelope` shell in envelope.rs, which serializes the envelope
#     metadata (not a SignedFrame) and is a separate, non-signed framing layer.
#
# A regression test (scripts/test-no-serde-json-wire.sh) pins this by dropping a
# temp .rs file that does `serde_json::to_vec(&frame)` on a SignedFrame and
# asserting the guard goes RED.
set -euo pipefail
cd "$(dirname "$0")/.."

hit=0
while IFS= read -r f; do
  # Only inspect proto-wire (the wire path). proto-cap's serde_json uses are
  # dev-only regression fixtures (Cargo.toml dev-dependency) and the signing path
  # there already uses TLV (ARCHITECTURE.md:75) — not flagged.
  while IFS= read -r line; do
    # Drop doc-comment lines (//! and ///) — they only MENTION serde_json.
    if printf '%s' "$line" | grep -qE '^\s*//[!/]'; then
      continue
    fi
    # A real violation: serde_json serialize/deserialize of a frame VALUE.
    # Pattern: `serde_json::to_vec(&frame)` / `serde_json::from_slice(...)` where
    # a `frame` (SignedFrame) variable is in play on the same line.
    if printf '%s' "$line" | grep -qE 'serde_json::(to_vec|from_slice)' \
       && printf '%s' "$line" | grep -qE 'frame'; then
      echo "G1 violation in $f: $line"
      hit=1
    fi
  done < "$f"
done < <(grep -rlE 'serde_json::(to_vec|from_slice)' --include='*.rs' bebop2/proto-wire 2>/dev/null || true)

if [ "$hit" -eq 1 ]; then
  echo "FAIL: G1 guard red — SignedFrame is still serialized with serde_json on the wire (must use wire_codec::encode_frame/decode_frame)"
  exit 1
fi
echo "PASS: G1 — no serde_json SignedFrame wire (de)serialization (canonical binary codec in force)."
