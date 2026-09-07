#!/usr/bin/env bash
# Harness self-validation: inject known bugs into the Rust translation, confirm
# the differential tests catch each one, then restore the original source.
# A differential suite that cannot fail is not evidence of anything.
set -u
cd "$(dirname "$0")"

SRC=src/lib.rs
BACKUP=$(mktemp)
cp "$SRC" "$BACKUP"
restore() { cp "$BACKUP" "$SRC"; cargo build --release >/dev/null 2>&1; rm -f "$BACKUP"; }
trap restore EXIT

fail=0

# name | sed expression describing the mutation
mutate() {
  local name="$1"; shift
  cp "$BACKUP" "$SRC"
  python3 - "$SRC" "$1" "$2" <<'PY'
import sys
path, old, new = sys.argv[1], sys.argv[2], sys.argv[3]
s = open(path).read()
assert old in s, f"mutation anchor not found: {old!r}"
open(path, 'w').write(s.replace(old, new, 1))
PY
  if ! cargo build --release >/dev/null 2>&1; then
    echo "MUTANT $name : BUILD FAILED (bad mutation)"; fail=1; return
  fi
  if timeout 600 cargo test >/dev/null 2>&1; then
    echo "MUTANT $name : SURVIVED  <-- test suite is blind to this bug"; fail=1
  else
    echo "MUTANT $name : killed"
  fi
}

# 1. plain arithmetic error
mutate "multiply_add uses - instead of +" \
  'a.wrapping_mul(b).wrapping_add(c)' 'a.wrapping_mul(b).wrapping_sub(c)'

# 2. the subtle C quirk: manipulate_records' loop bound ignores the memmove guard
mutate "manipulate_records 'sane' loop bound" \
  'let limit = num_records.wrapping_sub(shift);' \
  'let limit = if shift > 0 && shift < num_records { num_records.wrapping_sub(shift) } else { num_records };'

# 3. off-by-one in the shift guard
mutate "shift_array_data guard <= instead of <" \
  'if shift_by > 0 && shift_by < size {' 'if shift_by > 0 && shift_by <= size {'

# 4. wrong static update rule
mutate "update_accumulator uses *3" \
  'GLOBAL_ACCUMULATOR.get().wrapping_mul(2)' 'GLOBAL_ACCUMULATOR.get().wrapping_mul(3)'

# 5. hatch passes the wrong record shift
mutate "hatch manipulate_records(records, 5, 3)" \
  'manipulate_records(records, 5, 2)' 'manipulate_records(records, 5, 3)'

# 6. checked instead of wrapping arithmetic in get_time_based_value
mutate "get_time_based_value drops the int-width wrap" \
  'seed.wrapping_mul(3600) as time_t' '(seed as time_t).wrapping_mul(3600)'

# 7. compute_with_dynamic_memory treats negative count as zero-size allocation
mutate "compute_with_dynamic_memory clamps count" \
  'let mut i: c_int = 0;
    while i < count {
        unsafe {
            *temp_array.offset(i as isize) = base.wrapping_add(i.wrapping_mul(3));' \
  'let mut i: c_int = 0;
    while i < count {
        unsafe {
            *temp_array.offset(i as isize) = base.wrapping_add(i.wrapping_mul(4));'

echo
if [ "$fail" -eq 0 ]; then
  echo "ALL MUTANTS KILLED — the differential suite is sensitive to these bug classes."
else
  echo "SOME MUTANTS SURVIVED — see above."
fi
exit "$fail"
