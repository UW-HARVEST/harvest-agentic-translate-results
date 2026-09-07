#!/bin/bash
# Full symbol-parity sweep: for every (backend, thash, secpar) build the Rust
# cdylib and compare its exported symbols with the union of the C .so's.
set -u
ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT/translation"
rc=0
: > /tmp/symsweep.txt
for b in blake haraka sha2 shake; do
  for t in robust simple; do
    for s in 128s 128f 192s 192f 256s 256f; do
      CD="$ROOT/c_build/${b}_${t}_${s}"
      if ! cargo build --release --offline --no-default-features \
            --features "$b,$t,$s" > /tmp/sym_build.log 2>&1; then
        echo "BUILDFAIL $b $t $s" | tee -a /tmp/symsweep.txt; rc=1; continue
      fi
      nm -D --defined-only "$CD/app/libsphincs_core_det.so" \
        "$CD/app/libsphincs_core.so" "$CD/lib/$b/lib$b.so" \
        | awk 'NF==3{print $3}' | sort -u > /tmp/cs.txt
      nm -D --defined-only target/release/libsphincsplus.so \
        | awk 'NF==3{print $3}' | sort -u > /tmp/rs.txt
      miss=$(comm -23 /tmp/cs.txt /tmp/rs.txt | tr '\n' ' ')
      extra=$(comm -13 /tmp/cs.txt /tmp/rs.txt | tr '\n' ' ')
      if [ -n "$miss" ]; then
        echo "MISSING $b $t $s : $miss" | tee -a /tmp/symsweep.txt; rc=1
      else
        echo "OK $b $t $s (C=$(wc -l </tmp/cs.txt) RS=$(wc -l </tmp/rs.txt)) extra:[$extra]" >> /tmp/symsweep.txt
      fi
    done
  done
done
echo "sweep rc=$rc"
grep -c '^OK' /tmp/symsweep.txt
exit $rc
