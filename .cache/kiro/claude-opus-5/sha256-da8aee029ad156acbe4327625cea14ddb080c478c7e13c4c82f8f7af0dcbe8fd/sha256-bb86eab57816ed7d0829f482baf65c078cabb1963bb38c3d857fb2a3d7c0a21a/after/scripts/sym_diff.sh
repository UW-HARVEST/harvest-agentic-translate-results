#!/bin/bash
# Compare the C symbol union against the Rust cdylib exports for every
# configuration.  Prints one line per config plus the missing/extra symbols.
set -u
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
rc=0
for b in haraka sha2 shake blake; do
  for t in robust simple; do
    for s in 128s 128f 192s 192f 256s 256f; do
      "$ROOT/scripts/c_syms.sh" "$b" "$t" "$s" > /tmp/c_syms.txt
      ( cd "$ROOT/translation" && timeout 300 cargo build --release \
          --no-default-features --features "$b,$t,$s" > /tmp/rb.log 2>&1 ) \
        || { echo "BUILDFAIL $b-$t-$s"; rc=1; continue; }
      nm -D --defined-only "$ROOT/translation/target/release/libsphincs_plus.so" \
        | awk '$2 != "" {print $3}' | grep -v '^_' | sort -u > /tmp/r_syms.txt
      miss=$(comm -23 /tmp/c_syms.txt /tmp/r_syms.txt | tr '\n' ' ')
      extra=$(comm -13 /tmp/c_syms.txt /tmp/r_syms.txt | tr '\n' ' ')
      if [ -n "$miss" ] || [ -n "$extra" ]; then
        echo "DIFF $b-$t-$s  MISSING[$miss] EXTRA[$extra]"
        rc=1
      else
        echo "OK   $b-$t-$s  ($(wc -l < /tmp/c_syms.txt) symbols)"
      fi
    done
  done
done
exit $rc
