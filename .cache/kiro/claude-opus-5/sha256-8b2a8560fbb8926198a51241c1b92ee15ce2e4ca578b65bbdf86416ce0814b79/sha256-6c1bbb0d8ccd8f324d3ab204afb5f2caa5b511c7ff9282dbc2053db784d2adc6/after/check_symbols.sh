#!/bin/bash
# For every feature combination, build the Rust cdylib and compare its exported
# dynamic symbols against the union of the C shared libraries' exports.
set -u
ROOT="$(cd "$(dirname "$0")" && pwd)"
rc=0
for backend in haraka sha2 shake blake; do
  for thash in robust simple; do
    for secpar in 128s 128f 192s 192f 256s 256f; do
      combo="${backend},${thash},${secpar}"
      dir="$ROOT/cbuild/${backend}_${secpar}_${thash}"
      ( cd "$ROOT/translation" && cargo build --release --no-default-features --features "$combo${EXTRA_FEATURES:-}" ) \
        > /tmp/symbuild.log 2>&1 || { echo "BUILD FAIL $combo"; rc=1; continue; }
      { nm -D --defined-only "$dir/lib/$backend/lib$backend.so"
        nm -D --defined-only "$dir/app/libsphincs_core.so"
        nm -D --defined-only "$dir/app/libsphincs_core_det.so"; } \
        | awk '{print $3}' | sort -u > /tmp/c_syms.txt
      nm -D --defined-only "$ROOT/translation/target/release/libsphincsplus.so" \
        | awk '{print $3}' | sort -u > /tmp/r_syms.txt
      missing=$(comm -23 /tmp/c_syms.txt /tmp/r_syms.txt)
      if [ -n "$missing" ]; then
        echo "MISSING in Rust for $combo:"
        echo "$missing" | sed 's/^/    /'
        rc=1
      else
        echo "OK  $combo"
      fi
    done
  done
done
exit $rc
