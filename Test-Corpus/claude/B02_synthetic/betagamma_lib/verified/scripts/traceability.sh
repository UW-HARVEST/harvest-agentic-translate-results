#!/usr/bin/env bash
# Checks that every row of CONFIGS.md and ERRORS.md is actually referenced by a
# label inside tests/, i.e. that no row was checked off without a test.
set -uo pipefail
cd "$(dirname "$0")/.."
rc=0

check() {
  local kind="$1" file="$2" prefix="$3" max="$4"
  echo "=== $file ==="
  for n in $(seq 1 "$max"); do
    if ! grep -qE "(^|\| *)${n} \|" "$file"; then continue; fi   # row not present
    if grep -rqE "${prefix} ?#?${n}([^0-9]|$)" tests/; then
      printf '  %-14s row %-3s -> referenced in tests/\n' "$kind" "$n"
    else
      printf '  %-14s row %-3s -> *** NO TEST REFERENCE ***\n' "$kind" "$n"
      rc=1
    fi
  done
}

check CONFIGS CONFIGS.md row 46
check ERRORS  ERRORS.md  'ERRORS' 18

echo
if [ $rc -eq 0 ]; then echo "TRACEABILITY OK"; else echo "TRACEABILITY GAPS"; fi
exit $rc
