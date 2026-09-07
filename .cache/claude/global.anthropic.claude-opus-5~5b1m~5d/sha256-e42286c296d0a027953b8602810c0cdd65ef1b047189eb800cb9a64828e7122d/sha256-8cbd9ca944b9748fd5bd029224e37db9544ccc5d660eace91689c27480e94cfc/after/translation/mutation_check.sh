#!/usr/bin/env bash
# Negative control for the differential harness.
#
# Injects a set of small, semantics-changing mutations into src/lib.rs one at a
# time and requires the differential suite to FAIL for each. If a mutant
# survives, the harness is not actually comparing the two libraries and every
# "pass" is vacuous. src/lib.rs is always restored from .mutation/lib.rs.orig.
set -uo pipefail
cd "$(dirname "$0")" || exit 1

ORIG=.mutation/lib.rs.orig
[[ -f $ORIG ]] || { echo "!! missing $ORIG"; exit 1; }

restore() { cp "$ORIG" src/lib.rs; }
trap 'restore; echo "(src/lib.rs restored)"' EXIT

mutate() { # <from> <to>
  python3 - "$1" "$2" <<'PY'
import sys
p = 'src/lib.rs'
s = open(p).read()
f, t = sys.argv[1], sys.argv[2]
n = s.count(f)
if n != 1:
    sys.exit(f"pattern appears {n} times, expected 1: {f!r}")
open(p, 'w').write(s.replace(f, t))
PY
}

SURVIVORS=0
KILLED=0

check() { # <desc> <from> <to>
  local desc="$1"
  restore
  if ! mutate "$2" "$3"; then
    echo "!! could not apply mutant [$desc]"
    SURVIVORS=$((SURVIVORS + 1))
    return
  fi
  # A mutant is KILLED iff `cargo test` exits non-zero. Relying on the
  # "N failed" summary line is wrong: a mutant can crash the test binary
  # (SIGSEGV) before libtest ever prints a summary.
  local out rc
  out=$(cargo test --offline --release 2>&1)
  rc=$?
  if [[ $rc -ne 0 ]]; then
    local why
    why=$(printf '%s\n' "$out" | grep -oE '[1-9][0-9]* failed|SIGSEGV|SIGABRT|SIGBUS' | head -1)
    echo "== KILLED  [$desc]  (exit=$rc ${why:-non-zero exit})"
    KILLED=$((KILLED + 1))
  else
    echo "!! SURVIVED [$desc]  (harness did not notice the injected bug!)"
    SURVIVORS=$((SURVIVORS + 1))
  fi
}

check "mode1 base_id 100 -> 101" \
  "create_entries(count, 100)" "create_entries(count, 101)"

check "mode2 base_id 200 -> 201" \
  "create_entries(count, 200)" "create_entries(count, 201)"

check "mode1 default count 5 -> 6" \
  "count = if param1 > 0 { param1 } else { 5 };" \
  "count = if param1 > 0 { param1 } else { 6 };"

check "mode2 default count 3 -> 4" \
  "count = if param1 > 0 { param1 } else { 3 };" \
  "count = if param1 > 0 { param1 } else { 4 };"

check "lookup_table cell 120 -> 121" \
  "[100, 110, 120]," "[100, 110, 121],"

check "calculate_lookup *2 -> *3" \
  "*result = temp.wrapping_mul(2);" "*result = temp.wrapping_mul(3);"

check "mode3 row bound 4 -> 3" \
  "if param1 >= 0 && param1 < 4 && param2 >= 0 && param2 < 3 {" \
  "if param1 >= 0 && param1 < 3 && param2 >= 0 && param2 < 3 {"

check "mode3 col bound 3 -> 2" \
  "if param1 >= 0 && param1 < 4 && param2 >= 0 && param2 < 3 {" \
  "if param1 >= 0 && param1 < 4 && param2 >= 0 && param2 < 2 {"

check "mode2 adds param3 unconditionally (drops the != 0 guard)" \
  "                    result = modify_entries(entries, count, param2);
                    if result != 0 {
                        result = result.wrapping_add(param3);
                    }" \
  "                    result = modify_entries(entries, count, param2);
                    result = result.wrapping_add(param3);"

check "default arm literal TestName -> TestNameX" \
  'b"TestName"' 'b"TestNameX"'

check "find_entry miss sentinel -2 -> -3" \
  "result = -2;" "result = -3;"

check "create_entries value *10 -> *11" \
  "(*e).value = base_id.wrapping_add(i).wrapping_mul(10);" \
  "(*e).value = base_id.wrapping_add(i).wrapping_mul(11);"

check "mode1 target 100 + param2 -> 99 + param2" \
  "find_entry(entries, count, 100i32.wrapping_add(param2))" \
  "find_entry(entries, count, 99i32.wrapping_add(param2))"

check "malloc size 40 -> DataEntry size ignored (allocation never fails)" \
  "let size = (count as isize as usize).wrapping_mul(core::mem::size_of::<DataEntry>());" \
  "let size = 4096usize;"

# NOTE: simply *dropping* modify_entries' `value != 0` guard is an EQUIVALENT
# mutant (value is always (200+i)*10 != 0), so it is not used here. Instead we
# accumulate the pre-multiply value, which is a genuine behaviour change.
check "modify_entries sums the pre-multiply value" \
  "                total = total.wrapping_add((*current).value);" \
  "                total = total.wrapping_add(temp_value);"

check "modify_entries iterates one entry short" \
  "    last = unsafe { entries.offset(count as isize) };" \
  "    last = unsafe { entries.offset(count as isize - 1) };"

check "find_entry returns the entry after the match" \
  "            if (*p).id == target_id {
                return p;
            }" \
  "            if (*p).id == target_id {
                return p.add(1);
            }"

# ---------------------------------------------------------------------------
# KNOWN-EQUIVALENT mutation targets — deliberately NOT used as kill targets,
# because `dataentry`'s only observable output is its `int` return value:
#
#   * `sprintf(temp_name, "Entry_%d", ...)` / `strcpy(entries[i].name, ...)`:
#     the `name` field is never read back through the public API. Mode 1 copies
#     it into the local `buffer`, which `dataentry` then discards. Overflowing
#     `name[32]` is unreachable (`base_id + i` never exceeds 15 characters for
#     any `count` whose allocation succeeds), so no name-formatting change is
#     observable.
#   * `process_name`'s return value: the default arm immediately overwrites
#     `result` with `strlen(buffer) * param1`, and `strlen("TestName") != 0`
#     always holds, so the returned length is never observable.
#   * `sizeof(DataEntry)` / field layout: the malloc size and the iteration
#     stride are derived from the same constant, so a different layout stays
#     internally consistent and produces identical results.
#   * dropping `modify_entries`' `value != 0` guard: `(200 + i) * 10` is never
#     zero for any reachable `i`.
#
# Those paths are still translated faithfully; they simply cannot be
# distinguished from outside, which is documented in CONFIGS.md / ERRORS.md.
# ---------------------------------------------------------------------------

check "mode1 case falls through to default (switch arm removed)" \
  "            1 => {" \
  "            10001 => {"

check "mode2 case falls through to default (switch arm removed)" \
  "            2 => {" \
  "            10002 => {"

check "mode3 case falls through to default (switch arm removed)" \
  "            3 => {" \
  "            10003 => {"

check "create_entries id off by one" \
  "            (*e).id = base_id.wrapping_add(i);" \
  "            (*e).id = base_id.wrapping_add(i).wrapping_add(1);"

check "mode1 NULL/count==0 sentinel -1 -> -4" \
  "                if entries.is_null() || count == 0 {
                    result = -1;" \
  "                if entries.is_null() || count == 0 {
                    result = -4;"

check "mode2 NULL sentinel -1 -> -5" \
  "                if entries.is_null() {
                    result = -1;" \
  "                if entries.is_null() {
                    result = -5;"

echo
echo "mutants killed: $KILLED   survived: $SURVIVORS"
if [[ $SURVIVORS -ne 0 ]]; then
  echo "RESULT: HARNESS IS NOT SOUND"
  exit 1
fi
echo "RESULT: harness kills every injected mutant"
