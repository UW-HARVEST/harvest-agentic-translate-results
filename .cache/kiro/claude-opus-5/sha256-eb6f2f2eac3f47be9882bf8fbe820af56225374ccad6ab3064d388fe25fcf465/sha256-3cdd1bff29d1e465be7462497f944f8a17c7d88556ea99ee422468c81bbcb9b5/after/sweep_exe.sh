#!/usr/bin/env bash
# Quick executable-level sweep: C reference (cbuild/exe_<op>_<r>/driver) vs the
# Rust `driver` binary, over all 24 OP x REPEAT configurations.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
cd "$here/translation"

INPUTS=("7 3" "0 0" "-5 9" "3 -4" "1 1" "2147483647 2" "-2147483648 -1"
        "  -12abc +9" "99999999999999999999 3" "-99999999999999999999 3"
        "-9223372036854775808 1" "9223372036854775807 1" "12x 7" "" "5")

fail=0
for OP in add sub mul; do
  for R in 0 1 2 3 4 5 6 7; do
    cargo build --release --no-default-features --features "$OP,$R" >/dev/null 2>&1 || {
      echo "RUST BUILD FAIL $OP $R"; fail=1; continue; }
    cref="$here/cbuild/exe_${OP}_${R}/driver"
    rbin="target/release/driver"
    for pair in "${INPUTS[@]}"; do
      cout=$("$cref" $pair 2>/dev/null); cst=$?
      rout=$("$rbin" $pair 2>/dev/null); rst=$?
      if [[ "$cout" != "$rout" || "$cst" != "$rst" ]]; then
        echo "MISMATCH $OP/$R args[$pair] exit c=$cst r=$rst"
        diff <(printf '%s\n' "$cout") <(printf '%s\n' "$rout") | sed 's/^/    /'
        fail=1
      fi
    done
  done
done
[[ $fail -eq 0 ]] && echo "EXE SWEEP: ALL MATCH"
exit $fail
