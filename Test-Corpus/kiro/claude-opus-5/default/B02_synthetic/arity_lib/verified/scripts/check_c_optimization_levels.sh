#!/usr/bin/env bash
# Robustness check: does the C library behave the same at every optimization
# level, and does the Rust match all of them?
#
# `c_src/CMakeLists.txt` sets no CMAKE_BUILD_TYPE, so the graded build is -O0.
# But `arity4` relies on signed-integer overflow and on reading through an alias
# (`uninit_ptr`), both of which are UB that an optimizer is free to treat
# differently. If -O0 and -O2 disagree with each other, no Rust translation can
# match both, and that has to be known rather than assumed. This script builds
# the C at -O0/-O1/-O2/-O3/-Os OUT OF TREE (nothing under c_src/ is modified) and
# runs the full differential suite against each.
set -uo pipefail

cd "$(dirname "$0")/.." || exit 1

C_SRC="$(cd .. && pwd)/c_src"
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT

TIMEOUT=${TIMEOUT:-600}
fail=0

for opt in -O0 -O1 -O2 -O3 -Os; do
  so="$WORK/libc_$opt.so"
  if ! gcc $opt -fPIC -shared -I"$C_SRC/include" -o "$so" "$C_SRC/src/lib.c" 2>"$WORK/cc.log"; then
    echo "-- SKIP $opt (compile failed)"; cat "$WORK/cc.log"; fail=1; continue
  fi
  echo
  echo "=============================================================="
  echo "== C built with $opt"
  echo "=============================================================="
  log=$(mktemp)
  if HARVEST_C_SO="$so" timeout "$TIMEOUT" cargo test --quiet -- --test-threads=1 \
      >"$log" 2>&1; then
    grep -E '^test result:' "$log" | sed 's/^/   /'
    echo "-- PASS: C at $opt"
  else
    grep -E '^(test result:|error|thread .* panicked|assertion|  left| right)' "$log" \
      | sed 's/^/   /' | head -n 30
    echo "-- FAIL: C at $opt"
    fail=1
  fi
  rm -f "$log"
done

echo
if ((fail)); then
  echo "RESULT: the Rust does NOT match the C at every optimization level"
  exit 1
fi
echo "RESULT: the Rust matches the C at every optimization level"
