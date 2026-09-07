#!/bin/bash
# Symbol-parity check.  The C surface is the union of the three shared objects
# CMake produces for a configuration:
#   app/libsphincs_core.so      (core + randombytes.c -> /dev/urandom)
#   app/libsphincs_core_det.so  (core + rng.c         -> NIST AES-CTR-DRBG)
#   lib/<be>/lib<be>.so         (hash backend + utils.c)
# Every symbol any of them exports must also be exported by the Rust cdylib
# built with the matching feature combination.
ROOT="$(cd "$(dirname "$0")" && pwd)"
FAIL=0
for be in haraka sha2 shake blake; do
  for th in robust simple; do
    for sp in 128s 128f 192s 192f 256s 256f; do
      cdir="$ROOT/cbuild/${be}_${th}_${sp}"
      rso="$ROOT/rustlib/libsphincs_plus_${be}_${th}_${sp}.so"
      nm -D --defined-only "$cdir/app/libsphincs_core.so" \
                           "$cdir/app/libsphincs_core_det.so" \
                           "$cdir/lib/$be/lib$be.so" 2>/dev/null \
        | awk 'NF==3{print $3}' | grep -v '^_' | sort -u > /tmp/sp_c.txt
      nm -D --defined-only "$rso" | awk 'NF==3{print $3}' | grep -v '^_' | sort -u > /tmp/sp_r.txt
      miss=$(comm -23 /tmp/sp_c.txt /tmp/sp_r.txt)
      if [ -n "$miss" ]; then
        echo "MISSING $be,$th,$sp: $(echo $miss | tr '\n' ' ')"
        FAIL=1
      else
        echo "OK      $be,$th,$sp ($(wc -l < /tmp/sp_c.txt) C symbols, all present in Rust)"
      fi
    done
  done
done
exit $FAIL
