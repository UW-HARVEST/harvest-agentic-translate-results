#!/bin/bash
# Differential test of the C executable vs the Rust executable, all configs.
ROOT="$(cd "$(dirname "$0")/.." && pwd)"; cd "$ROOT" || exit 1
fail=0; n=0
ARGSETS=(
  "3 4" "0 0" "-5 9" "9 -5" "1 -1" "-1 1"
  "2147483647 1" "2147483647 2147483647" "-2147483648 -1" "-2147483648 1"
  "-2147483648 -2147483648" "65536 65536" "46341 46341" "-46341 46341"
  "abc xyz" "  12  34" "+7 -8" "007 -007" "12abc 34def" "" "5"
  "9223372036854775808 -9223372036854775809" "99999999999999999999 -99999999999999999999"
  "2147483648 -2147483649" "0x10 010" "- -" "1e5 2e5"
)
for d in artifacts/*/; do
  cfg="$(basename "$d")"
  for args in "${ARGSETS[@]}"; do
    co="$(cd "$d/cbin" && ./driver $args 2>&1; echo "rc=$?")"
    ro="$(cd "$d/rbin" && ./driver $args 2>&1; echo "rc=$?")"
    n=$((n+1))
    if [ "$co" != "$ro" ]; then
      echo "DIFF cfg=$cfg args='$args'"
      diff <(printf '%s\n' "$co") <(printf '%s\n' "$ro") | sed 's/^/    /'
      fail=1
    fi
  done
done
echo "--- ran $n executable comparisons"
[ $fail -eq 0 ] && echo "EXECUTABLE OUTPUT IDENTICAL FOR ALL CONFIGS" || echo "EXECUTABLE DIFFERENCES FOUND"
exit $fail
