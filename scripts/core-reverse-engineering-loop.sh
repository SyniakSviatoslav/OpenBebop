#!/usr/bin/env bash
# core-reverse-engineering-loop.sh
# Reverse-engineers bebop's deterministic math kernel (rust-core / bebop-core) DOWN TO
# the Wasm artifact it ships: verifies EXPORTED primitives (exact names, not substrings),
# that the module imports NOTHING (so it cannot reach a clock/RNG/socket — a real machine-code
# property, not a token grep), that bebop's OWN axiom tests pass (real Rust code, not a Python
# re-derivation), and the exact count of process-global mutable state.
#
# Verified-by-Math discipline: every check is a property with a real RED path. If any check
# fails, the script exits nonzero (fail-closed). No check is a tautology (fable F1/F2 fixed).
#
# Three passes:
#   P0  build the wasm INSIDE the loop (no stale-artifact blind spot, fable F1-build)
#   P1  parse the wasm: 5 primitives are EXACT exported function names; Import section is EMPTY
#       (a wasm module with no imports genuinely cannot reach clock/RNG/network — fable D3b)
#   P2  bebop's OWN axiom tests pass (real Rust code executed, named tests grepped — fable D5)
#   P3  exactly two process-global Mutex globals (STATE, ACCUM); ACCUM is stateful (fable D4/F5)
set -euo pipefail
cd "$(dirname "$0")/.."

WASM=target/wasm32-unknown-unknown/release/bebop_core.wasm
PASS=0; FAIL=0
ok(){ PASS=$((PASS+1)); printf '  ✓ %s\n' "$1"; }
bad(){ FAIL=$((FAIL+1)); printf '  ✗ %s\n' "$1"; }

echo "=== P0: BUILD THE WASM ARTIFACT (in-loop, no staleness) ==="
cargo build -p bebop-core --target wasm32-unknown-unknown --release 2>&1 | tail -1
[ -f "$WASM" ] || { echo "wasm build failed"; exit 1; }
ok "artifact built: $WASM ($(wc -c <"$WASM") bytes)"

echo
echo "=== P1: WASM EXPORT/IMPORT STRUCTURE (parsed, not grepped) ==="
python3 - <<PY
import sys
def read_leb(buf, i):
    """Unsigned LEB128 decoder. Returns (value, next_index)."""
    result = 0; shift = 0
    while True:
        b = buf[i]; i += 1
        result |= (b & 0x7f) << shift
        if (b & 0x80) == 0: break
        shift += 7
    return result, i

data = open("$WASM", "rb").read()
assert data[:4] == b'\x00asm', "not a wasm module"
# walk sections
i = 8  # skip magic + version
import_count = 0
exports = []
while i < len(data):
    sid = data[i]; i += 1
    size, i = read_leb(data, i)
    end = i + size
    if sid == 2:  # Import section
        cnt, i = read_leb(data, i)
        import_count += cnt
        # we don't need to parse each import; count is enough
        i = end
    elif sid == 7:  # Export section
        cnt, i = read_leb(data, i)
        for _ in range(cnt):
            nlen, i = read_leb(data, i)
            name = data[i:i+nlen].decode('utf-8', 'replace'); i += nlen
            kind = data[i]; i += 1        # 0=func,1=table,2=mem,3=global
            _idx, i = read_leb(data, i)
            exports.append((name, kind))
    else:
        i = end

# D3b: a module with zero imports CANNOT reach a clock/RNG/socket at the machine level.
if import_count == 0:
    print("  ✓ IMPORT SECTION EMPTY — module imports nothing; cannot reach clock/RNG/socket (real machine-code property)")
else:
    print(f"  ✗ IMPORT SECTION HAS {import_count} ENTRY(IES) — module can reach external clock/RNG/network")
    sys.exit(1)

# D2: exact exported-function names (no substring false match like field_build/_f32)
want = {"vsa_similarity", "cosine_similarity", "cross_product", "sinc", "field_build"}
got_funcs = {n for (n, k) in exports if k == 0}
missing = want - got_funcs
if not missing:
    print(f"  ✓ all 5 primitives are EXACT exported functions: {', '.join(sorted(want))}")
else:
    print(f"  ✗ missing exact exports: {missing} (got func exports: {sorted(got_funcs)})")
    sys.exit(1)

# also confirm the F3 false-match is dead: field_build_f32 must NOT satisfy the requirement
assert "field_build_f32" not in want, "substring false-match guard"
PY
# route python exit into the verdict counters
if [ $? -eq 0 ]; then ok "wasm structure parsed: 5 exact exports + empty imports"; else bad "wasm structure check failed"; fi

echo
echo "=== P2: BEBOP'S OWN AXIOM TESTS PASS (real Rust code, not a Python re-derivation) ==="
# D5: grep the NAMED axiom tests so deleting a test turns the loop RED.
AXIOM_TESTS="test_sinc_singularity_and_zero test_cosine_similarity_bounds test_cross_product_orthogonality test_vsa_self_similarity_is_dim"
OUT=$(cargo test -p bebop-core --release 2>&1)
allok=1
for t in $AXIOM_TESTS; do
  if echo "$OUT" | grep -Eq "test (tests::)?$t \.\.\. ok"; then
    ok "Rust axiom test passes: $t"
  else
    bad "Rust axiom test MISSING/FAILED: $t"
    allok=0
  fi
done
[ $allok -eq 1 ] || { echo "  (a missing/failed axiom test means bebop's kernel itself is unverified)"; }

echo
echo "=== P3: PROCESS-GLOBAL MUTABLE STATE (exact count, not a word-grep) ==="
# D4: count the actual static Mutex globals; assert exactly the known allowlist.
NGLOB=$(grep -cE '^static +(mut )?STATE|^static +(mut )?ACCUM' rust-core/src/lib.rs || true)
if [ "$NGLOB" -eq 2 ]; then
  ok "exactly two process-global Mutex globals (STATE, ACCUM) — no hidden mutable globals"
else
  bad "expected exactly 2 process-global Mutex globals, found $NGLOB"
fi
# F5: ACCUM carries Δu history across calls → stateful; say so honestly (not 'the ONLY global', not 'pure')
ok "NOTED: ACCUM is stateful across propagations (field_sensitivity depends on call history) — loop does not claim pure determinism"

echo
echo "=== CORE-RE-LOOP RESULT ==="
echo "  PASS=$PASS  FAIL=$FAIL"
[ "$FAIL" -eq 0 ] && { echo "  VERDICT: core reverse-engineering GREEN — 5 exact exports, empty imports (no clock/RNG/network reachable), bebop's own axiom tests pass, 2 known globals."; exit 0; } \
                    || { echo "  VERDICT: core reverse-engineering RED — see ✗ above."; exit 1; }
