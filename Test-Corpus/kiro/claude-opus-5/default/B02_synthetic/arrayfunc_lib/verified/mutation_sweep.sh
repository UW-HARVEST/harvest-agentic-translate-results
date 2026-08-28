#!/usr/bin/env bash
# Mutation sweep: prove the differential suite detects a defect in EVERY
# translated function. Each mutant must be caught by at least one test.
# Restores src/lib.rs unconditionally on exit.
set -uo pipefail
cd "$(dirname "$0")"

BAK=$(mktemp)
cp src/lib.rs "$BAK"
trap 'cp "$BAK" src/lib.rs; rm -f "$BAK"' EXIT

# name|sed-expression
# NOTE: the following mutants were tried and are *semantically equivalent*, so
# no test can distinguish them (they are not coverage gaps):
#   `d >= i32::MAX` -> `d > i32::MAX`   : the fall-through cast yields i32::MAX anyway
#   `d <= i32::MIN` -> `d < i32::MIN`   : likewise yields i32::MIN
#   `if d != d`     -> `if false`       : Rust's saturating float cast maps NaN to 0
#   `count < 10`    -> `count <= 10`    : both branches give 10 when count == 10
#   `i < count - 1` -> `i < count`      : the extra compare hits the idx>=count guard and adds 0
# They are replaced below by mutants that do change observable behaviour.
MUTANTS=(
  'add_operation|s/a\.wrapping_add(b)$/a.wrapping_add(b).wrapping_add(1)/'
  'multiply_operation|s/a\.wrapping_mul(b)$/a.wrapping_mul(b).wrapping_add(1)/'
  'subtract_operation|s/a\.wrapping_sub(b)$/b.wrapping_sub(a)/'
  'modulo_operation|s/a\.wrapping_rem(b)$/a.wrapping_rem(b).wrapping_neg()/'
  'modulo_operation:guard|s/^    if b == 0 {$/    if b == 1 {/'
  'safe_double_to_int:maxclamp|s/^        return i32::MAX;$/        return i32::MAX - 1;/'
  'safe_double_to_int:minclamp|s/^        return i32::MIN;$/        return i32::MIN + 1;/'
  'safe_double_to_int:trunc|s/^    d as c_int$/    d.floor() as c_int/'
  'safe_double_to_int:round|s/^    d as c_int$/    d.round() as c_int/'
  'compute_scaled_value|s/let scaled = (base as f64) \* scale_factor;/let scaled = (base as f64) * scale_factor + 1.0;/'
  'init_result_array:scale|s/scaled: (v as f64) \* 1\.5,/scaled: (v as f64) * 1.5000001,/'
  'init_result_array:clamp|s/arr\.count = if count < MAX_RESULTS as c_int {/arr.count = if count < 9 as c_int {/'
  'init_result_array:rank|s/^            rank: i,$/            rank: i + 1,/'
  'compare_results:bound|s/if idx1 >= arr\.count || idx2 >= arr\.count/if idx1 > arr.count || idx2 > arr.count/'
  'compare_results:sign|s/^    if idx1 < idx2 {$/    if idx1 > idx2 {/'
  'process_foreach:scale|s/let temp = (result as f64) \* 0\.75;/let temp = (result as f64) * 0.7500001;/'
  'process_foreach:args|s/op(item\.value, item\.rank, 0, 0)/op(item.rank, item.value, 0, 0)/'
  'process_foreach:noscaled|s/^        item\.scaled = temp;$/        item.scaled = temp + 0.0000001;/'
  'process_foreach:order|s/^        item\.value = safe_double_to_int_impl(temp);$/        item.value = safe_double_to_int_impl(temp) + 0;\n        total = total.wrapping_add(1);/'
  'weighted_sum:factor|s/\* (weight as f64) \* 0\.8;/* (weight as f64) * 0.80000001;/'
  'weighted_sum:weight|s/let weight: c_int = if i > 0 { i } else { 1 };/let weight: c_int = if i > 0 { i } else { 0 };/'
  'arrayfunc:final|s/let final_scale = (result as f64) \* 0\.333;/let final_scale = (result as f64) * 0.3330001;/'
  'arrayfunc:values|s/param4\.wrapping_div(2)\.wrapping_add(1)/param4.wrapping_div(2)/'
  'arrayfunc:oplist|s/^        subtract_operation,$/        add_operation,/'
  'arrayfunc:cmploop|s/while i < arr\.count - 1 {/while i < arr.count - 2 {/'
  'arrayfunc:initcount|s/init_result_array_impl(&mut arr, &values, 8);/init_result_array_impl(\&mut arr, \&values, 7);/'
)

pass=0; fail=0
for entry in "${MUTANTS[@]}"; do
  name="${entry%%|*}"
  expr="${entry#*|}"

  cp "$BAK" src/lib.rs
  if ! sed -i "$expr" src/lib.rs 2>/dev/null; then
    echo "SKIP  $name (sed error)"; continue
  fi
  if cmp -s "$BAK" src/lib.rs; then
    echo "!! NO-OP $name — mutation pattern did not apply"; fail=$((fail+1)); continue
  fi

  out=$(timeout 600 cargo test --release --no-default-features 2>&1)
  rc=$?

  # A mutant that does not compile proves nothing — report and skip it.
  if echo "$out" | grep -qE '^error(\[|: could not compile|: expected)'; then
    echo "SKIP  $name (mutant does not compile)"; continue
  fi

  # "Caught" means the suite did not come back green. Note that the cdylib is
  # built with panic=abort, so a mismatch that trips a bounds check or a
  # division by zero kills the test process outright instead of printing
  # "test result: FAILED" — a non-zero exit status covers both shapes.
  if [ "$rc" -ne 0 ]; then
    caught=$(echo "$out" | grep '^test .* FAILED' | sed 's/^test //; s/ \.\.\. FAILED//' | paste -sd, -)
    [ -z "$caught" ] && caught="test process aborted"
    echo "CAUGHT $name  <- $caught"
    pass=$((pass+1))
  else
    echo "MISSED $name  ** no test detected this defect **"
    fail=$((fail+1))
  fi
done

echo "-----"
echo "caught=$pass missed=$fail"
[ "$fail" -eq 0 ]
