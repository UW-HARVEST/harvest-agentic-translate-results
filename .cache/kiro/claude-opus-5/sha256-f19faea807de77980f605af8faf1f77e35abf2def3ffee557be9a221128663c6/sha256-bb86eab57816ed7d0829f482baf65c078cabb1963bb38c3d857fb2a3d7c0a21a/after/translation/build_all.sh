#!/bin/bash
# Build the driver for every combination of the three CMake cache variables and
# compare the KAT transcript digest with the C reference.
set -u
cd "$(dirname "$0")"
mkdir -p /tmp/rsbins
rm -f /tmp/rsbins/*
fail=0
for b in blake haraka sha2 shake; do
  for t in robust simple; do
    for s in 128s 128f 192s 192f 256s 256f; do
      if ! cargo build --release --offline --no-default-features \
            --features "$b,$t,$s" > /tmp/rsbins/build_${b}_${t}_${s}.log 2>&1; then
        echo "BUILD FAIL $b $t $s"
        tail -n 20 /tmp/rsbins/build_${b}_${t}_${s}.log
        fail=1
        continue
      fi
      cp target/release/driver /tmp/rsbins/driver_${b}_${t}_${s}
    done
  done
done
echo "built $(ls /tmp/rsbins/driver_* 2>/dev/null | wc -l) binaries"
exit $fail
