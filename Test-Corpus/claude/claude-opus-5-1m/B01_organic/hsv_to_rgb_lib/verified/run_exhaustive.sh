#!/usr/bin/env bash
# Exhaustive differential sweep: for each argument slot (h, s, v) and each
# fixed-operand preset, compare the C .so and the Rust .so over ALL 2^32 bit
# patterns of that slot. Sharded across CPUs.
#
#   ./run_exhaustive.sh                # every slot x every preset (slow)
#   SLOTS="0" PRESETS="0" ./run_exhaustive.sh
set -uo pipefail
cd "$(dirname "$0")"
ROOT="$PWD"
LOGDIR="${TMPDIR:-/tmp}/harvest-exhaustive"
mkdir -p "$LOGDIR"

JOBS="${JOBS:-$(nproc)}"
SLOTS="${SLOTS:-0 1 2}"
PRESETS="${PRESETS:-0 1 2 3 4}"
TEST=cfg_38_exhaustive_one_slot_all_bits
FAIL=0

cargo build --offline --release --no-default-features >"$LOGDIR/build.log" 2>&1 || {
  echo "cargo build failed"; tail -20 "$LOGDIR/build.log"; exit 1; }
cargo test --offline --release --no-default-features --test valid_paths --no-run \
  >"$LOGDIR/testbuild.log" 2>&1 || { echo "test build failed"; tail -20 "$LOGDIR/testbuild.log"; exit 1; }

TB=$(grep -oE 'target/release/deps/valid_paths-[0-9a-f]+' "$LOGDIR/testbuild.log" | head -1)
[[ -z "$TB" ]] && TB=$(ls -t target/release/deps/valid_paths-* | grep -v '\.d$' | head -1)
echo "test binary: $TB   jobs: $JOBS"

export HARVEST_C_LIB="$ROOT/c_src/build/libtranslated_rust.so"
export HARVEST_RUST_LIB="$ROOT/target/release/libhsv_to_rgb_lib.so"

for slot in $SLOTS; do
  for preset in $PRESETS; do
    printf '\n== slot=%s preset=%s : sweeping all 2^32 bit patterns\n' "$slot" "$preset"
    pids=()
    for ((s = 0; s < JOBS; s++)); do
      SLOT=$slot PRESET=$preset SHARDS=$JOBS SHARD=$s \
        "$TB" "$TEST" --exact --ignored --nocapture \
        >"$LOGDIR/s${slot}_p${preset}_$s.log" 2>&1 &
      pids+=($!)
    done
    bad=0
    for p in "${pids[@]}"; do wait "$p" || bad=1; done
    if ((bad)); then
      echo "  [FAIL] slot=$slot preset=$preset"
      grep -h -A20 "DIVERGENCE\|panicked" "$LOGDIR/s${slot}_p${preset}_"*.log | head -30
      FAIL=$((FAIL + 1))
    else
      echo "  [ok]   4294967296 inputs matched byte-for-byte"
    fi
  done
done

printf '\n'
if ((FAIL == 0)); then echo "EXHAUSTIVE SWEEP: ALL PASSED"; else echo "EXHAUSTIVE SWEEP: $FAIL configuration(s) failed"; fi
exit $((FAIL > 0))
