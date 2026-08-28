#!/usr/bin/env bash
# Harness self-test ("does the suite have teeth?").
#
# Injects a deliberate bug into translation/src/lib.rs, re-runs the differential
# suite, and requires it to FAIL. A mutation that survives means the suite has a
# blind spot on that code path. Two mutations of provably-dead C branches are
# included as negative controls -- those MUST survive.
set -uo pipefail
cd "$(dirname "$0")" || exit 1

SRC=src/lib.rs
BAK=$(mktemp "${TMPDIR:-/tmp}/lib.rs.bak.XXXXXX")
OUT="${TMPDIR:-/tmp}/mutation"; mkdir -p "$OUT"
cp "$SRC" "$BAK"
restore() { cp "$BAK" "$SRC"; }
trap restore EXIT

rc=0
# name | expectation(killed|survives) | sed-style old text | new text
run_mutation() {
  local name="$1" expect="$2" old="$3" new="$4"
  restore
  if ! python3 - "$SRC" "$old" "$new" <<'PY'
import sys
p, old, new = sys.argv[1], sys.argv[2], sys.argv[3]
s = open(p).read()
if s.count(old) != 1:
    print(f"PATCH-ERROR: {s.count(old)} occurrences of {old!r}")
    sys.exit(1)
open(p, "w").write(s.replace(old, new))
PY
  then printf '  \033[31mSKIP\033[0m %-42s (could not apply patch)\n' "$name"; rc=1; return; fi

  if timeout 600 cargo test --offline >"$OUT/$name.log" 2>&1; then
    result=survives
  else
    result=killed
  fi

  if [ "$result" = "$expect" ]; then
    printf '  \033[32mOK\033[0m   %-42s %s (as expected)\n' "$name" "$result"
    if [ "$result" = killed ]; then
      printf '         first divergence: %s\n' \
        "$(grep -oE 'DIVERGENCE[^\\]*' "$OUT/$name.log" | head -1 | cut -c1-110)"
    fi
  else
    printf '  \033[31mBAD\033[0m  %-42s got %s, expected %s\n' "$name" "$result" "$expect"
    rc=1
  fi
}

printf '\033[1m== mutation testing the Rust translation ==\033[0m\n'

# --- live code paths: every one of these MUST be caught --------------------
run_mutation mode1_not_found_sentinel   killed 'result = -2;'                   'result = -3;'
run_mutation mode1_base_id              killed 'create_entries(count, 100)'     'create_entries(count, 101)'
run_mutation mode1_default_count        killed 'if param1 > 0 { param1 } else { 5 }' 'if param1 > 0 { param1 } else { 6 }'
run_mutation mode1_target_offset        killed '100i32.wrapping_add(param2)'    '101i32.wrapping_add(param2)'
run_mutation mode2_default_count        killed 'if param1 > 0 { param1 } else { 3 }' 'if param1 > 0 { param1 } else { 4 }'
run_mutation mode2_base_id              killed 'create_entries(count, 200)'     'create_entries(count, 201)'
run_mutation mode2_value_scale          killed 'wrapping_mul(10)'               'wrapping_mul(11)'
run_mutation mode2_skip_param3          killed 'result = result.wrapping_add(param3);' 'result = result;'
run_mutation mode2_guard_observable     killed 'if temp_value != 0 {'           'if temp_value != 2000 {'
run_mutation mode3_double               killed 'temp.wrapping_mul(2)'           'temp.wrapping_mul(3)'
run_mutation mode3_row_bound            killed 'param1 < 4'                     'param1 <= 4'
run_mutation mode3_col_bound            killed 'param2 < 3'                     'param2 <= 3'
run_mutation mode3_table_cell           killed '[100, 110, 120]'                '[100, 110, 121]'
run_mutation default_string_len         killed 'b"TestName"'                    'b"TestNam"'
run_mutation default_prefill            killed 'b"Default"'                     'b""'
run_mutation find_entry_off_by_one      killed 'while p < end {'                'while p <= end {'
run_mutation modify_entries_bound       killed 'while current < last {'         'while current <= last {'
run_mutation alloc_size                 killed 'wrapping_mul(core::mem::size_of::<DataEntry>())' 'wrapping_mul(8)'
run_mutation switch_arm_dispatch        killed '3 => {'                          '30 => {'

# --- provably-equivalent / dead branches: these SHOULD survive -------------
# (negative controls: they prove a "killed" verdict means something, and they
#  document exactly which C branches are unreachable -- see ERRORS.md E5/E8/E11)
#
# count <= 0 is unreachable: both call sites use `param1 > 0 ? param1 : <5|3>`.
run_mutation ctl_dead_count_guard       survives 'if entries.is_null() || count <= 0 {' 'if entries.is_null() || count < 0 {'
# found->id == 0 is unreachable: ids are 100+i / 200+i for reachable counts.
run_mutation ctl_dead_found_id_guard    survives 'if found.is_null() || (*found).id == 0 {' 'if found.is_null() {'
# `if temp_value != 0` is an EQUIVALENT mutant: when temp_value == 0 the guarded
# body computes 0*multiplier == 0 and adds 0, so both branches agree exactly.
run_mutation ctl_equivalent_zero_guard  survives 'if temp_value != 0 {'           'if true {'

restore
printf '\n'
[ $rc -eq 0 ] && printf '\033[32mHARNESS SELF-TEST PASSED\033[0m\n' \
              || printf '\033[31mHARNESS SELF-TEST FOUND BLIND SPOTS\033[0m\n'
exit $rc
