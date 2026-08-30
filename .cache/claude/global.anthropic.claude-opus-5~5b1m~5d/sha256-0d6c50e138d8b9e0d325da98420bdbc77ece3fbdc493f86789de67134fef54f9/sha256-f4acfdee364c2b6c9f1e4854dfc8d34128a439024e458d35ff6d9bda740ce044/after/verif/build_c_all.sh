#!/usr/bin/env bash
# Build the C reference shared libraries for every (backend, thash, secpar)
# combination that the CMake cache variables allow.
#
# Targets built per configuration:
#   app/libsphincs_core.so      (core + randombytes.c, /dev/urandom)
#   app/libsphincs_core_det.so  (core + rng.c, NIST AES-CTR-DRBG)  <-- used by
#                                the differential tests, because it makes the
#                                whole API deterministic, exactly like driver
#   lib/<backend>/lib<backend>.so
#   app/driver                  (the KAT transcript program)
#
# rng.c needs OpenSSL headers, which are not in /usr/include on this host; a
# nix-store OpenSSL is located automatically and injected through the CMake
# flag variables (c_src itself is never modified).
set -u
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CS="$ROOT/c_src"
LOG="$ROOT/verif/build_c_all.log"
: > "$LOG"

# --- locate OpenSSL ------------------------------------------------------
OSSL_INC="${OSSL_INC:-}"
OSSL_LIB="${OSSL_LIB:-}"
if [ -z "$OSSL_INC" ]; then
  if [ -f /usr/include/openssl/evp.h ]; then
    OSSL_INC=/usr/include
  else
    OSSL_INC=$(dirname "$(dirname "$(ls -d /nix/store/*openssl*-dev/include/openssl/evp.h 2>/dev/null | head -1)")")
  fi
fi
# The runtime library is taken from the system (the nix-store libcrypto needs a
# newer glibc than this host provides, so it cannot be linked into executables).
OSSL_CRYPTO="${OSSL_CRYPTO:-}"
if [ -z "$OSSL_CRYPTO" ]; then
  for cand in /usr/lib64/libcrypto.so /usr/lib64/libcrypto.so.3 \
              /usr/lib/x86_64-linux-gnu/libcrypto.so.3; do
    [ -f "$cand" ] && { OSSL_CRYPTO="$cand"; break; }
  done
fi
echo "OpenSSL include: $OSSL_INC" | tee -a "$LOG"
echo "OpenSSL crypto : $OSSL_CRYPTO" | tee -a "$LOG"

# `app/CMakeLists.txt` links the driver with plain `-lcrypto`, so a `libcrypto.so`
# development symlink is provided in a scratch directory (c_src stays untouched).
STUB="$ROOT/verif/osslink"
mkdir -p "$STUB"
ln -sf "$OSSL_CRYPTO" "$STUB/libcrypto.so"

EXTRA_C="-I$OSSL_INC"
# `--no-as-needed` is required because CMake emits linker *flags* before the
# object files, and the default `--as-needed` would otherwise drop libcrypto
# before rng.c's EVP_* references are seen.
EXTRA_LD="-L$STUB -Wl,-rpath,$(dirname "$OSSL_CRYPTO") -Wl,--no-as-needed -lcrypto -Wl,--as-needed"

BACKENDS="${BACKENDS:-haraka sha2 shake blake}"
SECPARS="${SECPARS:-128s 128f 192s 192f 256s 256f}"
THASHES="${THASHES:-robust simple}"

fail=0
for b in $BACKENDS; do
  for s in $SECPARS; do
    for t in $THASHES; do
      d="$CS/build-$b-$s-$t"
      mkdir -p "$d"
      {
        echo "########## $b $s $t ##########"
        cmake -S "$CS" -B "$d" -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
              -DHASH_BACKEND="$b" -DSECPAR="$s" -DTHASH="$t" \
              -DCMAKE_C_FLAGS="$EXTRA_C" \
              -DCMAKE_SHARED_LINKER_FLAGS="$EXTRA_LD" \
              -DCMAKE_EXE_LINKER_FLAGS="$EXTRA_LD" 2>&1
        cmake --build "$d" -- -j4 2>&1
      } >> "$LOG" 2>&1
      ok=1
      for f in "app/libsphincs_core.so" "app/libsphincs_core_det.so" \
               "lib/$b/lib$b.so" "app/driver"; do
        [ -f "$d/$f" ] || { ok=0; echo "  missing $f"; }
      done
      if [ $ok -eq 1 ]; then echo "OK   $b $s $t"; else echo "FAIL $b $s $t"; fail=1; fi
    done
  done
done
exit $fail
