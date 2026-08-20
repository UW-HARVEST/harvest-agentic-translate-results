#!/usr/bin/env bash
# Sanity-check the differential test suite itself: inject a deliberate bug into
# src/lib.rs, confirm the suite CATCHES it, then restore the file.
#
# Each entry is  <sed expression>|<description>|<catch|equivalent>.
#   catch      - the suite MUST fail on this mutant.
#   equivalent - the mutant is provably indistinguishable from the original for
#                every input, so surviving is the correct outcome (a control that
#                shows the script is not just failing on everything).
set -uo pipefail
cd "$(dirname "$0")" || exit 1

SRC=src/lib.rs
BAK="$(mktemp)"
cp "$SRC" "$BAK"
restore() { cp "$BAK" "$SRC"; }
# The restore touches src/lib.rs, so the cdylib must be rebuilt afterwards or the
# next test run would (correctly) refuse to use a stale .so.
trap 'restore; rm -f "$BAK"; cargo build --offline >/dev/null 2>&1' EXIT

declare -a MUTATIONS=(
  # --- leaf predicates and arithmetic ---
  "s/const ONE: c_char = b'1' as c_char;/const ONE: c_char = b'2' as c_char;/|is_valid_operation lower bound|catch"
  "s/const FIVE: c_char = b'5' as c_char;/const FIVE: c_char = b'6' as c_char;/|is_valid_operation upper bound|catch"
  "s/let priority = op.wrapping_mul(10);/let priority = op.wrapping_mul(11);/|get_operation_priority factor|catch"
  "s/^    a.wrapping_add(b)\$/    a.wrapping_add(b).wrapping_add(1)/|add_operation off-by-one|catch"
  "s/    a.wrapping_mul(b)/    a.wrapping_mul(b).wrapping_neg()/|multiply_operation sign|catch"
  "s/    a.wrapping_sub(b)/    b.wrapping_sub(a)/|subtract_operation operand order|catch"
  "s/    a.wrapping_div(b)/    a.wrapping_div(b).wrapping_add(1)/|divide_operation off-by-one|catch"
  "s/    a.wrapping_rem(b)/    a.wrapping_rem(b.wrapping_abs().max(1))/|modulo_operation sign handling|catch"
  "s/    if b == 0 {\$/    if b == 1 {/|div\/mod zero guard|catch"

  # --- dispatch ---
  "s/        OP_DIVIDE => divide_operation,/        OP_DIVIDE => subtract_operation,/|select_operation wrong arm|catch"
  "s/        _ => add_operation,/        _ => multiply_operation,/|select_operation default arm|catch"

  # --- timestamp (only reachable with the LD_PRELOAD fixture) ---
  "s/    current_time >>= 29;/    current_time >>= 28;/|timestamp shift amount|catch"
  "s/    current_time >>= 29;/    current_time = ((current_time as u64) >> 29) as i64;/|timestamp logical instead of arithmetic shift|catch"

  # --- history recording ---
  "s/if raw_load(history_count) < 10 {/if raw_load(history_count) < 9 {/|history capacity|catch"
  "s/raw_store(history_count, 0);/raw_store(history_count, 1);/|history_count reset value|catch"
  "s/raw_store(\&raw mut (\*entry).status, STATUS_SUCCESS);/raw_store(\&raw mut (*entry).status, STATUS_WARNING);/|recorded status|catch"
  "s/allocate_results(10)/allocate_results(9)/|lazy allocation size|catch"
  "s/    base.wrapping_offset(index as isize)/    base.wrapping_offset(index as isize + 1)/|slot() index off-by-one|catch"
  "s/    while i < size_of::<T>() {\$/    while i + 1 < size_of::<T>() {/|raw_load\/raw_store truncated width|catch"
  "s/    let results =\$/    let _unused = count; let results =/|allocate_results (no-op refactor)|equivalent"

  # --- mathop ---
  "s/let selected_op: Operation = param3.wrapping_rem(5).wrapping_add(1);/let selected_op: Operation = param3.wrapping_rem(5).wrapping_abs().wrapping_add(1);/|mathop selected_op clamping|catch"
  "s/let second_op: Operation = param4.wrapping_add(1).wrapping_rem(5).wrapping_add(1);/let second_op: Operation = param4.wrapping_rem(5).wrapping_add(1);/|mathop second_op derivation|catch"
  "s/final_result = final_result.wrapping_add(operation_priority);/final_result = final_result.wrapping_sub(operation_priority);/|mathop priority sign|catch"
  "s/let time_modifier = (computation_time % 100) as c_int;/let time_modifier = (computation_time % 10) as c_int;/|mathop time modifier divisor|catch"
  "s/let time_modifier = (computation_time % 100) as c_int;/let time_modifier = (computation_time as c_int) % 100;/|mathop time modifier narrowing order|catch"
  "s/            computation_time as c_long,/            computation_time as c_int,/|mathop printf %ld argument width|catch"
  "s/b\"History entries: %d\\\\n\\\\0\"/b\"History entries:%d\\\\n\\\\0\"/|printf format text|catch"
  "s/            \*history_count,\$/            history_count.read().wrapping_add(1),/|mathop printed history count|catch"

  # --- provably equivalent control ---
  "s/let condition = (op_char != 0) \&\& (op_char >= ONE \&\& op_char <= FIVE);/let condition = op_char >= ONE \&\& op_char <= FIVE;/|is_valid_operation redundant null test (0 < '1' already fails)|equivalent"
)

CAUGHT=0
BAD=0
for entry in "${MUTATIONS[@]}"; do
  expr="$(echo "$entry" | cut -d'|' -f1)"
  desc="$(echo "$entry" | cut -d'|' -f2)"
  want="$(echo "$entry" | cut -d'|' -f3)"
  restore
  if ! sed -i "$expr" "$SRC" 2>/dev/null; then
    echo "!! SKIP (sed failed)           : $desc"
    BAD=$((BAD + 1))
    continue
  fi
  if cmp -s "$SRC" "$BAK"; then
    echo "!! SKIP (pattern did not match): $desc"
    BAD=$((BAD + 1))
    continue
  fi
  if ! timeout 300 cargo build --offline >/dev/null 2>&1; then
    echo "!! SKIP (mutant did not build) : $desc"
    BAD=$((BAD + 1))
    continue
  fi
  if timeout 600 cargo test --offline >/dev/null 2>&1; then
    result=survived
  else
    result=caught
  fi
  if [ "$want" = catch ] && [ "$result" = caught ]; then
    echo "caught                         : $desc"
    CAUGHT=$((CAUGHT + 1))
  elif [ "$want" = equivalent ] && [ "$result" = survived ]; then
    echo "survived (expected, equivalent): $desc"
  else
    echo "!! UNEXPECTED ($result, wanted $want): $desc"
    BAD=$((BAD + 1))
  fi
done

restore
cargo build --offline >/dev/null 2>&1
echo
echo "mutations caught: $CAUGHT   unexpected outcomes: $BAD"
[ "$BAD" = 0 ]
