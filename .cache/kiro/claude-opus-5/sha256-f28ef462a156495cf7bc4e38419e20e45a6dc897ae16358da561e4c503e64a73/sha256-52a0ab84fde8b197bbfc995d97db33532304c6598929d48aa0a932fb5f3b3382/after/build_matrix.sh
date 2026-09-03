#!/bin/bash
# Build the C reference shared libraries for every (HASH_BACKEND, THASH, SECPAR)
# combination and the matching Rust cdylib, then stash both under /tmp/dif/<combo>/.
#
# openssl headers are not installed system-wide on this host; node ships a full
# OpenSSL 3 header tree, and /usr/lib64/libcrypto.so.3 provides the runtime.
set -u
ROOT="$(cd "$(dirname "$0")" && pwd)"
NODE_INC=/local/home/scheschb/.local/share/mise/installs/node/24.19.0/include/node
OSSL_INC="-I$NODE_INC/openssl/archs/linux-x86_64/asm/include -I$NODE_INC"
mkdir -p /tmp/osslib
ln -sf /usr/lib64/libcrypto.so.3 /tmp/osslib/libcrypto.so

BACKENDS="${BACKENDS:-blake haraka sha2 shake}"
THASHES="${THASHES:-simple robust}"
SECPARS="${SECPARS:-128f 128s 192f 192s 256f 256s}"

fail=0
for b in $BACKENDS; do
  for t in $THASHES; do
    for s in $SECPARS; do
      combo="${b}_${t}_${s}"
      out=/tmp/dif/$combo
      mkdir -p "$out"

      # ---- C ----
      bdir=/tmp/cbuild/$combo
      rm -rf "$bdir"; mkdir -p "$bdir"
      if ! ( cd "$bdir" && cmake "$ROOT/c_src" \
              -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
              -DHASH_BACKEND="$b" -DTHASH="$t" -DSECPAR="$s" \
              -DCMAKE_C_FLAGS="$OSSL_INC" \
              -DCMAKE_EXE_LINKER_FLAGS=-L/tmp/osslib \
              -DCMAKE_SHARED_LINKER_FLAGS=-L/tmp/osslib \
              > cmake.log 2>&1 && cmake --build . -j 4 > build.log 2>&1 ); then
        echo "C BUILD FAIL $combo"; tail -n 15 "$bdir/build.log"; fail=1; continue
      fi
      cp "$bdir/app/libsphincs_core_det.so" "$out/libc_core_det.so"
      cp "$bdir/app/libsphincs_core.so"     "$out/libc_core.so"
      cp "$bdir/lib/$b/lib$b.so"            "$out/libc_backend.so"
      cp "$bdir/app/driver"                 "$out/c_driver"

      # ---- ground-truth parameter dump (C preprocessor is the source) ----
      if ! gcc -std=c99 -O0 -DPARAMS="sphincs-${b}-${s}" \
             -o "$bdir/dump_params" "$ROOT/harness/dump_params.c" \
             > "$bdir/dump.log" 2>&1; then
        echo "DUMP BUILD FAIL $combo"; tail -n 15 "$bdir/dump.log"; fail=1; continue
      fi
      "$bdir/dump_params" > "$out/params.txt"
      { echo "COMBO=$combo"; echo "HASH_BACKEND=$b"; echo "THASH=$t"; echo "SECPAR=$s"; } \
        >> "$out/params.txt"

      # ---- Rust ----
      if ! ( cd "$ROOT/translation" && cargo build --release --offline \
               --no-default-features --features "$b,$t,$s" \
               > /tmp/cbuild/$combo/rs.log 2>&1 ); then
        echo "RUST BUILD FAIL $combo"; tail -n 20 /tmp/cbuild/$combo/rs.log; fail=1; continue
      fi
      cp "$ROOT/translation/target/release/libsphincs_plus.so" "$out/librs.so"
      cp "$ROOT/translation/target/release/driver"             "$out/rs_driver"
    done
  done
done
echo "built $(ls -d /tmp/dif/*/ 2>/dev/null | wc -l) combos, fail=$fail"
exit $fail
