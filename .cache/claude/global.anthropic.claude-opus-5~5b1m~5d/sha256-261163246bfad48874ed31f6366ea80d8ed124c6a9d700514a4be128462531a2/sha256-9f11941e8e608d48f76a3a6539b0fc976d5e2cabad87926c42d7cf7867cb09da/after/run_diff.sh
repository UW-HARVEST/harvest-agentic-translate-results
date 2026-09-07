#!/bin/bash
# Phase B+C+D driver: for every (OP, REPEAT) build configuration
#   1. build the C .so + C driver
#   2. build the Rust cdylib + Rust driver with the matching features
#   3. diff `nm -D` symbol name lists (Phase D)
#   4. run the differential test suite (Phases B and C)
# Usage: ./run_diff.sh [OP ...] [--reps "0 1 ..."]
set -uo pipefail
ROOT="$(cd "$(dirname "$0")" && pwd)"
LOGS="$ROOT/logs"
mkdir -p "$LOGS"
OPS=${OPS:-"add sub mul"}
REPS=${REPS:-"0 1 2 3 4 5 6 7"}

pass=0; fail=0; failed_cfgs=()
for op in $OPS; do for rep in $REPS; do
  tag="${op}_${rep}"
  log="$LOGS/$tag.log"
  {
    echo "########## CONFIG OP=$op REPEAT=$rep ##########"
    "$ROOT/build_c.sh" "$op" "$rep" || exit 90
    cd "$ROOT/translation" || exit 91
    timeout 300 cargo build --release --offline --no-default-features \
        --features "$op,$rep" || exit 92

    echo "--- Phase D: nm -D symbol parity ---"
    nm -D --defined-only --format=posix "$ROOT/cbuild/$tag/libdriver_c.so" \
        | awk '{print $1}' | sort -u > "$LOGS/$tag.c.syms"
    nm -D --defined-only --format=posix "$ROOT/translation/target/release/libdriver.so" \
        | awk '{print $1}' | sort -u > "$LOGS/$tag.rust.syms"
    missing=$(comm -23 "$LOGS/$tag.c.syms" "$LOGS/$tag.rust.syms")
    if [ -n "$missing" ]; then
      echo "SYMBOL DIFF NOT EMPTY - missing from Rust .so:"; echo "$missing"; exit 93
    fi
    echo "symbol diff: EMPTY ($(wc -l < "$LOGS/$tag.c.syms") C symbols all present in Rust .so)"

    echo "--- Phases B/C: differential tests ---"
    timeout 500 cargo test --release --offline --no-default-features \
        --features "$op,$rep" || exit 94
  } > "$log" 2>&1
  rc=$?
  if [ $rc -eq 0 ]; then
    printf 'PASS  OP=%-3s REPEAT=%s  %s\n' "$op" "$rep" "$(grep -m1 '^test result: ok' "$log")"
    pass=$((pass+1))
  else
    printf 'FAIL  OP=%-3s REPEAT=%s  (rc=%d) see %s\n' "$op" "$rep" "$rc" "$log"
    fail=$((fail+1)); failed_cfgs+=("$tag")
  fi
done; done

echo
echo "=============================================="
echo "configurations passed: $pass   failed: $fail"
[ $fail -ne 0 ] && echo "failed: ${failed_cfgs[*]}"
exit $fail
