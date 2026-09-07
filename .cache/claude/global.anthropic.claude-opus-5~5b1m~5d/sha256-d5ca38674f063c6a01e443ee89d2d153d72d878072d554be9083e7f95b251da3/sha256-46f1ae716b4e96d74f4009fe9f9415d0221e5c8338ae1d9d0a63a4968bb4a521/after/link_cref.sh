#!/bin/bash
# Link one self-contained C reference .so per configuration:
#   cbuild/<cfg>/libcref.so
# It contains the *unmodified* CMake-produced object files for
#   sphincs_obj (address/fors/merkle/sign/utils/utilsx1/wots/wotsx1) + rng.c
# and pulls in lib<backend>.so via DT_NEEDED, so dlopen() of libcref.so with
# RTLD_LOCAL resolves every SPX_* backend symbol without polluting the global
# namespace (which would collide with the Rust .so under test).
#
# Usage: ./link_cref.sh <backend> <thash> <secpar>
set -e
ROOT="$(cd "$(dirname "$0")" && pwd)"
B="${1:-blake}"; T="${2:-simple}"; S="${3:-128f}"
OUT="$ROOT/cbuild/$B-$T-$S"
[ -d "$OUT" ] || { echo "no such build dir: $OUT" >&2; exit 1; }

APP_OBJS=$(find "$OUT/app/CMakeFiles/sphincs_obj.dir" -name '*.o' | sort)
RNG_OBJ="$OUT/app/CMakeFiles/sphincs_core_det.dir/src/rng.c.o"
BACKEND_DIR="$OUT/lib/$B"

gcc -shared -o "$OUT/libcref.so" \
  $APP_OBJS "$RNG_OBJ" \
  "-L$BACKEND_DIR" "-l$B" \
  "-L$ROOT/osslstub" -lcrypto \
  -Wl,-rpath,"$BACKEND_DIR" -Wl,-rpath,/usr/lib64
echo "linked $OUT/libcref.so"
