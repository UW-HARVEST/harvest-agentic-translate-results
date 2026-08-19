#!/usr/bin/env bash
# Exhaustive differential verification of ALL 2^32 `int` inputs of `driver`,
# split into shards so no single command runs longer than ~10 minutes.
#
# Usage: ./exhaustive_sweep.sh [shard_count]   (default 8)
set -u

cd "$(dirname "$0")" || exit 1
SHARDS="${1:-8}"
LOGDIR="${TMPDIR:-/tmp}"

echo "== building release artifacts"
cargo build --offline --release >/dev/null 2>&1 || {
  echo "release build failed"
  exit 1
}
[ -f c_src/build/libdriver.so ] || {
  echo "missing c_src/build/libdriver.so — build the C library first"
  exit 1
}

export DRIVER_RUST_SO="$PWD/target/release/libdriver.so"

fail=0
for ((i = 0; i < SHARDS; i++)); do
  log="$LOGDIR/exh_shard_$i.log"
  echo "== shard $i / $SHARDS  (log: $log)"
  SHARD_COUNT="$SHARDS" SHARD_INDEX="$i" timeout 590 \
    cargo test --offline --release --test exhaustive -- --ignored --nocapture \
    >"$log" 2>&1
  rc=$? # NOTE: no pipe here, so this really is cargo's status
  grep -E "SHARD .* OK|REPRODUCIBLE DIVERGENCE|bisect|unstable|panicked|test result" "$log" |
    sed 's/^/   /'
  if [ "$rc" -ne 0 ]; then
    echo "   shard $i FAILED (exit $rc)"
    fail=1
  fi
done

echo "=============================================================="
if [ "$fail" -eq 0 ]; then
  echo "EXHAUSTIVE SWEEP COMPLETE: all 4294967296 inputs byte-identical"
else
  echo "EXHAUSTIVE SWEEP FAILED"
fi
exit "$fail"
