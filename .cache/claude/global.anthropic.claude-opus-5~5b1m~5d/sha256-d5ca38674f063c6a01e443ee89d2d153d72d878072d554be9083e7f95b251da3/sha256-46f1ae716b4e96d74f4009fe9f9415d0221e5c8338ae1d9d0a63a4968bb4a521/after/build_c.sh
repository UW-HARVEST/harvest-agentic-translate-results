#!/bin/bash
# Build the C reference for one configuration into cbuild/<backend>-<thash>-<secpar>
# Usage: ./build_c.sh <backend> <thash> <secpar>
set -e
ROOT="$(cd "$(dirname "$0")" && pwd)"
OSSL_D=/nix/store/dbvxz51s7m6401ycyp3l38407y11hq6p-openssl-3.6.3-dev
OSSL_L="$ROOT/osslstub"
B="${1:-blake}"; T="${2:-simple}"; S="${3:-128f}"
OUT="$ROOT/cbuild/$B-$T-$S"
mkdir -p "$OUT"
cmake -S "$ROOT/c_src" -B "$OUT" \
  -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
  -DHASH_BACKEND="$B" -DSECPAR="$S" -DTHASH="$T" \
  -DCMAKE_C_FLAGS="-I$OSSL_D/include" \
  -DCMAKE_EXE_LINKER_FLAGS="-L$OSSL_L -Wl,-rpath,/usr/lib64" \
  -DCMAKE_SHARED_LINKER_FLAGS="-L$OSSL_L -Wl,-rpath,/usr/lib64" \
  > "$OUT/cmake.log" 2>&1
cmake --build "$OUT" -j8 > "$OUT/build.log" 2>&1
echo "built $B-$T-$S"
