#!/usr/bin/env bash
# Meta-test: verify the differential suite can actually DETECT divergence.
#
# Injects a deliberate bug into src/lib.rs, runs the suite, and requires a
# NON-ZERO exit (a test failure OR a debug overflow-check abort). A mutation
# that survives means the suite has a blind spot. src/lib.rs is always restored.
#
# Replacements are applied only to CODE, never to comments (an earlier version
# of this script silently mutated a comment and reported false "blind spots").
set -uo pipefail
cd "$(dirname "$0")"

ORIG=$(mktemp); cp src/lib.rs "$ORIG"
restore() { cp "$ORIG" src/lib.rs; }
trap 'restore; rm -f "$ORIG"' EXIT

mutate() { python3 -c '
import sys
p="src/lib.rs"; old,new = sys.argv[1], sys.argv[2]
lines = open(p).read().split("\n")
n = 0
for i,l in enumerate(lines):
    # split off a trailing line comment; only mutate the code part
    code, sep, comment = l.partition("//")
    if old in code:
        code = code.replace(old, new)
        lines[i] = code + sep + comment
        n += 1
if n == 0:
    sys.exit("MUTATION PATTERN NOT FOUND IN CODE: " + old)
open(p,"w").write("\n".join(lines))
' "$1" "$2"; }

FAILURES=0
SURVIVORS=()
check() {
  local desc="$1" old="$2" new="$3"
  restore
  if ! mutate "$old" "$new"; then
    echo "  SKIP (pattern missing)                     | $desc"; return
  fi
  # Sanity: the code really changed.
  if diff -q src/lib.rs "$ORIG" >/dev/null; then
    echo "  SKIP (no-op mutation)                      | $desc"; restore; return
  fi
  local det_dbg=no det_rel=no
  timeout 600 cargo test          >/dev/null 2>&1 || det_dbg=yes
  timeout 600 cargo test --release >/dev/null 2>&1 || det_rel=yes
  if [ "$det_dbg" = yes ] || [ "$det_rel" = yes ]; then
    printf '  DETECTED (debug=%-3s release=%-3s)          | %s\n' "$det_dbg" "$det_rel" "$desc"
  else
    echo "  *** SURVIVED *** (blind spot!)             | $desc"
    SURVIVORS+=("$desc"); FAILURES=$((FAILURES+1))
  fi
  restore
}

# A mutation that is provably semantically identical MUST survive; if the suite
# "detects" it, the suite is over-constrained (asserting non-semantics).
check_equiv() {
  local desc="$1" old="$2" new="$3"
  restore
  if ! mutate "$old" "$new"; then
    echo "  SKIP (pattern missing)                     | $desc"; return
  fi
  local det=no
  timeout 600 cargo test          >/dev/null 2>&1 || det=yes
  timeout 600 cargo test --release >/dev/null 2>&1 || det=yes
  if [ "$det" = no ]; then
    echo "  SURVIVED as expected (equivalent)          | $desc"
  else
    echo "  *** FALSE POSITIVE *** (suite too strict)  | $desc"
    SURVIVORS+=("FALSE POSITIVE: $desc"); FAILURES=$((FAILURES+1))
  fi
  restore
}

echo "=== baseline must PASS in both profiles ==="
restore
b_dbg=ok; b_rel=ok
timeout 600 cargo test          >/dev/null 2>&1 || b_dbg=FAIL
timeout 600 cargo test --release >/dev/null 2>&1 || b_rel=FAIL
echo "  baseline debug=$b_dbg release=$b_rel"
[ "$b_dbg" = ok ] && [ "$b_rel" = ok ] || { echo "BASELINE BROKEN"; exit 1; }

echo "=== value / constant mutations ==="
check "M1  ceiling constant +7 -> +6"       "wrapping_add(7)"   "wrapping_add(6)"
check "M2  ceiling constant +7 -> +8"       "wrapping_add(7)"   "wrapping_add(8)"
check "M3  bitdepth boundary 32 -> 31"      "b(bitdepth != 32)" "b(bitdepth != 31)"
check "M4  bitdepth boundary 32 -> 33"      "b(bitdepth != 32)" "b(bitdepth != 33)"
check "M5  channels boundary 2 -> 3 (t1)"   "b(channels != 2)"  "b(channels != 3)"
check "M6  base constant 18 -> 17"          "18u32"             "17u32"
check "M7  base constant 18 -> 19"          "18u32"             "19u32"
check "M8  divisor 8 -> 4"                  "sum / 8"           "sum / 4"
check "M9  divisor 8 -> 16"                 "sum / 8"           "sum / 16"
check "M10 quotient off-by-one"             "sum / 8"           "(sum.wrapping_add(1)) / 8"
check "M11 rounding instead of truncation"  "sum / 8"           "(((sum as u64) + 4) / 8) as u32"
check "M12 shift instead of divide (wrong)" "sum / 8"           "sum >> 2"

echo "=== structural mutations ==="
check "M13 drop the (bitdepth!=32) +1"      "bitdepth.wrapping_add(b(bitdepth != 32))" "bitdepth"
check "M14 flip stereo predicate (t2)"      "b(channels == 2)"  "b(channels != 2)"
check "M15 off-by-one bitdepth in t1"       "wrapping_mul(bitdepth)" "wrapping_mul(bitdepth.wrapping_add(1))"
check "M16 drop channels!=2 factor in t1"   "channels.wrapping_mul(b(channels != 2))" "channels"
check "M17 drop term3 from the sum"         ".wrapping_add(term3)" ""
check "M18 drop term2 from the sum"         ".wrapping_add(term2)" ""
check "M19 drop term1 from the sum"         "let sum = term1"   "let sum = 0u32"
check "M20 drop 'channels' from the result" "wrapping_add(channels).wrapping_add(sum / 8)" "wrapping_add(sum / 8)"
check "M21 arg mix-up: blocksize for channels" "wrapping_add(channels).wrapping_add(sum / 8)" "wrapping_add(blocksize).wrapping_add(sum / 8)"
check "M22 arg mix-up: channels for bitdepth in t1" "        .wrapping_mul(bitdepth)" "        .wrapping_mul(channels)"

echo "=== overflow-semantics mutations (caught by debug overflow-checks) ==="
check "M23 18+channels non-wrapping"        "18u32.wrapping_add(channels)" "(18u32 + channels)"
check "M24 term2 product non-wrapping"      "blocksize.wrapping_mul(bitdepth).wrapping_mul(b(channels == 2))" "(blocksize * bitdepth) * b(channels == 2)"
check "M25 sum + 7 non-wrapping"            ".wrapping_add(7)"  " + 7"
check "M26 SIGNED division of the sum"      "sum / 8"           "((sum as i32) / 8) as u32"

echo "=== equivalent mutants (MUST survive: they are provably identical) ==="
check_equiv "E1  64-bit math then truncate to 32 bits" "18u32.wrapping_add(channels).wrapping_add(sum / 8)" "((18u64 + channels as u64 + (sum / 8) as u64) & 0xFFFF_FFFF) as u32"
check_equiv "E2  explicit % 2^32 instead of wrapping"  "18u32.wrapping_add(channels).wrapping_add(sum / 8)" "(((18u64 + channels as u64 + (sum / 8) as u64) % (1u64 << 32)) as u32)"
check_equiv "E3  shift-by-3 instead of divide-by-8"    "sum / 8"  "sum >> 3"

echo "=== ABI / export mutations ==="
check "M28 remove #[no_mangle]"             "#[unsafe(no_mangle)]" ""
check "M29 wrong exported symbol name"      "fn max_size_frame(" "fn max_size_frame_v2("
check "M30 truncate return to 16 bits"      "wrapping_add(sum / 8)" "wrapping_add(sum / 8) & 0xFFFF"
check "M31 return only the low byte"        "wrapping_add(sum / 8)" "wrapping_add(sum / 8) & 0xFF"

echo
if [ "$FAILURES" -eq 0 ]; then
  echo "ALL MUTATIONS DETECTED -- the differential suite is sensitive to divergence."
else
  echo "$FAILURES MUTATION(S) SURVIVED -- blind spots:"
  for s in "${SURVIVORS[@]}"; do echo "   - $s"; done
fi
exit "$FAILURES"
