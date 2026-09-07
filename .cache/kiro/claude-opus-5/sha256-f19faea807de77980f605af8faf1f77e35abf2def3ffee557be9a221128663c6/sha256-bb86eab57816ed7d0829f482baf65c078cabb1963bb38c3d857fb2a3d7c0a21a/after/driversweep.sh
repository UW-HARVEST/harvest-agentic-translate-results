#!/bin/bash
# Compare the C and Rust `driver` (PQCgenKAT_sign) stdout byte-for-byte for
# every (backend, thash, secpar) configuration.
set -u
ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT/translation"
rc=0
: > /tmp/driversweep.txt
for b in blake haraka sha2 shake; do
  for t in robust simple; do
    for s in 128f 192f 256f 128s 192s 256s; do
      tag="${b}_${t}_${s}"
      if ! cargo build --release --offline --no-default-features \
            --features "$b,$t,$s" > /tmp/drv_build.log 2>&1; then
        echo "BUILDFAIL $tag" | tee -a /tmp/driversweep.txt; rc=1; continue
      fi
      timeout 600 "$ROOT/c_build/$tag/app/driver" > /tmp/co_$tag.txt 2>/dev/null; ce=$?
      timeout 600 ./target/release/driver > /tmp/ro_$tag.txt 2>/dev/null; re=$?
      if [ "$ce" -ne "$re" ]; then
        echo "EXITDIFF $tag c=$ce rs=$re" | tee -a /tmp/driversweep.txt; rc=1; continue
      fi
      if cmp -s /tmp/co_$tag.txt /tmp/ro_$tag.txt; then
        echo "OK $tag $(cat /tmp/co_$tag.txt)" >> /tmp/driversweep.txt
      else
        echo "STDOUTDIFF $tag" | tee -a /tmp/driversweep.txt
        echo "  C : $(cat /tmp/co_$tag.txt)" | tee -a /tmp/driversweep.txt
        echo "  RS: $(cat /tmp/ro_$tag.txt)" | tee -a /tmp/driversweep.txt
        rc=1
      fi
    done
  done
done
echo "driver sweep rc=$rc  ok=$(grep -c '^OK' /tmp/driversweep.txt)"
exit $rc
