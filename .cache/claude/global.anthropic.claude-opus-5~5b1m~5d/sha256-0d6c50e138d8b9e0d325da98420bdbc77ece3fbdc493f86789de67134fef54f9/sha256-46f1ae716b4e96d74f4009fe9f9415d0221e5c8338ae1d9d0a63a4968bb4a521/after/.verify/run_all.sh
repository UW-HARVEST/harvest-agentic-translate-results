#!/bin/bash
# Run all 48 combinations in parallel (each combo's binaries are pre-staged by
# build_all_rust.sh, so no cargo invocation happens here).
ROOT=$HARVEST_WORKDIR
for b in haraka sha2 shake blake; do for t in robust simple; do
  for s in 128s 128f 192s 192f 256s 256f; do echo "$b $t $s"; done
done; done | xargs -P 12 -L 1 bash "$ROOT/.verify/run_combo.sh" > "$ROOT/.verify/all.log" 2>&1
echo "=== RESULTS ==="
sort "$ROOT/.verify/all.log"
echo "pass: $(grep -c '^PASS' "$ROOT/.verify/all.log")   fail: $(grep -c '^FAIL' "$ROOT/.verify/all.log")"
