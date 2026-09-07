#!/bin/bash
# Build the Rust cdylib for every feature combination and collect the .so files
# next to the matching C build, so `nm -D` diffs can be scripted.
ROOT="$(cd "$(dirname "$0")" && pwd)"
OUT="$ROOT/rustlib"
mkdir -p "$OUT"
cd "$ROOT/translation" || exit 1
FAIL=0
for be in haraka sha2 shake blake; do
  for th in robust simple; do
    for sp in 128s 128f 192s 192f 256s 256f; do
      combo="$be,$th,$sp"
      if timeout 600 cargo build --release --no-default-features --features "$combo" \
           > "/tmp/rb_${be}_${th}_${sp}.log" 2>&1; then
        cp target/release/libsphincs_plus.so "$OUT/libsphincs_plus_${be}_${th}_${sp}.so"
        echo "PASS $combo"
      else
        echo "FAIL $combo -> /tmp/rb_${be}_${th}_${sp}.log"
        FAIL=1
      fi
    done
  done
done
exit $FAIL
