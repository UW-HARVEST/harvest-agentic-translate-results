#!/bin/bash
# Build the Rust cdylib + driver binary for every valid feature combination.
# Artifacts land in rbuild/ as librdriver_<OP>_<REPEAT>.so / rdriver_<OP>_<REPEAT>.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"
OUT="$ROOT/rbuild"
mkdir -p "$OUT"

cd "$ROOT/translation"
for op in add sub mul; do
  for rep in 0 1 2 3 4 5 6 7; do
    tag="${op}_${rep}"
    timeout 300 cargo build --release --quiet \
      --no-default-features --features "$op,repeat_$rep"
    cp target/release/libdriver.so "$OUT/librdriver_${tag}.so"
    cp target/release/driver       "$OUT/rdriver_${tag}"
  done
done
echo "built $(ls "$OUT" | wc -l) artifacts in $OUT"
