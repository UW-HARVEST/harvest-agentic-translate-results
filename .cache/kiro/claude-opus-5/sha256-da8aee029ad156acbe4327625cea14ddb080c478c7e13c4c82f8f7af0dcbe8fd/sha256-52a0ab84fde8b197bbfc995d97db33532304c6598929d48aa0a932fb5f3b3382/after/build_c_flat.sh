#!/bin/bash
# Links ONE self-contained C shared object per feature combination, from the
# same sources and the same flags CMake uses (-O3 -std=c99 -fPIC
# -DPARAMS=sphincs-<be>-<secpar>).
#
# Why: CMake splits the library into app/libsphincs_core[_det].so and
# lib/<be>/lib<be>.so which reference each other's symbols (libsphincs_core_det
# needs SPX_thash; lib<be> needs SPX_set_tree_index).  Loading that pair with
# dlopen(RTLD_GLOBAL) puts the C symbols into the global scope, where they can
# interpose — or be interposed by — the identically named Rust exports.  A
# single self-contained object can be opened RTLD_LOCAL, so the differential
# tests are guaranteed to call the implementation they asked for.
#
# The symbol-parity check (symdiff.sh) still uses the real CMake outputs.
# c_src is not modified.
ROOT="$(cd "$(dirname "$0")" && pwd)"
OUT="$ROOT/cbuild_flat"
CRYPTO=/usr/lib64/libcrypto.so.3
mkdir -p "$OUT"
FAIL=0

CORE="src/address.c src/fors.c src/merkle.c src/sign.c src/utils.c src/utilsx1.c src/wots.c src/wotsx1.c src/rng.c"

for be in haraka sha2 shake blake; do
  case $be in
    haraka) BESRC="haraka.c hash_haraka.c" ;;
    sha2)   BESRC="sha2.c hash_sha2.c" ;;
    shake)  BESRC="fips202.c hash_shake.c" ;;
    blake)  BESRC="blake256.c blake512.c hash_blake.c" ;;
  esac
  for th in robust simple; do
    for sp in 128s 128f 192s 192f 256s 256f; do
      srcs=""
      for f in $CORE; do srcs="$srcs $ROOT/c_src/app/$f"; done
      for f in $BESRC "thash_${be}_${th}.c"; do srcs="$srcs $ROOT/c_src/lib/$be/src/$f"; done
      so="$OUT/libspx_${be}_${th}_${sp}.so"
      if gcc -O3 -std=c99 -fPIC -shared -I/tmp/osslshim \
             -DPARAMS="sphincs-${be}-${sp}" \
             -o "$so" $srcs "$CRYPTO" 2> "$OUT/${be}_${th}_${sp}.log"; then
        echo "PASS $be,$th,$sp"
      else
        echo "FAIL $be,$th,$sp -> $OUT/${be}_${th}_${sp}.log"
        FAIL=1
      fi
    done
  done
done
exit $FAIL
