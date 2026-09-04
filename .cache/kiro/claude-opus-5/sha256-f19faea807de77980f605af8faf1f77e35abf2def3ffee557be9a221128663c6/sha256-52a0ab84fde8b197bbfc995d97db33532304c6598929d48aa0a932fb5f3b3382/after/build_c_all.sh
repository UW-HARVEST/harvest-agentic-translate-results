#!/bin/bash
# Build the C reference shared libraries for every HASH_BACKEND x THASH x SECPAR
# combination into cbuild/<backend>_<thash>_<secpar>/.
#
# Nothing inside c_src/ is modified: cmake is invoked with an out-of-tree binary
# directory.  openssl headers are not installed system-wide on this host, so the
# nix-store copy is added to the include path and the system libcrypto.so.3 is
# linked (rng.c needs EVP_aes_256_ecb).
set -u
ROOT="$(cd "$(dirname "$0")" && pwd)"
SRC="$ROOT/c_src"
OUT="$ROOT/cbuild"
SSLDEV=/nix/store/dbvxz51s7m6401ycyp3l38407y11hq6p-openssl-3.6.3-dev
SSLLINK=/tmp/ssllink
mkdir -p "$SSLLINK"
ln -sf /usr/lib64/libcrypto.so.3 "$SSLLINK/libcrypto.so"

mkdir -p "$OUT"
fail=0
for b in blake haraka sha2 shake; do
  for t in robust simple; do
    for s in 128s 128f 192s 192f 256s 256f; do
      tag="${b}_${t}_${s}"
      d="$OUT/$tag"
      if [ -f "$d/.ok" ]; then continue; fi
      mkdir -p "$d"
      (
        cd "$d"
        cmake "$SRC" -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
          -DHASH_BACKEND="$b" -DTHASH="$t" -DSECPAR="$s" \
          -DCMAKE_C_FLAGS="-I$SSLDEV/include" \
          -DCMAKE_SHARED_LINKER_FLAGS="-L$SSLLINK -lcrypto" \
          -DCMAKE_EXE_LINKER_FLAGS="-L$SSLLINK" > cmake.log 2>&1 &&
        cmake --build . -- -j4 > build.log 2>&1
      )
      if [ $? -eq 0 ]; then
        touch "$d/.ok"
        echo "OK   $tag"
      else
        echo "FAIL $tag"
        fail=1
      fi
    done
  done
done
exit $fail
