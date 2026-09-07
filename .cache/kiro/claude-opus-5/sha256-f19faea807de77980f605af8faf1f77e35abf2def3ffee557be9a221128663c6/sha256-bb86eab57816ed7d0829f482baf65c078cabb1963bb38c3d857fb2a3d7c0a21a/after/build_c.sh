#!/bin/bash
# Build the C reference for one (HASH_BACKEND, THASH, SECPAR) triple into
# c_build/<backend>_<thash>_<secpar>/.
#
# The host has libcrypto.so.3 but no openssl-devel headers, so the openssl
# shim in translation/c_compat is put on the include path and a libcrypto.so
# link is synthesised for -lcrypto.  c_src/ itself is never modified.
set -eu
ROOT="$(cd "$(dirname "$0")" && pwd)"
B="${1:-blake}"; T="${2:-simple}"; S="${3:-128f}"

SHIM="$ROOT/translation/c_compat"
LINKDIR="$ROOT/c_build/_link"
mkdir -p "$LINKDIR"
[ -e "$LINKDIR/libcrypto.so" ] || ln -s /usr/lib64/libcrypto.so.3 "$LINKDIR/libcrypto.so"

OUT="$ROOT/c_build/${B}_${T}_${S}"
mkdir -p "$OUT"
cd "$OUT"
cmake "$ROOT/c_src" \
  -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
  -DHASH_BACKEND="$B" -DSECPAR="$S" -DTHASH="$T" \
  -DCMAKE_C_FLAGS="-O3 -I$SHIM" \
  -DCMAKE_EXE_LINKER_FLAGS="-L$LINKDIR -Wl,-rpath,$LINKDIR" \
  -DCMAKE_SHARED_LINKER_FLAGS="-L$LINKDIR -Wl,-rpath,$LINKDIR" \
  > cmake.log 2>&1
cmake --build . -- -j4 > build.log 2>&1
echo "built $B/$T/$S"
find . -name '*.so' -o -name driver -type f | sed 's/^/  /'
