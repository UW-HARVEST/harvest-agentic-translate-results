#!/bin/bash
# Build ONE self-contained C shared object per configuration, containing the
# same translation units the `driver` executable links (core + deterministic
# rng.c + the selected hash backend + the selected thash variant).
#
# A single self-contained .so lets the differential tests dlopen it with
# RTLD_LOCAL, so the C and Rust libraries (which export identical symbol names)
# can never interpose on each other.
set -u
ROOT=$HARVEST_WORKDIR
OSSL_INC=/nix/store/dbvxz51s7m6401ycyp3l38407y11hq6p-openssl-3.6.3-dev/include
OUT=$ROOT/.verify/clibs
CC=$(command -v clang || command -v gcc)
mkdir -p "$OUT"

CORE="app/src/address.c app/src/fors.c app/src/merkle.c app/src/sign.c
      app/src/utils.c app/src/utilsx1.c app/src/wots.c app/src/wotsx1.c
      app/src/rng.c"

fail=0
for b in haraka sha2 shake blake; do
  case $b in
    haraka) BSRC="lib/haraka/src/haraka.c lib/haraka/src/hash_haraka.c" ;;
    sha2)   BSRC="lib/sha2/src/sha2.c lib/sha2/src/hash_sha2.c" ;;
    shake)  BSRC="lib/shake/src/fips202.c lib/shake/src/hash_shake.c" ;;
    blake)  BSRC="lib/blake/src/blake256.c lib/blake/src/blake512.c lib/blake/src/hash_blake.c" ;;
  esac
  for t in robust simple; do
    for s in 128s 128f 192s 192f 256s 256f; do
      TR=$(printf '%s' "$b" | tr 'a-z' 'A-Z')_TR
      (cd "$ROOT/c_src" && "$CC" -shared -fPIC -O3 -std=c99 \
        "-DPARAMS=sphincs-$b-$s" "-D${TR}=1" -I"$OSSL_INC" \
        -o "$OUT/libspx_c_${b}_${t}_${s}.so" \
        $CORE $BSRC "lib/$b/src/thash_${b}_${t}.c" \
        -L"$ROOT/.verify/osslib" -lcrypto 2>"$OUT/${b}_${t}_${s}.cc.log") \
        || { echo "CC FAIL $b $t $s"; fail=$((fail+1)); }
    done
  done
done

# Also build the driver executable per configuration (Phase B stdout compare).
for b in haraka sha2 shake blake; do
  for t in robust simple; do
    for s in 128s 128f 192s 192f 256s 256f; do
      d=$ROOT/.verify/cbuilds/${b}_${t}_${s}
      [ -x "$d/app/driver" ] || { echo "NO DRIVER $b $t $s"; fail=$((fail+1)); }
    done
  done
done

echo "combined-C build failures: $fail"
ls "$OUT"/*.so | wc -l
