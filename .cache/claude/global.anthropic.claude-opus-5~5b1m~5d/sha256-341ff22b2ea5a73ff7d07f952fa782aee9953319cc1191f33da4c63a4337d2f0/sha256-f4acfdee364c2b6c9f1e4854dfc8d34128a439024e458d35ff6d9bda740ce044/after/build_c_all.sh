#!/bin/bash
# Builds the C reference implementation as shared libraries for every valid
# CMake cache-variable combination (HASH_BACKEND x THASH x SECPAR).
#
# Output layout:  cbuild/<backend>-<thash>-<secpar>/lib{sphincs_core_det,<backend>}.so
#
# OpenSSL comes from the nix store because the distro's -devel headers are
# absent in this environment; the linked libcrypto is functionally identical.
set -u

ROOT="$(cd "$(dirname "$0")" && pwd)"
SRC="$ROOT/c_src"
OUT="$ROOT/cbuild"

SSL_INC=$(ls -d /nix/store/*-openssl-*-dev/include 2>/dev/null | head -1)
# Link against the *system* libcrypto: the nix-store one is built against a
# newer glibc than this host provides, so a .so linked to it cannot be dlopen'd.
SSL_SO=$(ls /usr/lib64/libcrypto.so.3 /usr/lib/x86_64-linux-gnu/libcrypto.so.3 2>/dev/null | head -1)

mkdir -p "$OUT"

BACKENDS="${BACKENDS:-haraka sha2 shake blake}"
THASHES="${THASHES:-robust simple}"
SECPARS="${SECPARS:-128s 128f 192s 192f 256s 256f}"

fail=0
for b in $BACKENDS; do
  for t in $THASHES; do
    for s in $SECPARS; do
      combo="$b-$t-$s"
      d="$OUT/$combo"
      if [ -f "$d/libsphincs_core_det.so" ] && [ -f "$d/lib$b.so" ] && [ -z "${FORCE:-}" ]; then
        echo "skip $combo (already built)"
        continue
      fi
      rm -rf "$d"; mkdir -p "$d/_b"
      (
        cd "$d/_b" || exit 1
        cmake "$SRC" \
          -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
          -DHASH_BACKEND="$b" -DTHASH="$t" -DSECPAR="$s" \
          -DCMAKE_C_FLAGS="-O3 -I$SSL_INC" \
          -DCMAKE_SHARED_LINKER_FLAGS="$SSL_SO" \
          > cmake.log 2>&1
        # `driver` needs -lcrypto too, but we only need the libraries; build the
        # two shared-library targets explicitly so a driver link error does not
        # abort the whole configuration.
        cmake --build . --target sphincs_core_det > build1.log 2>&1
        cmake --build . --target "$b" > build2.log 2>&1
        cmake --build . --target sphincs_core > build3.log 2>&1
      )
      cp "$d/_b/app/libsphincs_core_det.so" "$d/" 2>/dev/null
      cp "$d/_b/app/libsphincs_core.so" "$d/" 2>/dev/null
      cp "$d/_b/lib/$b/lib$b.so" "$d/" 2>/dev/null
      # ---------------------------------------------------------------------
      # Additionally link ONE self-contained shared library holding the exact
      # same translation units (deterministic rng.c variant + the selected hash
      # backend), because that is the shape the Rust cdylib has.  CMake splits
      # them into libsphincs_core_det.so + lib<backend>.so which reference each
      # other circularly (libblake.so needs SPX_set_tree_*; libsphincs_core_det
      # needs SPX_thash), which makes dlopen-based differential testing awkward.
      # app/src/utils.c is deliberately compiled only once - CMake compiles it
      # into BOTH libraries, so the union of symbols is unchanged.
      case "$b" in
        blake) BSRC="lib/blake/src/blake256.c lib/blake/src/blake512.c lib/blake/src/hash_blake.c lib/blake/src/thash_blake_$t.c" ;;
        sha2)  BSRC="lib/sha2/src/sha2.c lib/sha2/src/hash_sha2.c lib/sha2/src/thash_sha2_$t.c" ;;
        shake) BSRC="lib/shake/src/fips202.c lib/shake/src/hash_shake.c lib/shake/src/thash_shake_$t.c" ;;
        haraka) BSRC="lib/haraka/src/haraka.c lib/haraka/src/hash_haraka.c lib/haraka/src/thash_haraka_$t.c" ;;
      esac
      CC=$(command -v clang || command -v gcc)
      ( cd "$SRC" && $CC -std=c99 -O3 -fPIC -shared -maes -msse4.1 \
          "-DPARAMS=sphincs-$b-$s" -I"$SSL_INC" \
          app/src/address.c app/src/fors.c app/src/merkle.c app/src/sign.c \
          app/src/utils.c app/src/utilsx1.c app/src/wots.c app/src/wotsx1.c \
          app/src/rng.c $BSRC \
          -o "$d/libcsphincs_all.so" "$SSL_SO" \
      ) > "$d/combined.log" 2>&1

      if [ -f "$d/libsphincs_core_det.so" ] && [ -f "$d/lib$b.so" ] && [ -f "$d/libcsphincs_all.so" ]; then
        echo "ok   $combo"
        rm -rf "$d/_b"
      else
        echo "FAIL $combo"
        tail -5 "$d/_b/build1.log" "$d/_b/build2.log" 2>/dev/null
        fail=1
      fi
    done
  done
done
exit $fail
