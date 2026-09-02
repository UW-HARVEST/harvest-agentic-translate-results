#!/usr/bin/env bash
# Mutation check: prove the differential suite is actually SENSITIVE.
#
# A green test suite is only evidence if it would go red on a real divergence.
# This script injects one deliberate behavioral bug into src/lib.rs at a time,
# rebuilds the Rust .so, runs the suite, and requires it to FAIL. Then it
# restores the original source. Any mutation the suite fails to catch is a
# coverage hole in the tests, not in the translation.
set -uo pipefail

cd "$(dirname "$0")/.." || exit 1

SRC=src/lib.rs
BACKUP=$(mktemp)
cp "$SRC" "$BACKUP"
restore() { cp "$BACKUP" "$SRC"; rm -f "$BACKUP"; }
trap restore EXIT

TIMEOUT=${TIMEOUT:-600}
undetected=0
total=0

# Each mutation is "description|python-replacement-expression".
# The replacement uses python so multi-line / exact-string edits are safe.
mutate_and_test() {
  local desc="$1" old="$2" new="$3"
  total=$((total + 1))
  cp "$BACKUP" "$SRC"
  if ! python3 - "$SRC" "$old" "$new" <<'PY'
import sys
path, old, new = sys.argv[1], sys.argv[2], sys.argv[3]
s = open(path).read()
# Skip occurrences that live inside doc comments: this file quotes the original C
# in `///` blocks, so a naive "first occurrence" replace would mutate a comment
# and produce a no-op mutant that looks like a test blind spot.
pos, chosen = 0, -1
while True:
    i = s.find(old, pos)
    if i < 0:
        break
    line_start = s.rfind("\n", 0, i) + 1
    if not s[line_start:i].lstrip().startswith("//"):
        chosen = i
        break
    pos = i + 1
if chosen < 0:
    sys.exit("MUTATION TARGET NOT FOUND IN CODE (only in comments?): " + old)
open(path, "w").write(s[:chosen] + new + s[chosen + len(old):])
PY
  then
    echo "!! could not apply mutation: $desc"
    undetected=$((undetected + 1))
    return
  fi
  if timeout "$TIMEOUT" cargo test --quiet >/dev/null 2>&1; then
    echo "UNDETECTED  $desc"
    undetected=$((undetected + 1))
  else
    echo "caught      $desc"
  fi
}

echo "== mutation sensitivity check =="

mutate_and_test "arity: drop the unsigned-char narrowing of len" \
  'let len = (len as u32 & 0xFF) as c_uchar;' \
  'let len = len;'

mutate_and_test "arity: reject len < 3 instead of len < 2" \
  'if len < 2 {
            -1' \
  'if len < 3 {
            -1'

mutate_and_test "arity: return 0 instead of the -1 sentinel" \
  'if len < 2 {
            -1' \
  'if len < 2 {
            0'

mutate_and_test "apply_bitmask: default returns 0 instead of value" \
  '_ => value,' \
  '_ => 0,'

mutate_and_test "apply_bitmask: swap mask1 and mask2" \
  '0 => value & mask1,' \
  '0 => value & mask2,'

mutate_and_test "apply_bitmask: use XOR where the C uses OR" \
  '2 => value | mask3,' \
  '2 => value ^ mask3,'

mutate_and_test "arity4: euclidean modulo instead of C truncating remainder" \
  'apply_bitmask(result, param1 % 4)' \
  'apply_bitmask(result, param1.rem_euclid(4))'

mutate_and_test "arity4: floor division instead of C truncation toward zero" \
  'result = result.wrapping_mul(param3).wrapping_div(100);' \
  'result = result.wrapping_mul(param3).div_euclid(100);'

mutate_and_test "arity4: scale by param3 even when param3 == 0" \
  'if param3 != 0 {' \
  'if true {'

mutate_and_test "arity4: skip the param4 addition" \
  'if param4 != 0 {
        result = result.wrapping_add(param4);' \
  'if false {
        result = result.wrapping_add(param4);'

mutate_and_test "arity4: matrix contribution uses [0][0]+[2][2]" \
  'matrix[2][3]' \
  'matrix[2][2]'

mutate_and_test "shift_array: allow positions == size" \
  'if positions > 0 && positions < size {' \
  'if positions > 0 && positions <= size {'

mutate_and_test "shift_array: allow positions == 0" \
  'if positions > 0 && positions < size {' \
  'if positions >= 0 && positions < size {'

mutate_and_test "shift_array: zero-fill one element too many" \
  'for i in 0..positions as usize {' \
  'for i in 0..(positions as usize + 1) {'

mutate_and_test "shift_array: non-overlapping copy semantics (copy_nonoverlapping)" \
  'std::ptr::copy(arr, arr.add(positions as usize), count);' \
  'std::ptr::copy_nonoverlapping(arr, arr.add(positions as usize), count);'

mutate_and_test "process_string: treat a negative char (byte >= 0x80) as end-of-string" \
  'if *str != 0 {' \
  'if *str > 0 {'

mutate_and_test "init_matrix: change one matrix element" \
  '[[1, 2, 3, 4], [5, 6, 7, 8], [9, 10, 11, 12]]' \
  '[[1, 2, 3, 4], [5, 6, 7, 8], [9, 10, 11, 13]]'

mutate_and_test "compare_allocations: hardcode the ordering result to 1" \
  'if ptr1 < ptr2 {
            result = 1;' \
  'if true {
            result = 1;'

mutate_and_test "compare_allocations: bonus on val1 >= 0 instead of > 0" \
  'if *uninit_ptr > 0 { 10 } else { 0 }' \
  'if *uninit_ptr >= 0 { 10 } else { 0 }'

mutate_and_test "compare_allocations: read val2 instead of aliasing ptr1" \
  'uninit_ptr = ptr1;' \
  'uninit_ptr = ptr2;'

mutate_and_test "compare_allocations: leak the two allocations (changes the malloc pattern)" \
  'free(ptr1 as *mut c_void);
        free(ptr2 as *mut c_void);

        result' \
  'result'

mutate_and_test "arity3: forward a non-zero fourth argument" \
  'arity4(p1, p2, p3, 0)' \
  'arity4(p1, p2, p3, 1)'

mutate_and_test "arity2: forward p2 twice" \
  'arity4(p1, p2, 0, 0)' \
  'arity4(p2, p2, 0, 0)'

restore
trap - EXIT
cp "$SRC" /dev/null 2>/dev/null

echo
echo "mutations: $total, undetected: $undetected"
if ((undetected)); then
  echo "RESULT: the suite has $undetected blind spot(s)"
  exit 1
fi
echo "RESULT: every mutation was caught"
