#!/usr/bin/env bash
# Sanity check for the differential test suite: deliberately inject a bug into
# the Rust translation, rebuild the cdylib, and assert the suite FAILS.
# A mutation that survives means the suite has a blind spot.
#
# Usage: ./mutation_check.sh
set -uo pipefail
cd "$(dirname "$0")"

SRC=src/lib.rs
BAK="$(mktemp "${TMPDIR:-/tmp}/lib.rs.orig.XXXXXX")"
cp "$SRC" "$BAK"
restore() { cp "$BAK" "$SRC"; rm -f "$BAK"; }
trap restore EXIT

# name|python-replacement (old ~~~ new), applied literally & exactly once
run_mutation() {
  local name="$1" old="$2" new="$3"
  cp "$BAK" "$SRC"
  python3 - "$SRC" "$old" "$new" <<'PY' || { echo "  SKIP  $name (pattern not found)"; return 2; }
import sys
p, old, new = sys.argv[1], sys.argv[2], sys.argv[3]
s = open(p).read()
if s.count(old) != 1:
    sys.exit(1)
open(p, "w").write(s.replace(old, new))
PY

  if ! cargo build --quiet 2>/dev/null; then
    echo "  SKIP  $name (mutant does not compile)"
    return 2
  fi
  if timeout 600 cargo test --quiet --test valid_paths --test error_paths --test smoke \
       -- --test-threads=1 >/dev/null 2>&1; then
    echo "  SURVIVED  $name   <-- BLIND SPOT"
    return 1
  else
    echo "  killed    $name"
    return 0
  fi
}

echo "=== mutation sanity check (each mutant MUST be killed) ==="
fails=0
declare -a M=(
 "complex_calc: + counter -> - counter"
 ".wrapping_add(global_counter_get())"
 ".wrapping_sub(global_counter_get())"

 "update_accumulator: *2 -> *3"
 ".wrapping_mul(2)
            .wrapping_add(value),"
 ".wrapping_mul(3)
            .wrapping_add(value),"

 "get_time_based_value: truncate -> floor"
 "((diff / 100.0) as c_int).wrapping_add(seed)"
 "(((diff / 100.0).floor()) as c_int).wrapping_add(seed)"

 "get_time_based_value: 3600 -> 3601"
 "seed.wrapping_mul(3600) as TimeT"
 "seed.wrapping_mul(3601) as TimeT"

 "compute_with_dynamic_memory: i*3 -> i*2"
 "base.wrapping_add(i.wrapping_mul(3))"
 "base.wrapping_add(i.wrapping_mul(2))"

 "shift_array_data: shift_by < size -> <= size"
 "if shift_by > 0 && shift_by < size {"
 "if shift_by > 0 && shift_by <= size {"

 "shift_array_data: memset skipped"
 "std::ptr::write_bytes(arr.add(remaining), 0u8, shift);"
 "let _ = shift;"

 "manipulate_records: bound num_records-shift -> num_records"
 "while i < num_records.wrapping_sub(shift) {"
 "while i < num_records {"

 # NOTE: `shift > 0` -> `shift >= 0` is an EQUIVALENT mutant and is deliberately
 # not listed: with shift == 0 the extra branch performs memmove(p, p, n), a
 # copy onto itself, which is unobservable. Instead mutate the same branch in
 # ways that ARE observable:
 "manipulate_records: guard shift > 0 -> shift > 1"
 "if shift > 0 && shift < num_records {"
 "if shift > 1 && shift < num_records {"

 "manipulate_records: memmove src offset shift -> shift+1"
 "records.add(shift as usize),
                records,"
 "records.add(shift as usize + 1),
                records,"

 "manipulate_records: memmove count n-shift -> n-shift-1"
 "(num_records - shift) as usize,"
 "(num_records - shift - 1) as usize,"

 "DataRecord: timestamp i64 -> i32 (ABI stride)"
 "pub type TimeT = i64;
#[cfg(all(target_pointer_width = \"32\", not(windows)))]"
 "pub type TimeT = i32;
#[cfg(all(target_pointer_width = \"32\", not(windows)))]"

 "hatch: dynamic_data[i] = p1+i -> p1+2i"
 "dynamic_data[i as usize] = param1.wrapping_add(i);"
 "dynamic_data[i as usize] = param1.wrapping_add(i.wrapping_mul(2));"

 "hatch: shift 3 -> 4"
 "shift_array_data(dynamic_data.as_mut_ptr(), 10, 3)"
 "shift_array_data(dynamic_data.as_mut_ptr(), 10, 4)"

 "hatch: records value p4+i*10 -> p4+i*11"
 "rec.value = param4.wrapping_add(i.wrapping_mul(10));"
 "rec.value = param4.wrapping_add(i.wrapping_mul(11));"

 "hatch: manipulate_records(...,5,2) -> (...,5,3)"
 "manipulate_records(records.as_mut_ptr(), 5, 2)"
 "manipulate_records(records.as_mut_ptr(), 5, 3)"

 "hatch: compute_with_dynamic_memory(p1,8) -> (p1,9)"
 "compute_with_dynamic_memory(param1, 8)"
 "compute_with_dynamic_memory(param1, 9)"

 "hatch: process_pointer_data offset 5 -> 6"
 "process_pointer_data(dynamic_data.as_mut_ptr().add(5), param2)"
 "process_pointer_data(dynamic_data.as_mut_ptr().add(6), param2)"

 "add_three: a+b+c -> a+b-c"
 "a.wrapping_add(b).wrapping_add(c)
}"
 "a.wrapping_add(b).wrapping_sub(c)
}"

 "multiply_add: a*b+c -> a*b-c"
 "a.wrapping_mul(b).wrapping_add(c)
}"
 "a.wrapping_mul(b).wrapping_sub(c)
}"

 "increment_counter: += -> -="
 "global_counter_set(global_counter_get().wrapping_add(value));"
 "global_counter_set(global_counter_get().wrapping_sub(value));"

 "process_pointer_data: + accumulator -> - accumulator"
 ".wrapping_add(global_accumulator_get())
}"
 ".wrapping_sub(global_accumulator_get())
}"
)

total=0
for ((i = 0; i < ${#M[@]}; i += 3)); do
  total=$((total + 1))
  run_mutation "${M[i]}" "${M[i+1]}" "${M[i+2]}"
  rc=$?
  [[ $rc -eq 1 ]] && fails=$((fails + 1))
  [[ $rc -eq 2 ]] && fails=$((fails + 1))
done

restore
trap - EXIT
cargo build --quiet
echo "=== $((total - fails))/$total mutants killed ==="
[[ $fails -eq 0 ]] || { echo "MUTATION CHECK FAILED"; exit 1; }
echo "MUTATION CHECK PASSED"
