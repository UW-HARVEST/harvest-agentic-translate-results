#!/usr/bin/env bash
# Full differential verification run.
#
#   1. build the C shared library with CMake
#   2. build the Rust cdylib (cargo test does NOT do this for a cdylib-only lib)
#   3. diff the exported symbol tables (Phase D)
#   4. run every test under every feature combination (Phase B + C)
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(dirname "$HERE")"
OFFLINE="${CARGO_OFFLINE:---offline}"

echo "== 1. build C shared library =="
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null )
C_SO="$(find "$ROOT/c_src/build" -maxdepth 1 -name '*.so' | head -1)"
echo "   C  .so: $C_SO"

echo "== 2. build Rust cdylib =="
( cd "$HERE" && cargo build $OFFLINE --release >/dev/null )
RS_SO="$HERE/target/release/libupdate_md5_lib.so"
echo "   RS .so: $RS_SO"

echo "== 3. symbol parity (Phase D) =="
nm -D --defined-only "$C_SO"  | awk '{print $3}' | sort > "${TMPDIR:-/tmp}/c_syms.txt"
nm -D --defined-only "$RS_SO" | awk '{print $3}' | sort > "${TMPDIR:-/tmp}/r_syms.txt"
MISSING="$(comm -23 "${TMPDIR:-/tmp}/c_syms.txt" "${TMPDIR:-/tmp}/r_syms.txt" || true)"
if [ -n "$MISSING" ]; then
  echo "   FAIL - symbols exported by C but missing from Rust:"
  echo "$MISSING" | sed 's/^/     /'
  exit 1
fi
echo "   OK - 0 missing symbols ($(wc -l < "${TMPDIR:-/tmp}/c_syms.txt") exported by C)"

echo "== 4. feature-combination test matrix =="
# Enumerate every declared feature; this crate declares none, so the matrix is
# {default} + {no-default-features}. The loop is generic so it keeps working if
# features are ever added.
FEATURES="$(cd "$HERE" && cargo metadata $OFFLINE --no-deps --format-version 1 \
             | tr ',' '\n' | grep -o '"features":{[^}]*}' | head -1 || true)"
echo "   declared features: ${FEATURES:-<none>}"

run() {
  echo "   --- cargo test $* ---"
  ( cd "$HERE" && cargo build $OFFLINE --release "$@" >/dev/null )
  ( cd "$HERE" && RS_SO_OVERRIDE="$RS_SO" cargo test $OFFLINE "$@" -- --test-threads=4 ) \
    | grep -E 'test result|FAILED'
}

run
run --no-default-features
run --all-features

echo "== 5. cross-optimisation matrix =="
# (a) Rust cdylib built at -O0 (debug profile) instead of --release.
( cd "$HERE" && cargo build $OFFLINE >/dev/null )
echo "   --- Rust DEBUG cdylib vs C (default -O0) ---"
( cd "$HERE" && RS_SO_OVERRIDE="$HERE/target/debug/libupdate_md5_lib.so" \
    cargo test $OFFLINE -- --test-threads=4 ) | grep -E 'test result|FAILED'

# (b) C library built at -O2 vs the Rust release cdylib.
O2DIR="$HERE/target/c_o2"
rm -rf "$O2DIR" && mkdir -p "$O2DIR"
( cd "$O2DIR" \
  && cmake "$ROOT/c_src" -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
       -DCMAKE_BUILD_TYPE=Release -DCMAKE_C_FLAGS=-O2 >/dev/null \
  && cmake --build . >/dev/null )
C_O2="$(find "$O2DIR" -maxdepth 1 -name '*.so' | head -1)"
echo "   --- Rust RELEASE cdylib vs C -O2 ---"
( cd "$HERE" && C_SO_OVERRIDE="$C_O2" RS_SO_OVERRIDE="$RS_SO" \
    cargo test $OFFLINE -- --test-threads=4 ) | grep -E 'test result|FAILED'

echo "== ALL PHASES PASSED =="
