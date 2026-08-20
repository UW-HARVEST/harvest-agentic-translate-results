#!/usr/bin/env bash
# Sanity-checks that the differential harness is not vacuous: inject a series of
# behaviour-changing mutations into src/lib.rs and require each one to be caught
# by at least one test binary. src/lib.rs is always restored at the end.
set -uo pipefail
cd "$(dirname "$0")/.."
TMP="${TMPDIR:-/tmp}"
GOOD="$TMP/lib.rs.good.$$"
cp src/lib.rs "$GOOD"
trap 'cp "$GOOD" src/lib.rs; rm -f "$GOOD"; cargo build >/dev/null 2>&1' EXIT

FAIL=0
run_mut() {
  local desc="$1" old="$2" new="$3"
  python3 -c "
import sys
s = open(sys.argv[1]).read()
old, new = sys.argv[2], sys.argv[3]
assert old in s, 'pattern not found: ' + old
open('src/lib.rs', 'w').write(s.replace(old, new, 1))
" "$GOOD" "$old" "$new" || { echo "  [SKIP]    $desc (pattern not found)"; return; }

  timeout 600 cargo build >/dev/null 2>&1
  local fails=""
  for t in phase_b_configs phase_c_errors phase_overunder; do
    timeout 600 cargo test --test "$t" >/dev/null 2>&1 || fails="$fails $t"
  done
  if [ -n "$fails" ]; then
    echo "  [CAUGHT]  $desc ->$fails"
  else
    echo "  [MISSED]  $desc  <-- harness blind spot"
    FAIL=1
  fi
}

echo "=== mutation testing src/lib.rs (every mutation must be CAUGHT) ==="
run_mut "process_with_fallthrough: case-5 delta 50 -> 51" \
        "result = result.wrapping_add(50);" "result = result.wrapping_add(51);"
run_mut "safe_double_to_int: high clamp threshold INT_MAX -> INT_MAX-1" \
        "if d > INT_MAX as c_double {" "if d > (INT_MAX - 1) as c_double {"
run_mut "safe_double_to_int: low clamp threshold INT_MIN -> INT_MIN+1" \
        "if d < INT_MIN as c_double {" "if d < (INT_MIN + 1) as c_double {"
run_mut "safe_double_to_int: NaN arm 0 -> -1" \
        "} else if d.is_nan() {
        0" "} else if d.is_nan() {
        -1"
run_mut "overunder: temp2 factor 2.7 -> 2.7000001" \
        "(b as c_double) * 2.7" "(b as c_double) * 2.7000001"
run_mut "overunder: a % 6 -> a.rem_euclid(6) (Euclidean instead of C modulo)" \
        "process_with_fallthrough(a % 6, b)" "process_with_fallthrough(a.rem_euclid(6), b)"
run_mut "overunder: d*d + a*a widened to i64 (loses the C's int overflow wrap)" \
        "((d.wrapping_mul(d)).wrapping_add(a.wrapping_mul(a)) as c_double).sqrt()" \
        "((d as i64 * d as i64 + a as i64 * a as i64) as c_double).sqrt()"
run_mut "overunder: label \"Source\" -> \"source\"" \
        'let src = b"Source";' 'let src = b"source";'
run_mut "handle_pointer_operations: +100 -> +101" \
        "(*ptr).wrapping_add(100)" "(*ptr).wrapping_add(101)"
run_mut "copy_data_block: 40 bytes -> 36 bytes" \
        "std::mem::size_of::<DataBlock>()," "36,"
run_mut "copy_data_block: libc memcpy -> ptr::copy_nonoverlapping (NULL fault differs)" \
        "        memcpy(
            dest as *mut core::ffi::c_void,
            src as *const core::ffi::c_void,
            std::mem::size_of::<DataBlock>(),
        );" \
        "        std::ptr::copy_nonoverlapping(
            src as *const u8,
            dest as *mut u8,
            std::mem::size_of::<DataBlock>(),
        );"

echo
if [ "$FAIL" -eq 0 ]; then
  echo "ALL MUTATIONS CAUGHT -- the differential harness has teeth."
else
  echo "SOME MUTATIONS WERE MISSED."
fi

# Note on one mutation deliberately NOT in the list:
#   `if d > INT_MAX as c_double`  ->  `if d >= INT_MAX as c_double`
# is a *semantically equivalent* mutant, not a coverage gap: the only input for
# which the predicates differ is d == 2147483647.0 exactly, and there both the
# clamp arm and the `(int)d` arm produce 2147483647. (double)INT_MAX is exactly
# representable, so no other input can distinguish them. The same argument
# applies to `d < INT_MIN` -> `d <= INT_MIN` at d == -2147483648.0. Both
# boundary inputs are nonetheless asserted explicitly by
# `err_e8_e9_inrange_boundaries`.
exit "$FAIL"
