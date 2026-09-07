#!/bin/bash
# Build every C configuration into c_build/<backend>-<thash>-<secpar>/
# Usage: build_c.sh [backend thash secpar]   (default: all)
set -u
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SHIM=/tmp/ossl_shim

build_one() {
  local b=$1 t=$2 s=$3
  local d="$ROOT/c_build/$b-$t-$s"
  mkdir -p "$d"
  ( cd "$d" && \
    timeout 600 cmake "$ROOT/c_src" \
      -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
      -DHASH_BACKEND="$b" -DSECPAR="$s" -DTHASH="$t" \
      -DCMAKE_C_FLAGS="-I$SHIM" \
      -DCMAKE_EXE_LINKER_FLAGS="-L$SHIM/lib" \
      -DCMAKE_SHARED_LINKER_FLAGS="-L$SHIM/lib" > cmake.log 2>&1 && \
    timeout 600 cmake --build . -j4 > build.log 2>&1 ) \
    && echo "OK   $b-$t-$s" || { echo "FAIL $b-$t-$s"; tail -5 "$d/build.log"; }
}

if [ $# -eq 3 ]; then
  build_one "$1" "$2" "$3"
else
  for b in haraka sha2 shake blake; do
    for t in robust simple; do
      for s in 128s 128f 192s 192f 256s 256f; do
        build_one "$b" "$t" "$s"
      done
    done
  done
fi
