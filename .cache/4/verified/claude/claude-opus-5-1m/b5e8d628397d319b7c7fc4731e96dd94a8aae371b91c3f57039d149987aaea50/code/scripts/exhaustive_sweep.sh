#!/usr/bin/env bash
# CONFIGS.md row 29: run `perform_expensive_operations` over EVERY one of the
# 2^32 possible int values (16384 calls x 262144 slots) on both implementations
# and compare, sharded across processes.
#
# usage: scripts/exhaustive_sweep.sh [shards] [impls]
#        scripts/exhaustive_sweep.sh 16 c-O2,c-O0
set -uo pipefail
cd "$(dirname "$0")/.."

SHARDS="${1:-16}"
IMPLS="${2:-c-O2}"

cargo build --release >/dev/null || exit 1
cargo test --release --test exhaustive --no-run >/dev/null 2>&1 || exit 1
BIN="$(find target/release/deps -maxdepth 1 -name 'exhaustive-*' -type f -newermt '-1 day' \
  -printf '%T@ %p\n' | sort -rn | head -1 | cut -d' ' -f2-)"
[ -x "$BIN" ] || { echo "cannot find the exhaustive test binary"; exit 1; }
echo "test binary: $BIN"
echo "sharding 16384 chunks (2^32 values) across $SHARDS processes, IMPLS=$IMPLS"

logdir="$(mktemp -d)"
pids=()
for ((s = 0; s < SHARDS; s++)); do
  SHARD=$s SHARDS=$SHARDS IMPLS=$IMPLS \
    "$BIN" --ignored --nocapture --exact exhaustive_domain_sweep \
    >"$logdir/shard.$s.log" 2>&1 &
  pids+=($!)
done

fail=0
for ((s = 0; s < SHARDS; s++)); do
  if ! wait "${pids[$s]}"; then
    echo "SHARD $s FAILED:"
    tail -n 15 "$logdir/shard.$s.log"
    fail=1
  fi
done

total=0
for ((s = 0; s < SHARDS; s++)); do
  line="$(grep -o 'DONE: [0-9]* chunks' "$logdir/shard.$s.log" | head -1)"
  n="$(echo "$line" | awk '{print $2}')"
  total=$((total + ${n:-0}))
  printf 'shard %2d: %s\n' "$s" "${line:-NO RESULT}"
done
echo "chunks verified: $total / 16384  (values: $((total * 262144)))"
rm -rf "$logdir"

if [ "$fail" -ne 0 ] || [ "$total" -ne 16384 ]; then
  echo "EXHAUSTIVE SWEEP FAILED"
  exit 1
fi
echo "EXHAUSTIVE SWEEP PASSED: all 4294967296 int values agree"
