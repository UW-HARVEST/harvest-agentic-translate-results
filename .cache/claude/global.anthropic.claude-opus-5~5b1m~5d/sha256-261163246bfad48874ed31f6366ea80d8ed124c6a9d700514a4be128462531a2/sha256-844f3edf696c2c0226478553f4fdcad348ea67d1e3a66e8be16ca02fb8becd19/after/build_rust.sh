#!/bin/bash
# Builds the Rust cdylib + driver binary for every (OP, REPEAT) feature
# combination into ./rustbuild.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")" && pwd)"
OUT="$ROOT/rustbuild"
mkdir -p "$OUT"
OPS="${OPS:-add sub mul}"
REPS="${REPS:-0 1 2 3 4 5 6 7}"
for op in $OPS; do
  for rep in $REPS; do
    (cd "$ROOT/translation" && cargo build --release --no-default-features --features "$op,$rep" -q)
    cp "$ROOT/translation/target/release/libdriver.so" "$OUT/libdriver_${op}_${rep}.so"
    cp "$ROOT/translation/target/release/driver"       "$OUT/rdriver_${op}_${rep}"
  done
done
echo "built: $(ls "$OUT" | wc -l) artifacts in $OUT"
