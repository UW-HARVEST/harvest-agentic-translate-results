#!/bin/bash
# Extra end-to-end assurance: a spread of seeds beyond the ones in the cargo
# suite, C vs Rust, full workload, all in parallel.
set -u
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
C="$ROOT/c_src/build/driver"
R="$ROOT/translation/target/release/driver"
OUT="$ROOT/scratch/sweep"
mkdir -p "$OUT"

SEEDS=(3 7 8 9 10 123 4096 65535 127773 16807 999999999 1000000000
       1073741823 1073741824 2147483646 2147483650 2863311530 3141592653
       4294967293 12345)

for s in "${SEEDS[@]}"; do
  ( "$C" "$s" > "$OUT/$s.c.out" 2> "$OUT/$s.c.err"; echo $? > "$OUT/$s.c.rc" ) &
  ( "$R" "$s" > "$OUT/$s.rs.out" 2> "$OUT/$s.rs.err"; echo $? > "$OUT/$s.rs.rc" ) &
done
wait
echo done > "$OUT/.done"
