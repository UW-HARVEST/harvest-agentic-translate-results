#!/bin/bash
# Build the C shared libraries + driver for every (HASH_BACKEND, THASH, SECPAR)
# combination.
#
# c_src is NEVER modified.  This environment has libcrypto.so.3 but no openssl
# development headers, so a minimal header shim (declaring only the six EVP/ERR
# entry points app/src/rng.c uses) is placed on the include path via
# CMAKE_C_FLAGS, and a libcrypto.so symlink on the library path via the linker
# flags.  Everything else is the project's own CMake description.
ROOT="$(cd "$(dirname "$0")" && pwd)"
OUT="$ROOT/cbuild"
SHIM_I="-I/tmp/osslshim"
SHIM_L="-L/tmp/osslshim/lib"
mkdir -p "$OUT"
FAIL=0
for be in haraka sha2 shake blake; do
  for th in robust simple; do
    for sp in 128s 128f 192s 192f 256s 256f; do
      d="$OUT/${be}_${th}_${sp}"
      mkdir -p "$d"
      log="$d/build.log"
      if (cd "$d" && timeout 600 cmake "$ROOT/c_src" -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
             -DHASH_BACKEND=$be -DSECPAR=$sp -DTHASH=$th \
             -DCMAKE_C_FLAGS="$SHIM_I" \
             -DCMAKE_SHARED_LINKER_FLAGS="$SHIM_L" \
             -DCMAKE_EXE_LINKER_FLAGS="$SHIM_L" > "$log" 2>&1 \
          && timeout 600 cmake --build . >> "$log" 2>&1); then
        echo "PASS $be,$th,$sp"
      else
        echo "FAIL $be,$th,$sp -> $log"
        FAIL=1
      fi
    done
  done
done
exit $FAIL
