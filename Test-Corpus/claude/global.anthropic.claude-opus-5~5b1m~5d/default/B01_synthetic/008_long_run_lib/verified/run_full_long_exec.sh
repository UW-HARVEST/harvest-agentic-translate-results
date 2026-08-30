#!/usr/bin/env bash
# CONFIGS.md rows 17 & 18 -- the full `long_exec` end-to-end differential.
#
# One `long_exec` call performs 262144 * 100 * 2000 = 5.24e10 `step()`
# evaluations: about 470 s through the C `.so` and 56 s through the optimised
# Rust `.so`. Each (library, seed) pair therefore runs as its own background
# process so that no single command comes anywhere near a 600 s budget, and the
# captured stdout is diffed across processes afterwards.
#
# Usage: ./run_full_long_exec.sh [seed ...]        (default seeds: 1 0)
set -uo pipefail
cd "$(dirname "$0")"

SEEDS=("$@")
[ ${#SEEDS[@]} -eq 0 ] && SEEDS=(1 0)

echo "== building =="
cargo build --offline --release || exit 1
cargo build --offline || exit 1
cargo test --offline --no-run 2>/dev/null || exit 1

BIN=$(ls -t target/debug/deps/long_exec_full-* 2>/dev/null | grep -v '\.d$' | head -1)
if [ -z "$BIN" ]; then echo "cannot locate the long_exec_full test binary"; exit 1; fi
echo "test binary: $BIN"

mkdir -p target/fullrun
PIDS=()
for seed in "${SEEDS[@]}"; do
  for which in c rust; do
    rm -f "target/full_run_${which}_${seed}.txt"
    log="target/fullrun/${which}_${seed}.log"
    LONG_ONLY="$which" LONG_SEED="$seed" \
      "$BIN" --ignored --exact full_long_exec_single_library --nocapture \
      >"$log" 2>&1 &
    PIDS+=($!)
    echo "launched $which seed=$seed pid=${PIDS[-1]} log=$log"
  done
done

echo "== waiting (expect ~8 minutes, all runs are concurrent) =="
FAIL=0
for pid in "${PIDS[@]}"; do
  wait "$pid" || { echo "pid $pid FAILED"; FAIL=1; }
done

echo
echo "== results =="
for seed in "${SEEDS[@]}"; do
  cf="target/full_run_c_${seed}.txt"
  rf="target/full_run_rust_${seed}.txt"
  if [ ! -s "$cf" ] || [ ! -s "$rf" ]; then
    echo "seed=$seed: MISSING OUTPUT (c='$(cat "$cf" 2>/dev/null)' rust='$(cat "$rf" 2>/dev/null)')"
    FAIL=1
    continue
  fi
  c=$(cat "$cf"); r=$(cat "$rf")
  if [ "$c" == "$r" ]; then
    echo "seed=$seed: MATCH   long_exec printed '$(echo "$c" | tr -d '\n')' from both libraries"
  else
    echo "seed=$seed: DIVERGE C='$c' RUST='$r'"
    FAIL=1
  fi
done

# The outputs for different seeds must differ, otherwise the comparison above
# would be satisfied by a constant and prove nothing.
if [ ${#SEEDS[@]} -ge 2 ]; then
  first=$(cat "target/full_run_c_${SEEDS[0]}.txt" 2>/dev/null)
  for seed in "${SEEDS[@]:1}"; do
    other=$(cat "target/full_run_c_${seed}.txt" 2>/dev/null)
    if [ -n "$first" ] && [ "$first" == "$other" ]; then
      # glibc's srand(0) aliases srand(1), so that one pair is expected to match.
      if { [ "${SEEDS[0]}" == "0" ] && [ "$seed" == "1" ]; } || \
         { [ "${SEEDS[0]}" == "1" ] && [ "$seed" == "0" ]; }; then
        echo "note: seeds 0 and 1 match, as expected (glibc srand(0) aliases srand(1))"
      else
        echo "WARNING: seeds ${SEEDS[0]} and $seed produced identical output;"
        echo "         the differential may be satisfied by a constant"
      fi
    fi
  done
fi

echo
[ $FAIL -eq 0 ] && echo "FULL LONG_EXEC DIFFERENTIAL: PASS" || echo "FULL LONG_EXEC DIFFERENTIAL: FAIL"
exit $FAIL
