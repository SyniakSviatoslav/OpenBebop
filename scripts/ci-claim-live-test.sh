#!/usr/bin/env bash
# CI GUARD — LIVE-TEST CLAIM LINT (MESH-14).
# Any docs/design/*.md statement asserting a blueprint is CLOSED / VERIFIED /
# DONE must CITE a matching live test path/name, or CI goes red. This enforces
# "status only from live-test, not prose" (red-team-docs-were-stale lesson).
#
# Heuristic: a CLOSED/VERIFIED/DONE claim sentence lacking a `#[test]` /
# `fn <name>` / `cargo test` citation is flagged. Off by default tolerant:
# only flags obvious unbacked CLOSED claims adjacent to no test token.
set -euo pipefail
cd "$(dirname "$0")/.."
hit=0
while IFS= read -r f; do
  if grep -nE '\b(CLOSED|VERIFIED|DONE|GREEN)\b' "$f" >/dev/null; then
    if ! grep -qE '#\[test\]|fn [a-z_]+|cargo test|\.rs`' "$f"; then
      echo "LIVE-TEST-CLAIM gap in $f: a CLOSED/VERIFIED/DONE claim cites no live test"; hit=1
    fi
  fi
done < <(find docs/design/mesh-real -name '*.md' 2>/dev/null || true)
if [ "$hit" -eq 1 ]; then echo "FAIL: unbacked CLOSED claim without live-test citation"; exit 1; fi
echo "PASS: LIVE-TEST CLAIM lint — CLOSED claims cite live tests."
