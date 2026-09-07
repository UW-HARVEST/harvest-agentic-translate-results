#!/bin/bash
# Build the C reference implementation as shared libraries for a given config.
# Usage: ./build_c.sh <HASH_BACKEND> <SECPAR> <THASH>
# Output goes to cbuild/<backend>-<secpar>-<thash>/
set -e
R="$(cd "$(dirname "$0")" && pwd)"
BACKEND=${1:-blake}
SECPAR=${2:-128f}
THASH=${3:-simple}

SSLD=$(ls -d /nix/store/*openssl-3*-dev 2>/dev/null | head -1)
mkdir -p "$R/syslibs"
ln -sf /usr/lib64/libcrypto.so.3 "$R/syslibs/libcrypto.so"
LIBDIR="$R/syslibs"

OUT="$R/cbuild/$BACKEND-$SECPAR-$THASH"
mkdir -p "$OUT"
cd "$OUT"
cmake "$R/c_src" \
  -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
  -DHASH_BACKEND="$BACKEND" -DSECPAR="$SECPAR" -DTHASH="$THASH" \
  -DCMAKE_C_FLAGS="-I$SSLD/include" \
  -DCMAKE_EXE_LINKER_FLAGS="-L$LIBDIR -Wl,-rpath,$LIBDIR" \
  -DCMAKE_SHARED_LINKER_FLAGS="-L$LIBDIR -Wl,-rpath,$LIBDIR" > cmake.log 2>&1
cmake --build . -j8 > build.log 2>&1 || { tail -40 build.log; exit 1; }
echo "OK $BACKEND-$SECPAR-$THASH"
find "$OUT" -name '*.so' | sort
