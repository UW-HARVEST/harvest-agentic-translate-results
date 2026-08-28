#!/usr/bin/env bash
# Sanity-check that the differential suite actually DETECTS divergence.
# Each mutation injects one deliberate bug into src/lib.rs, rebuilds the cdylib
# and expects `cargo test` to FAIL. The file is always restored.
set -u
cd "$(dirname "$0")"
ulimit -c 0 2>/dev/null || true

SRC=src/lib.rs
# Pin the .so under test: the mutants are rebuilt into target/release, and
# `cargo test` would otherwise be free to pick up a stale target/debug artifact.
export HATCH_RUST_SO="$PWD/target/release/libhatch_lib.so"
BAK=$(mktemp "${TMPDIR:-/tmp}/lib.rs.bak.XXXXXX")
cp "$SRC" "$BAK"
restore() { cp "$BAK" "$SRC"; cargo build --release --offline >/dev/null 2>&1; }
trap restore EXIT

# Mutants that are semantically EQUIVALENT to the C and therefore *must*
# survive — surviving is the correct outcome, not a blind spot. See the note at
# the bottom of this file.
EXPECTED_SURVIVORS="int_to_size_zero_extend records_guard_le"

# name | sed expression
MUTATIONS=(
  "add_three_drops_c|s|a.wrapping_add(b).wrapping_add(c)|a.wrapping_add(b)|"
  "multiply_add_swaps_op|s|a.wrapping_mul(b).wrapping_add(c)|a.wrapping_add(b).wrapping_mul(c)|"
  "complex_calc_add_instead_of_sub|s|a.wrapping_sub(b).wrapping_mul(c)|a.wrapping_add(b).wrapping_mul(c)|"
  "counter_decrements|s|GLOBAL_COUNTER.wrapping_add(value)|GLOBAL_COUNTER.wrapping_sub(value)|"
  "accumulator_times_three|s|GLOBAL_ACCUMULATOR.wrapping_mul(2)|GLOBAL_ACCUMULATOR.wrapping_mul(3)|"
  "shift_guard_ge_zero|s|if shift_by > 0 \&\& shift_by < size|if shift_by >= 0 \&\& shift_by < size|"
  "shift_guard_le_size|s|shift_by > 0 \&\& shift_by < size|shift_by > 0 \&\& shift_by <= size|"
  "shift_memset_one_short|s|c_int_to_size(shift_by).wrapping_mul|c_int_to_size(shift_by - 1).wrapping_mul|"
  "time_divides_by_ten|s|diff / 100.0|diff / 10.0|"
  "time_widens_before_multiply|s|seed.wrapping_mul(3600) as time_t|(seed as time_t).wrapping_mul(3600)|"
  "time_saturating_cast|s|((diff / 100.0) as c_int)|(if diff / 100.0 > 2147483647.0 { c_int::MAX } else { (diff / 100.0) as c_int + 1 })|"
  "records_memmove_wrong_elem_size|s|std::mem::size_of::<DataRecord>()|std::mem::size_of::<c_int>()|"
  "records_guard_le|s|if shift > 0 \&\& shift < num_records|if shift > 0 \&\& shift <= num_records|"
  "records_loop_bound_no_wrap|s|while i < num_records.wrapping_sub(shift)|while i < num_records.saturating_sub(shift)|"
  "records_reads_id_not_value|s|(\*records.offset(i as isize)).value|(*records.offset(i as isize)).id|"
  "cwdm_step_two|s|i.wrapping_mul(3)|i.wrapping_mul(2)|"
  "cwdm_returns_count|s|^    sum$|    count|"
  "ppd_reads_next_int|s|let value: c_int = \*ptr;|let value: c_int = *ptr.offset(1);|"
  "hatch_uses_count_nine|s|compute_with_dynamic_memory(param1, 8)|compute_with_dynamic_memory(param1, 9)|"
  "hatch_shift_by_four|s|shift_array_data(dynamic_data, 10, 3)|shift_array_data(dynamic_data, 10, 4)|"
  "hatch_ppd_offset_six|s|process_pointer_data(dynamic_data.offset(5)|process_pointer_data(dynamic_data.offset(6)|"
  "hatch_records_shift_three|s|manipulate_records(records, 5, 2)|manipulate_records(records, 5, 3)|"
  "hatch_swaps_mod_args|s|mod_func(param1, 999)|mod_func(param2, 999)|"
  "hatch_record_value_step|s|param4.wrapping_add(i.wrapping_mul(10))|param4.wrapping_add(i.wrapping_mul(11))|"
  "int_to_size_zero_extend|s|v as isize as usize|v as u32 as usize|"
)

pass=0; fail=0
for m in "${MUTATIONS[@]}"; do
  name="${m%%|*}"; expr="${m#*|}"
  cp "$BAK" "$SRC"
  sed -i "$expr" "$SRC"
  if cmp -s "$BAK" "$SRC"; then
    echo "SKIP  $name (pattern not found — update the script)"
    fail=$((fail+1)); continue
  fi
  if ! cargo build --release --offline >/dev/null 2>&1; then
    echo "SKIP  $name (mutant does not compile)"
    fail=$((fail+1)); continue
  fi
  out=$(cargo test --offline -- --test-threads=1 2>&1)
  killed=no
  if echo "$out" | grep -qE '^test result: FAILED|error: test failed'; then killed=yes; fi
  expected_survivor=no
  case " $EXPECTED_SURVIVORS " in *" $name "*) expected_survivor=yes;; esac

  if [ "$killed" = yes ] && [ "$expected_survivor" = no ]; then
    first=$(echo "$out" | grep -oE '^test [a-z0-9_]+ \.\.\. FAILED' | head -3 | sed 's/^test //;s/ \.\.\. FAILED//' | paste -sd, -)
    echo "KILLED   $name  (by: ${first:-<process aborted / see log>})"
    pass=$((pass+1))
  elif [ "$killed" = no ] && [ "$expected_survivor" = yes ]; then
    echo "EQUIV    $name  (expected survivor: semantically identical to the C)"
    pass=$((pass+1))
  elif [ "$killed" = yes ] && [ "$expected_survivor" = yes ]; then
    echo "UNEXPECTED-KILL $name  (listed as equivalent but the suite killed it)"
    fail=$((fail+1))
  else
    echo "SURVIVED $name  <-- the test suite is blind to this bug!"
    fail=$((fail+1))
  fi
done

echo
echo "mutants handled correctly: $pass / $((pass+fail))"
[ "$fail" -eq 0 ]

# ---------------------------------------------------------------------------
# Why `int_to_size_zero_extend` is an EQUIVALENT mutant (must survive):
#
# `c_int_to_size` only feeds byte counts:
#   * shift_array_data: reached only when 0 < shift_by < size, so both
#     `size - shift_by` and `shift_by` are strictly positive -> sign- and
#     zero-extension agree bit for bit.
#   * manipulate_records: reached only when 0 < shift < num_records, so
#     `num_records - shift` is strictly positive -> the two agree.
#   * compute_with_dynamic_memory: `count <= 0` is the only case where they
#     differ, and then the byte count is astronomically large under BOTH
#     extensions, the two `for` loops execute zero times, and the function
#     returns 0 regardless of whether malloc succeeded or returned NULL. The
#     difference is therefore unobservable through the public API.
#
# Why `records_guard_le` is an EQUIVALENT mutant (must survive):
#
# Relaxing `shift < num_records` to `shift <= num_records` in
# manipulate_records only adds the case `shift == num_records`, where the extra
# memmove copies `(num_records - shift) * 48 == 0` bytes — a no-op — and the
# read loop still has bound 0. Note the ANALOGOUS mutant for shift_array_data
# (`shift_guard_le_size`) IS killed, because there the guard also enables
# `memset(arr + (size - shift_by), 0, shift_by * 4)`, i.e. zeroing the whole
# array, which is very much observable.
# ---------------------------------------------------------------------------
