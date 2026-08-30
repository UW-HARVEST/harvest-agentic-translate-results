#!/bin/bash
# Build the C reference implementation out-of-tree for one or all feature
# combinations.  c_src/ is never modified: every build tree lives under
# ./cbuild/<backend>_<secpar>_<thash>/.
#
# Usage:  ./build_c.sh                 # all 48 combinations
#         ./build_c.sh blake 128f simple
set -u

ROOT="$(cd "$(dirname "$0")" && pwd)"
OSSL_INC=/nix/store/djbmc5jk7ggq5hhcb1g976iadjbs99hb-openssl-3.6.3-dev/include
OSSL_LINKDIR=/tmp/ossl_link
mkdir -p "$OSSL_LINKDIR"
ln -sf /lib64/libcrypto.so.3 "$OSSL_LINKDIR/libcrypto.so"

build_one() {
  local backend="$1" secpar="$2" thash="$3"
  local dir="$ROOT/cbuild/${backend}_${secpar}_${thash}"
  if [ -f "$dir/libsphincs_all.so" ] && [ -z "${FORCE:-}" ]; then
    return 0
  fi
  mkdir -p "$dir"
  ( cmake -S "$ROOT/c_src" -B "$dir" \
      -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
      -DHASH_BACKEND="$backend" -DSECPAR="$secpar" -DTHASH="$thash" \
      -DCMAKE_C_FLAGS="-I$OSSL_INC" \
      -DCMAKE_EXE_LINKER_FLAGS="-L$OSSL_LINKDIR" \
      && cmake --build "$dir" -j 8 ) > "$dir/build.log" 2>&1
  if [ -f "$dir/app/libsphincs_core_det.so" ]; then
    # Link a stub shared object whose only purpose is to pull both C libraries
    # (which reference each other) and libcrypto into a single dlopen-able
    # *local* scope.  Without it the test harness would have to load them
    # RTLD_GLOBAL, and the C definitions would then interpose on the Rust
    # cdylib's own exported globals (e.g. DRBG_ctx).
    #
    # The DT_NEEDED order matches `target_link_libraries(driver
    # sphincs_core_det ${HASH_BACKEND} crypto)`, so symbols defined by both
    # libraries (utils.c is compiled into the SHA-2 and BLAKE backends too)
    # resolve the same way they do for the CMake `driver` target.
    echo '/* intentionally empty; see build_c.sh */' > "$dir/all_stub.c"
    cc -shared -fPIC -o "$dir/libsphincs_all.so" "$dir/all_stub.c" \
      -Wl,--no-as-needed \
      "$dir/app/libsphincs_core_det.so" \
      "$dir/lib/$backend/lib$backend.so" \
      /lib64/libcrypto.so.3 \
      -Wl,-rpath,"$dir/app" -Wl,-rpath,"$dir/lib/$backend" >> "$dir/build.log" 2>&1
    # The same, but around `sphincs_core` (randombytes.c / /dev/urandom)
    # instead of `sphincs_core_det` (rng.c), for the `urandom` feature.
    cc -shared -fPIC -o "$dir/libsphincs_all_urandom.so" "$dir/all_stub.c" \
      -Wl,--no-as-needed \
      "$dir/app/libsphincs_core.so" \
      "$dir/lib/$backend/lib$backend.so" \
      -Wl,-rpath,"$dir/app" -Wl,-rpath,"$dir/lib/$backend" >> "$dir/build.log" 2>&1
  fi
  if [ -f "$dir/libsphincs_all.so" ]; then
    echo "OK   ${backend}_${secpar}_${thash}"
  else
    echo "FAIL ${backend}_${secpar}_${thash} (see $dir/build.log)"
    return 1
  fi
}

if [ "$#" -eq 3 ]; then
  build_one "$1" "$2" "$3"
  exit $?
fi

rc=0
for backend in haraka sha2 shake blake; do
  for thash in robust simple; do
    for secpar in 128s 128f 192s 192f 256s 256f; do
      build_one "$backend" "$secpar" "$thash" || rc=1
    done
  done
done
exit $rc
