#!/usr/bin/env bash
# Sensitivity (mutation) check for the differential suite.
#
# A differential suite that cannot fail proves nothing. This builds deliberately
# WRONG C libraries from the real source and substitutes each one for the Rust
# side (DIFF_LIB_B). Every mutant MUST produce failures; if any mutant passes,
# the suite is blind to that class of defect and must be strengthened.
#
# Mutants (each a one-line change to a copy of c_src/src/driver.c):
#   literal — good() stores "strinG" instead of "string"
#   branch  — driver() inverts its `if (useGood)` test
#   guard   — printLine() inverts its NULL guard
#
# c_src/ is never modified; the source is copied out first.
set -euo pipefail

crate="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
csrc="$crate/../c_src"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

cp "$csrc/src/driver.c" "$work/plain.c"
cp "$csrc/include/driver.h" "$work/driver.h"

sed 's/data = "string";/data = "strinG";/'  "$work/plain.c" > "$work/literal.c"
sed 's/if (useGood)/if (!useGood)/'          "$work/plain.c" > "$work/branch.c"
sed 's/if (line != NULL)/if (line == NULL)/' "$work/plain.c" > "$work/guard.c"

cd "$crate"
fail=0
for m in literal branch guard; do
  cc -O0 -fPIC -shared -I"$work" -o "$work/$m.so" "$work/$m.c"
  # Confirm the mutation actually changed the object.
  if cmp -s "$work/$m.so" "$work/plain.so" 2>/dev/null; then
    echo "MUTANT $m: source change had no effect"; fail=1; continue
  fi

  set +e
  DIFF_LIB_B="$work/$m.so" timeout 600 cargo test \
    --test phase_b_configs --test phase_c_errors -- --test-threads=1 \
    >"$work/$m.log" 2>&1
  rc=$?
  set -e

  n_failed=$(grep -oE '[0-9]+ failed' "$work/$m.log" | awk '{s+=$1} END{print s+0}')
  if ((rc == 0)); then
    echo "MUTANT $m: SUITE PASSED -- the suite is blind to this defect class"
    fail=1
  else
    printf 'MUTANT %-8s detected (%s failing tests)\n' "$m" "$n_failed"
  fi
done

echo
echo "control: the real translation must pass"
if timeout 600 cargo test --test phase_b_configs --test phase_c_errors -- \
     --test-threads=1 >"$work/real.log" 2>&1; then
  grep -E '^test result' "$work/real.log" | sed 's/^/  /'
else
  echo "  REAL TRANSLATION FAILED"; grep -E 'panicked|ROW ' "$work/real.log" | head -20; fail=1
fi

((fail)) && { echo "RESULT: sensitivity check FAILED"; exit 1; }
echo "RESULT: suite detects all mutants and passes the real translation"
