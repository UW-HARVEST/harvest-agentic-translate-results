#!/bin/bash
# Build a single self-contained C shared library for one SPHINCS+ configuration.
#
# Usage: build_c.sh <backend> <secpar> <thash>
#   backend: haraka | sha2 | shake | blake
#   secpar : 128s | 128f | 192s | 192f | 256s | 256f
#   thash  : robust | simple
#
# The resulting .so contains the union of the sources that CMake spreads over
# libsphincs_core_det.so + lib<backend>.so, so a single dlopen() handle exposes
# every public symbol (exactly like the Rust cdylib does).
set -eu

BACKEND="$1"; SECPAR="$2"; THASH="$3"
W="$(cd "$(dirname "$0")/.." && pwd)"
CSRC="$W/c_src"
OUT="$W/cbuild/${BACKEND}_${SECPAR}_${THASH}"
mkdir -p "$OUT"

# OpenSSL: this host has no openssl-devel, so the headers come from the nix
# store, while we link the *system* libcrypto (the nix one needs a newer glibc
# than /lib64/libc.so.6 provides).  OpenSSL 3.x is ABI-stable, and rng.c only
# uses the stable EVP_* / ERR_* surface.
SSL_INC=/nix/store/dbvxz51s7m6401ycyp3l38407y11hq6p-openssl-3.6.3-dev/include
SYS_CRYPTO=/usr/lib64/libcrypto.so.3

CC=clang
command -v clang >/dev/null 2>&1 || CC=gcc

APP_SRC="address.c fors.c merkle.c sign.c utils.c utilsx1.c wots.c wotsx1.c rng.c"

case "$BACKEND" in
  blake)  LIB_SRC="blake256.c blake512.c hash_blake.c thash_blake_${THASH}.c" ;;
  haraka) LIB_SRC="haraka.c hash_haraka.c thash_haraka_${THASH}.c" ;;
  sha2)   LIB_SRC="sha2.c hash_sha2.c thash_sha2_${THASH}.c" ;;
  shake)  LIB_SRC="fips202.c hash_shake.c thash_shake_${THASH}.c" ;;
  *) echo "unknown backend $BACKEND" >&2; exit 1 ;;
esac

FLAGS="-std=gnu99 -O3 -fPIC -DPARAMS=sphincs-${BACKEND}-${SECPAR} -I$SSL_INC -w"

OBJS=""
for f in $APP_SRC; do
  o="$OUT/app_${f%.c}.o"
  $CC $FLAGS -c "$CSRC/app/src/$f" -o "$o"
  OBJS="$OBJS $o"
done
for f in $LIB_SRC; do
  o="$OUT/lib_${f%.c}.o"
  $CC $FLAGS -c "$CSRC/lib/$BACKEND/src/$f" -o "$o"
  OBJS="$OBJS $o"
done

$CC -shared -o "$OUT/libc_sphincs.so" $OBJS "$SYS_CRYPTO"
echo "$OUT/libc_sphincs.so"
