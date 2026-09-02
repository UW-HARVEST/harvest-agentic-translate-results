#!/usr/bin/env bash
# Mutation campaign: proves the differential suite actually detects divergence.
#
# Each mutant injects one deliberate defect into translation/src/lib.rs.
#   killed    -> the suite caught it (good)
#   SURVIVED  -> the suite is blind to it (bad, unless listed as EXPECTED)
#
# EXPECTED SURVIVORS are mutants that are *observationally equivalent* through
# the C ABI, i.e. the original C library cannot distinguish them either:
#   bad_prints_index_1 / loop_off_by_one -- `source[10] = {0}` is all zeros and
#   only `data[0]` is ever printed, so any index and any loop bound >= 1 prints
#   the same "0". No caller of the C library can tell the difference, so there
#   is nothing for a differential test to observe. Documented in CONFIGS.md.
#
# src/lib.rs is always restored, including on early exit.
set -uo pipefail
cd "$(dirname "$0")/../translation" || exit 1

BAK=$(mktemp); cp src/lib.rs "$BAK"
restore() { cp "$BAK" src/lib.rs; rm -f "$BAK"; cargo build -q 2>/dev/null; }
trap restore EXIT

EXPECTED_SURVIVORS=" bad_prints_index_1 loop_off_by_one "

# name | sed expression
MUTANTS=(
  "drop_null_guard|s/if !line.is_null() {/if true {/"
  "driver_eq_one|s/if useGood != 0 {/if useGood == 1 {/"
  "driver_inverted|s/if useGood != 0 {/if useGood == 0 {/"
  "int_format_space|s|c\"%d\\\\n\"|c\"%d \"|"
  "int_format_unsigned|s|c\"%d\\\\n\"|c\"%u\\\\n\"|"
  "int_format_hex|s|c\"%d\\\\n\"|c\"%x\\\\n\"|"
  "line_no_newline|s|c\"%s\\\\n\"|c\"%s\"|"
  "line_payload_as_format|s|printf(c\"%s\\\\n\".as_ptr(), line);|printf(line);|"
  "bad_prints_index_1|s|printIntLine(unsafe { \\*data });|printIntLine(unsafe { *data.add(1) });|"
  "loop_off_by_one|s/while i < 10 {/while i < 9 {/"
  "source_nonzero|s/let source: \\[c_int; 10\\] = \\[0; 10\\];/let source: [c_int; 10] = [1; 10];/"
  "fold_bad_into_good|s/pub extern \"C\" fn bad() {/pub extern \"C\" fn bad() { return good();/"
)

killed=0; unexpected=0; UNEXPECTED=()
for m in "${MUTANTS[@]}"; do
  name="${m%%|*}"; expr="${m#*|}"
  cp "$BAK" src/lib.rs
  sed -i "$expr" src/lib.rs 2>/dev/null
  if cmp -s src/lib.rs "$BAK"; then echo "SKIP      $name (sed matched nothing)"; continue; fi
  if ! cargo build -q 2>/dev/null; then echo "SKIP      $name (does not compile)"; continue; fi
  if timeout 600 cargo test -q -- --test-threads=1 >/dev/null 2>&1; then
    if [[ "$EXPECTED_SURVIVORS" == *" $name "* ]]; then
      echo "survived  $name   (EXPECTED: observationally equivalent via the C ABI)"
    else
      echo "SURVIVED  $name   <-- suite is BLIND to this defect"
      unexpected=$((unexpected+1)); UNEXPECTED+=("$name")
    fi
  else
    if [[ "$EXPECTED_SURVIVORS" == *" $name "* ]]; then
      echo "KILLED    $name   <-- FALSE KILL: expected to be unobservable"
      unexpected=$((unexpected+1)); UNEXPECTED+=("$name(false-kill)")
    else
      echo "killed    $name"
      killed=$((killed+1))
    fi
  fi
done

echo
echo "killed=$killed  unexpected=$unexpected"
if [ "$unexpected" -ne 0 ]; then
  echo "PROBLEM: ${UNEXPECTED[*]}"
  exit 1
fi
echo "mutation campaign OK"
