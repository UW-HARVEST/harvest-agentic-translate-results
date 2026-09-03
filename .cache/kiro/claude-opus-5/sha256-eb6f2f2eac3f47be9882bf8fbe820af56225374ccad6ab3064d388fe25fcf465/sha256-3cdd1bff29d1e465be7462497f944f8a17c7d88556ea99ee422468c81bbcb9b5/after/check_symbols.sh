#!/usr/bin/env bash
# Independent `nm -D` parity check, run straight from the shell rather than
# through the test harness. Builds the Rust cdylib for each of the 24 OP x REPEAT
# combinations and diffs its defined dynamic symbols against the matching C .so.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
cd "$here/translation"

syms() { nm -D --defined-only "$1" | awk '{print $2, $3}' | sort; }

fail=0
for OP in add sub mul; do
  for R in 0 1 2 3 4 5 6 7; do
    timeout 600 cargo build --release --no-default-features --features "$OP,$R" \
      >/dev/null 2>&1 || { echo "BUILD FAIL $OP/$R"; fail=1; continue; }
    c="$here/cbuild/libcdriver_${OP}_${R}.so"
    r="target/release/libdriver.so"
    d=$(diff <(syms "$c") <(syms "$r"))
    if [[ -n "$d" ]]; then
      echo "SYMBOL DIFF $OP/$R:"; echo "$d" | sed 's/^/    /'; fail=1
    else
      printf 'ok  %s/%s  %s symbols identical\n' "$OP" "$R" "$(syms "$c" | wc -l)"
    fi
  done
done
[[ $fail -eq 0 ]] && echo "SYMBOL PARITY: empty diff for all 24 configurations"
exit $fail
