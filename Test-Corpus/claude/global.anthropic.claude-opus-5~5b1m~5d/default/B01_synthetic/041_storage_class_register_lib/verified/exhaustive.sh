#!/usr/bin/env bash
# Exhaustive differential sweep: EVERY one of the 2^32 possible `int` arguments
# is passed to both the C and the Rust `driver` and the printed bytes compared.
#
# The range is sharded across N parallel workers (default 8); each worker is the
# already-built `differential` test binary run with DRIVER_SOAK=1 and a
# DRIVER_SOAK_LO/HI window, filtered to the soak test only.
#
# Usage: ./exhaustive.sh [num_shards]
set -uo pipefail

CRATE_DIR="$(cd "$(dirname "$0")" && pwd)"
SHARDS="${1:-8}"
LOGDIR="${EXHAUSTIVE_LOGDIR:-$CRATE_DIR/exhaustive_logs}"
mkdir -p "$LOGDIR"

BIN=$(ls -t "$CRATE_DIR"/target/debug/deps/differential-* 2>/dev/null | grep -v '\.d$' | head -1)
if [ -z "$BIN" ]; then
  echo "test binary not built; run: cargo test --offline --no-run"
  exit 1
fi

LO=-2147483648
HI=2147483647
SPAN=$(( (HI - LO + 1) / SHARDS ))

pids=()
for ((i=0; i<SHARDS; i++)); do
  lo=$(( LO + i * SPAN ))
  hi=$(( lo + SPAN - 1 ))
  [ "$i" -eq $((SHARDS-1)) ] && hi=$HI
  log="$LOGDIR/exhaustive_shard_$i.log"
  DRIVER_SOAK=1 DRIVER_SOAK_LO="$lo" DRIVER_SOAK_HI="$hi" \
    "$BIN" soak_wide_sweep >"$log" 2>&1 &
  pids+=("$!")
  echo "shard $i: [$lo, $hi] -> $log (pid ${pids[-1]})"
done

fail=0
for ((i=0; i<SHARDS; i++)); do
  if wait "${pids[$i]}"; then
    echo "shard $i: PASS"
  else
    echo "shard $i: FAIL"
    tail -n 20 "$LOGDIR/exhaustive_shard_$i.log"
    fail=1
  fi
done

if [ "$fail" -eq 0 ]; then
  echo "EXHAUSTIVE SWEEP PASSED: all 2^32 inputs produce byte-identical output"
else
  echo "EXHAUSTIVE SWEEP FAILED"
fi
exit "$fail"
