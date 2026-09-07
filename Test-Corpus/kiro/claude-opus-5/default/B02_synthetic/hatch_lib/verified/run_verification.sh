#!/usr/bin/env bash
# One-shot full verification: build the C .so, build the Rust cdylib, run the
# whole differential suite (Phases B, C, D), and re-check symbol parity.
set -uo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(dirname "$HERE")"

echo "== building C shared library =="
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && timeout 600 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/tmp/rv_cmake.log 2>&1 \
  && timeout 600 cmake --build . >>/tmp/rv_cmake.log 2>&1 ) \
  || { tail -20 /tmp/rv_cmake.log; exit 1; }
C_SO=$(ls "$ROOT"/c_src/build/lib*.so)
echo "   $C_SO"

echo "== building Rust cdylib (release) =="
( cd "$HERE" && timeout 600 cargo build --release >/tmp/rv_cargo.log 2>&1 ) \
  || { tail -30 /tmp/rv_cargo.log; exit 1; }
R_SO="$HERE/target/release/libhatch_lib.so"
echo "   $R_SO"

echo "== symbol parity (nm -D) =="
comm -23 <(nm -D --defined-only "$C_SO"  | awk '{print $3}' | sort -u) \
         <(nm -D --defined-only "$R_SO"  | awk '{print $3}' | sort -u) > /tmp/rv_missing.txt
if [ -s /tmp/rv_missing.txt ]; then
  echo "   MISSING FROM RUST:"; sed 's/^/     /' /tmp/rv_missing.txt; exit 1
fi
echo "   C exports $(nm -D --defined-only "$C_SO" | wc -l) symbols; 0 missing from Rust"

echo "== differential suite =="
( cd "$HERE" && timeout 600 cargo test 2>&1 | grep -E '^(running|test result|---- |thread|error)' )
