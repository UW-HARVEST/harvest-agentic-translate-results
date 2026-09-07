#!/bin/bash
# Final symbol-parity sweep over all 48 C configurations.
R="$(cd "$(dirname "$0")" && pwd)"
cd "$R/translation"
tot=0
for b in blake sha2 shake haraka; do for s in 128s 128f 192s 192f 256s 256f; do for t in robust simple; do
  CB="$R/cbuild/$b-$s-$t"
  CARGO_TARGET_DIR="$R/translation/target-sym" cargo build --release --offline \
    --no-default-features --features "$b,$t,$s" >/dev/null 2>&1 || { echo "build fail $b/$s/$t"; exit 1; }
  c=$( { nm -D --defined-only "$CB/app/libsphincs_core_det.so"; \
         nm -D --defined-only "$CB/app/libsphincs_core.so"; \
         nm -D --defined-only "$CB/lib/$b/lib$b.so"; } | awk '{print $3}' | grep -v '^$' | sort -u )
  r=$(nm -D --defined-only "$R/translation/target-sym/release/libsphincs_core_det.so" | awk '{print $3}' | sort -u)
  m=$(comm -23 <(echo "$c") <(echo "$r"))
  n=$(echo "$m" | grep -c .)
  tot=$((tot+n))
  [ "$n" -gt 0 ] && echo "$b/$s/$t missing: $(echo $m)"
done; done; done
echo "TOTAL MISSING SYMBOLS ACROSS 48 C CONFIGURATIONS: $tot"
