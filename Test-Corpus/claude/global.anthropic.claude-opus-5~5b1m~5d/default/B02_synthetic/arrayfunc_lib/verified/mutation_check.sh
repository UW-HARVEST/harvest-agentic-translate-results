#!/usr/bin/env bash
# Mutation test for the differential suite.
#
# "All tests pass" only means something if the suite would NOTICE a regression.
# This injects deliberate bugs into src/lib.rs one at a time, re-runs the suite,
# and requires it to FAIL for each one. A mutation that survives means the
# corresponding behaviour is not actually covered.
#
# Mutations come in two kinds:
#   run_mutation             - must be CAUGHT by the suite
#   run_equivalent_mutation  - provably cannot change observable behaviour, so it
#                              must SURVIVE. Recording these keeps the
#                              equivalence analysis honest and documents that the
#                              survivor is not a coverage gap.
#
# src/lib.rs is restored on every exit path.

set -u
cd "$(dirname "$0")"

SRC=src/lib.rs
BACKUP=$(mktemp)
cp "$SRC" "$BACKUP"
restore() { cp "$BACKUP" "$SRC"; rm -f "$BACKUP"; }
trap restore EXIT INT TERM

CAUGHT=0
EQUIV=0
FAILED=0

# run_mutation <name> <old-text> <new-text> <cargo-test-args>
run_mutation() {
  local name="$1" old="$2" new="$3" tests="$4"
  cp "$BACKUP" "$SRC"
  if ! python3 mutate.py "$SRC" "$old" "$new" >/dev/null; then
    echo "SKIP   $name  (mutation target not found)"
    FAILED=$((FAILED + 1))
    return
  fi

  local out
  out=$(timeout 600 cargo test --offline $tests 2>&1)
  if echo "$out" | grep -qE 'test result: FAILED|error: test failed|error: could not compile'; then
    local first
    first=$(echo "$out" | grep -oE 'test [a-z0-9_]+ \.\.\. FAILED' | head -3 | tr '\n' ' ')
    echo "CAUGHT $name  ->  ${first:-suite failed}"
    CAUGHT=$((CAUGHT + 1))
  else
    echo "SURVIVED (BAD) $name  --  the suite did not notice this bug!"
    FAILED=$((FAILED + 1))
  fi
}

# run_equivalent_mutation <name> <old-text> <new-text> <why-equivalent>
run_equivalent_mutation() {
  local name="$1" old="$2" new="$3" why="$4"
  cp "$BACKUP" "$SRC"
  if ! python3 mutate.py "$SRC" "$old" "$new" >/dev/null; then
    echo "SKIP   (equivalent) $name  (mutation target not found)"
    FAILED=$((FAILED + 1))
    return
  fi

  local out
  out=$(timeout 600 cargo test --offline 2>&1)
  if echo "$out" | grep -qE 'test result: FAILED|error: test failed|error: could not compile'; then
    echo "UNEXPECTED $name  --  claimed behaviour-preserving but the suite caught it!"
    FAILED=$((FAILED + 1))
  else
    echo "EQUIVALENT (as expected) $name"
    echo "         why: $why"
    EQUIV=$((EQUIV + 1))
  fi
}

echo "=== Mutation testing the differential suite ==="
echo

# ---------------------------------------------------------------------------
# safe_double_to_int
# ---------------------------------------------------------------------------
run_mutation "safe_double_to_int: upper clamp threshold shifted by 1" \
  'if d >= i32::MAX as c_double {' \
  'if d >= i32::MAX as c_double - 1.0 {' \
  "--test phase_b_valid --test phase_c_errors"

run_mutation "safe_double_to_int: lower clamp threshold shifted by 1" \
  'if d <= i32::MIN as c_double {' \
  'if d <= i32::MIN as c_double + 1.0 {' \
  "--test phase_b_valid --test phase_c_errors"

run_mutation "safe_double_to_int: upper clamp returns INT_MAX-1" \
  'if d >= i32::MAX as c_double {
        return i32::MAX;' \
  'if d >= i32::MAX as c_double {
        return i32::MAX - 1;' \
  "--test phase_b_valid --test phase_c_errors"

run_mutation "safe_double_to_int: lower clamp returns INT_MIN+1" \
  'if d <= i32::MIN as c_double {
        return i32::MIN;' \
  'if d <= i32::MIN as c_double {
        return i32::MIN + 1;' \
  "--test phase_b_valid --test phase_c_errors"

run_mutation "safe_double_to_int: NaN returns 1 instead of 0" \
  'if d != d {
        return 0;
    }' \
  'if d != d {
        return 1;
    }' \
  "--test phase_c_errors"

run_mutation "safe_double_to_int: truncation becomes rounding" \
  'd as c_int
}' \
  'd.round() as c_int
}' \
  "--test phase_b_valid --test phase_c_errors"

# ---------------------------------------------------------------------------
# The four operations
# ---------------------------------------------------------------------------
run_mutation "add_operation: + becomes -" \
  'pub extern "C" fn add_operation(a: c_int, b: c_int, _unused1: c_int, _unused2: c_int) -> c_int {
    a.wrapping_add(b)' \
  'pub extern "C" fn add_operation(a: c_int, b: c_int, _unused1: c_int, _unused2: c_int) -> c_int {
    a.wrapping_sub(b)' \
  "--test phase_b_valid"

run_mutation "multiply_operation: * becomes +" \
  'a.wrapping_mul(b)
}' \
  'a.wrapping_add(b)
}' \
  "--test phase_b_valid"

run_mutation "subtract_operation: a - b becomes b - a" \
  'a.wrapping_sub(b)
}' \
  'b.wrapping_sub(a)
}' \
  "--test phase_b_valid"

run_mutation "modulo_operation: b == 0 guard removed" \
  'if b == 0 {
        return 0;
    }
    c_int_rem(a, b)' \
  'c_int_rem(a, b)' \
  "--test phase_c_errors"

run_mutation "modulo_operation: idiv replaced by wrapping_rem (loses SIGFPE)" \
  'fn c_int_rem(a: c_int, b: c_int) -> c_int {
    let rem: c_int;' \
  'fn c_int_rem(a: c_int, b: c_int) -> c_int {
    if true { return a.wrapping_rem(b); }
    #[allow(unreachable_code)]
    let rem: c_int;' \
  "--test crash_probes"

# ---------------------------------------------------------------------------
# compute_scaled_value
# ---------------------------------------------------------------------------
run_mutation "compute_scaled_value: * becomes /" \
  'let scaled = base as c_double * scale_factor;' \
  'let scaled = base as c_double / scale_factor;' \
  "--test phase_b_valid --test phase_c_errors"

run_mutation "compute_scaled_value: ignores scale_factor" \
  'let scaled = base as c_double * scale_factor;' \
  'let scaled = base as c_double;' \
  "--test phase_b_valid --test phase_c_errors"

# ---------------------------------------------------------------------------
# init_result_array
# ---------------------------------------------------------------------------
run_mutation "init_result_array: scaled factor 1.5 becomes 1.4" \
  'scaled: v as c_double * 1.5,' \
  'scaled: v as c_double * 1.4,' \
  "--test phase_b_valid"

run_mutation "init_result_array: clamp 10 becomes 9" \
  '(*arr).count = if count < 10 { count } else { 10 };' \
  '(*arr).count = if count < 9 { count } else { 9 };' \
  "--test phase_b_valid --test phase_c_errors"

run_mutation "init_result_array: oversized clamp target 10 becomes 8" \
  '(*arr).count = if count < 10 { count } else { 10 };' \
  '(*arr).count = if count < 10 { count } else { 8 };' \
  "--test phase_b_valid --test phase_c_errors"

run_mutation "init_result_array: rank i becomes i+1" \
  'rank: i,' \
  'rank: i.wrapping_add(1),' \
  "--test phase_b_valid"

run_mutation "init_result_array: negative count clamped to 0" \
  '(*arr).count = if count < 10 { count } else { 10 };' \
  '(*arr).count = if count < 0 { 0 } else if count < 10 { count } else { 10 };' \
  "--test phase_c_errors"

# ---------------------------------------------------------------------------
# process_with_foreach
# ---------------------------------------------------------------------------
run_mutation "process_with_foreach: 0.75 becomes 0.7" \
  'let temp: c_double = result as c_double * 0.75;' \
  'let temp: c_double = result as c_double * 0.7;' \
  "--test phase_b_valid"

run_mutation "process_with_foreach: passes rank as unused1" \
  'let result: c_int = f((*item).value, (*item).rank, 0, 0);' \
  'let result: c_int = f((*item).value, (*item).rank, (*item).rank, 0);' \
  "--test phase_b_valid --test phase_c_errors"

run_mutation "process_with_foreach: swaps value and rank arguments" \
  'let result: c_int = f((*item).value, (*item).rank, 0, 0);' \
  'let result: c_int = f((*item).rank, (*item).value, 0, 0);' \
  "--test phase_b_valid"

run_mutation "process_with_foreach: FOREACH != becomes < (changes runaway case)" \
  'while count_iter != size {' \
  'while count_iter < size {' \
  "--test crash_probes"

run_mutation "process_with_foreach: unwraps op before the loop" \
  'let mut count_iter: c_int = 0;
    while count_iter != size {' \
  'let _precheck: unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int = op.unwrap();
    let mut count_iter: c_int = 0;
    while count_iter != size {' \
  "--test phase_c_errors --test crash_probes"

run_mutation "process_with_foreach: does not write back item->value" \
  '(*item).value = safe_double_to_int(temp);' \
  '(*item).value = (*item).value;' \
  "--test phase_b_valid"

# ---------------------------------------------------------------------------
# compute_weighted_sum
# ---------------------------------------------------------------------------
run_mutation "compute_weighted_sum: weight fallback 1 becomes 0" \
  'let weight: c_int = if i > 0 { i } else { 1 };' \
  'let weight: c_int = if i > 0 { i } else { 0 };' \
  "--test phase_b_valid --test phase_c_errors"

run_mutation "compute_weighted_sum: weight uses i+1 instead of i" \
  'let weight: c_int = if i > 0 { i } else { 1 };' \
  'let weight: c_int = if i > 0 { i.wrapping_add(1) } else { 1 };' \
  "--test phase_b_valid --test phase_c_errors"

run_mutation "compute_weighted_sum: 0.8 becomes 0.7" \
  'let weighted: c_double = (*current).value as c_double * weight as c_double * 0.8;' \
  'let weighted: c_double = (*current).value as c_double * weight as c_double * 0.7;' \
  "--test phase_b_valid"

# ---------------------------------------------------------------------------
# compare_results_in_array
# ---------------------------------------------------------------------------
run_mutation "compare_results_in_array: >= count becomes > count" \
  'if idx1 >= (*arr).count || idx2 >= (*arr).count {' \
  'if idx1 > (*arr).count || idx2 > (*arr).count {' \
  "--test phase_b_valid --test phase_c_errors"

run_mutation "compare_results_in_array: also rejects negative indices" \
  'if idx1 >= (*arr).count || idx2 >= (*arr).count {' \
  'if idx1 >= (*arr).count || idx2 >= (*arr).count || idx1 < 0 || idx2 < 0 {' \
  "--test phase_b_valid --test phase_c_errors"

run_mutation "compare_results_in_array: -1 and 1 swapped" \
  'if ptr1 < ptr2 {
        -1
    } else if ptr1 > ptr2 {
        1
    } else {
        0
    }' \
  'if ptr1 < ptr2 {
        1
    } else if ptr1 > ptr2 {
        -1
    } else {
        0
    }' \
  "--test phase_b_valid --test phase_c_errors"

run_mutation "compare_results_in_array: only checks idx1" \
  'if idx1 >= (*arr).count || idx2 >= (*arr).count {' \
  'if idx1 >= (*arr).count {' \
  "--test phase_c_errors"

# ---------------------------------------------------------------------------
# arrayfunc
# ---------------------------------------------------------------------------
run_mutation "arrayfunc: param4 / 2 becomes param4 >> 1" \
  '(param4 / 2).wrapping_add(1),' \
  '(param4 >> 1).wrapping_add(1),' \
  "--test phase_b_valid --test phase_c_errors"

run_mutation "arrayfunc: final scale 0.333 becomes 0.33" \
  'let final_scale: c_double = result as c_double * 0.333;' \
  'let final_scale: c_double = result as c_double * 0.33;' \
  "--test phase_b_valid"

run_mutation "arrayfunc: compare loop bound count-1 becomes count-2" \
  'while i < arr.count.wrapping_sub(1) {' \
  'while i < arr.count.wrapping_sub(2) {' \
  "--test phase_b_valid"

run_mutation "arrayfunc: operations order add/multiply swapped" \
  'Some(add_operation_shim),
        Some(multiply_operation_shim),' \
  'Some(multiply_operation_shim),
        Some(add_operation_shim),' \
  "--test phase_b_valid"

run_mutation "arrayfunc: runs only 3 of the 4 operations" \
  'while i < 4 {' \
  'while i < 3 {' \
  "--test phase_b_valid"

run_mutation "arrayfunc: init count 8 becomes 7" \
  'init_result_array(&mut arr, values.as_mut_ptr(), 8);' \
  'init_result_array(&mut arr, values.as_mut_ptr(), 7);' \
  "--test phase_b_valid"

run_mutation "arrayfunc: values[6] uses *3 instead of *2" \
  'param3.wrapping_mul(2),' \
  'param3.wrapping_mul(3),' \
  "--test phase_b_valid"

run_mutation "arrayfunc: values[5] uses + instead of -" \
  'param2.wrapping_sub(param3),' \
  'param2.wrapping_add(param3),' \
  "--test phase_b_valid"

run_mutation "arrayfunc: drops compute_weighted_sum" \
  'result = result.wrapping_add(compute_weighted_sum(&mut arr));' \
  'result = result.wrapping_add(0);' \
  "--test phase_b_valid"

# ---------------------------------------------------------------------------
# Provably behaviour-preserving changes: these MUST survive.
# ---------------------------------------------------------------------------
echo
echo "--- expected-equivalent mutants (must survive the full suite) ---"

run_equivalent_mutation "safe_double_to_int: >= INT_MAX becomes > INT_MAX" \
  'if d >= i32::MAX as c_double {' \
  'if d > i32::MAX as c_double {' \
  "only d == 2147483647.0 changes path, and (int)2147483647.0 == INT32_MAX, so both paths return the same value"

run_equivalent_mutation "safe_double_to_int: <= INT_MIN becomes < INT_MIN" \
  'if d <= i32::MIN as c_double {' \
  'if d < i32::MIN as c_double {' \
  "only d == -2147483648.0 changes path, and (int)(-2147483648.0) == INT32_MIN exactly, so both paths return the same value"

run_equivalent_mutation "arrayfunc: compare loop bound count-1 becomes count" \
  'while i < arr.count.wrapping_sub(1) {' \
  'while i < arr.count {' \
  "the extra iteration calls compare_results_in_array(count-1, count), whose 'idx2 >= count' guard returns 0 and contributes nothing to the sum"

run_equivalent_mutation "compute_scaled_value: safe_double_to_int replaced by a plain 'as' cast" \
  'let scaled = base as c_double * scale_factor;
    safe_double_to_int(scaled)' \
  'let scaled = base as c_double * scale_factor;
    scaled as c_int' \
  "Rust's float->int 'as' cast is defined to saturate and map NaN to 0, which coincides exactly with safe_double_to_int's three guards, so the two are indistinguishable for every double (this would NOT hold in C, where the cast is UB)"

run_equivalent_mutation "init_result_array: clamp threshold 10 becomes 11 in the true-branch test" \
  '(*arr).count = if count < 10 { count } else { 10 };' \
  '(*arr).count = if count < 11 { count } else { 10 };' \
  "the two branches only disagree for count == 10, and both yield 10"

restore
trap - EXIT INT TERM

echo
echo "=== Mutation summary: $CAUGHT caught, $EQUIV provably-equivalent, $FAILED unexpected ==="
if [ "$FAILED" -ne 0 ]; then
  echo "FAIL: some mutations behaved unexpectedly - the suite may have blind spots."
  exit 1
fi
echo "OK: every injected bug was detected, and every equivalent mutant survived."
