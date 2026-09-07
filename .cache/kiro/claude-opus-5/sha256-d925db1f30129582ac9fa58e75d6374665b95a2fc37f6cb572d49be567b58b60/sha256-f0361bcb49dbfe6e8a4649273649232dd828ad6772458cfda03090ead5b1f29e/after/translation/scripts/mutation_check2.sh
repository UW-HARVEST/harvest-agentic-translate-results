#!/usr/bin/env bash
# Round 2 of the negative control: the three MISSED mutants from
# mutation_check.sh (search_buffer_text / octal_literal / findrep_needle) only
# change data that `findrep` never reads back — `message`, `final_message` and
# the case of a letter that does not move the 'p' that memchr finds. They are
# unobservable through the public ABI, so the suite is right not to flag them.
#
# This script proves the underlying code paths ARE covered, by mutating the
# same lines in ways that DO change observable output.
set -u
cd "$(dirname "$0")/.." || exit 1

SRC=src/lib.rs
BAK=$(mktemp)
cp "$SRC" "$BAK"
restore() { cp "$BAK" "$SRC"; }
trap 'restore; rm -f "$BAK"; cargo build --quiet 2>/dev/null' EXIT

run_mutant() {
  local name="$1"
  restore
  local before after
  before=$(md5sum "$SRC" | cut -d' ' -f1)
  shift
  "$@"
  after=$(md5sum "$SRC" | cut -d' ' -f1)
  if [ "$before" = "$after" ]; then
    echo "SKIP  $name (mutation matched nothing)"
    return
  fi
  if ! timeout 300 cargo build --quiet 2>/dev/null; then
    echo "SKIP  $name (does not compile)"
    return
  fi
  local out
  out=$(timeout 500 cargo test --quiet 2>&1)
  if echo "$out" | grep -q "test result: FAILED"; then
    echo "CAUGHT  $name  ($(echo "$out" | grep -c 'FAILED$') failing tests)"
  else
    echo "MISSED  $name  <-- real gap"
  fi
}

# 1. Shift the haystack so memchr's 'p' offset changes 9 -> 10.
run_mutant search_buffer_shift \
  sed -i 's/b"Function pointer example with static vars"/b"XFunction pointer example with static vars"/' "$SRC"

# 2. Drop divide_multiplier's operation_count increment (only that one).
run_mutant divide_no_count perl -0777 -i -pe \
  's/(fn divide_multiplier.*?\n\}\n)/my $b=$1; $b =~ s{\n    OPERATION_COUNT = OPERATION_COUNT.wrapping_add\(1\);}{}s; $b/se' "$SRC"

# 3. Drop add_to_accumulator's operation_count increment.
run_mutant add_no_count perl -0777 -i -pe \
  's/(fn add_to_accumulator.*?\n\}\n)/my $b=$1; $b =~ s{\n    OPERATION_COUNT = OPERATION_COUNT.wrapping_add\(1\);}{}s; $b/se' "$SRC"

# 4. Make the divide branch use a divisor of 3 instead of 2.
run_mutant divide_arg sed -i 's/selected_op(MULTIPLIER, 2);/selected_op(MULTIPLIER, 3);/' "$SRC"

# 5. Remove divide_multiplier's `b != 0` guard replacement: divide by b+1.
run_mutant divide_off_by_one sed -i 's/MULTIPLIER = MULTIPLIER.wrapping_div(b);/MULTIPLIER = MULTIPLIER.wrapping_div(b.wrapping_add(1));/' "$SRC"

# 6. Swap the subtract branch's arguments (n1, n3) -> (n3, n1).
run_mutant subtract_args sed -i 's/let subtract_result = selected_op(normalized_p1, normalized_p3);/let subtract_result = selected_op(normalized_p3, normalized_p1);/' "$SRC"

# 7. Swap the multiply branch's arguments to p1/p2 instead of p3/p4.
run_mutant multiply_args sed -i 's/result = result.wrapping_add(selected_op(normalized_p3, normalized_p4));/result = result.wrapping_add(selected_op(normalized_p1, normalized_p2));/' "$SRC"

# 8. `both_active` uses OR instead of AND.
run_mutant both_active_or sed -i 's/has_accumulator != 0 \&\& has_multiplier != 0/has_accumulator != 0 || has_multiplier != 0/' "$SRC"

# 9. mode_add threshold 1 -> 2.
run_mutant mode_add sed -i 's/let mode_add: c_int = 0o1;/let mode_add: c_int = 0o2;/' "$SRC"

# 10. process_octal_string omits the decimal field.
run_mutant octal_decimal_field sed -i 's/, Decimal: {}/, Decimal: {:x}/' "$SRC"

restore
echo "-----"
echo "round 2 done"
