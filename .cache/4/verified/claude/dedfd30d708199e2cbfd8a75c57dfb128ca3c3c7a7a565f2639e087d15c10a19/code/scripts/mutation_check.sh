#!/usr/bin/env bash
# Validates that the differential test suite actually detects divergence from
# the C ground truth: each mutation below is injected into the Rust translation
# (src/lib.rs), the suite is run, and the mutation is expected to FAIL the suite.
#
# Usage: scripts/mutation_check.sh
set -uo pipefail

cd "$(dirname "$0")/.." || exit 1
SRC=src/lib.rs
BACKUP="$(mktemp)"
cp "$SRC" "$BACKUP"
restore() { cp "$BACKUP" "$SRC"; rm -f "$BACKUP"; }
trap restore EXIT

# name | sed expression
MUTATIONS=(
  "alias_gt_instead_of_ge|s/if \*outer >= \*inner {/if *outer > *inner {/"
  "alias_then_saturating|s/\*inner = (\*inner).wrapping_add(\*outer);/*inner = (*inner).saturating_add(*outer);/"
  "alias_else_saturating|s/\*outer = (\*outer).wrapping_add(\*inner);/*outer = (*outer).saturating_add(*inner);/"
  "inner_initial_zero|s/static mut INNER: c_int = 1;/static mut INNER: c_int = 0;/"
  "swap_error_messages|s/first argument must be an integer/1st argument must be an integer/"
  "skip_arg1_check|s/if consumed1 == 0 {/if consumed1 == 99 {/"
  "strtol_no_saturation|s/return (if negative { c_long::MIN } else { c_long::MAX }, i);/return (0, i);/"
  "loop_off_by_one|s/while i < iterations {/while i <= iterations {/"
  "printf_format|s/writeln!(out, \"{}\", \*running_sum)/writeln!(out, \" {}\", *running_sum)/"
  "strtol_drop_vtab|s/matches!(b, b' ' | b'\\\\t' | b'\\\\n' | 0x0b | 0x0c | b'\\\\r')/matches!(b, b' ' | b'\\\\t' | b'\\\\n' | 0x0c | b'\\\\r')/"
  "argc_check_relaxed|s/if argc != 3 {/if argc < 3 {/"
  "narrowing_clamped|s/let mut initial_value: c_int = raw1 as c_int;/let mut initial_value: c_int = raw1.clamp(c_int::MIN as c_long, c_int::MAX as c_long) as c_int;/"
)

pass=0
undetected=()
for entry in "${MUTATIONS[@]}"; do
  name="${entry%%|*}"
  expr="${entry#*|}"
  cp "$BACKUP" "$SRC"
  if ! sed -i "$expr" "$SRC"; then
    echo "!! $name: sed failed"; undetected+=("$name (sed failed)"); continue
  fi
  if cmp -s "$BACKUP" "$SRC"; then
    echo "!! $name: mutation did not change the source"
    undetected+=("$name (no-op mutation)")
    continue
  fi
  out=$(timeout 600 cargo test --offline 2>&1)
  rc=$?
  if [ "$rc" -ne 0 ]; then
    detail=$(printf '%s\n' "$out" | grep -E '^(test result: FAILED|error(\[|:))' | head -1)
    echo "OK   $name detected  ($detail)"
    pass=$((pass+1))
  else
    echo "FAIL $name NOT detected by the test suite"
    undetected+=("$name")
  fi
done

cp "$BACKUP" "$SRC"
echo
echo "mutations detected: $pass/${#MUTATIONS[@]}"
if [ "${#undetected[@]}" -ne 0 ]; then
  echo "undetected: ${undetected[*]}"
  exit 1
fi
echo "all mutations detected"
