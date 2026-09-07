#!/usr/bin/env bash
# Walks the whole 2^32 binary32 space through confuse_types(state, 1) at
# stride 8, in windows small enough that each `cargo test` invocation finishes
# inside libtest's 60s watchdog (whose message would otherwise be written into
# the redirected fd 1 and corrupt the capture).
#
# 8 windows x 128 chunks x 2^19 calls x stride 8 == 2^32 patterns of coverage.
set -uo pipefail
cd "$(dirname "$0")"

STRIDE=${STRIDE:-8}
CHUNKS=${CHUNKS:-128}
SPAN=$((CHUNKS * 524288 * STRIDE))
rc=0
begin=0
w=0
while [ "$begin" -le 4294967295 ]; do
  w=$((w + 1))
  printf 'window %d: begin=%d stride=%d chunks=%d\n' "$w" "$begin" "$STRIDE" "$CHUNKS"
  SWEEP_STRIDE=$STRIDE SWEEP_BEGIN=$begin SWEEP_CHUNKS=$CHUNKS \
    timeout 300 cargo test --release --test deep -- --ignored --nocapture \
    deep_op1_strided_full_float_space 2>&1 \
    | grep -E "SWEEP OK|panicked|diverges|mismatch|test result:" | sed 's/^/  /'
  # shellcheck disable=SC2181
  if [ ${PIPESTATUS[0]} -ne 0 ]; then echo "  WINDOW $w FAILED"; rc=1; fi
  begin=$((begin + SPAN))
done

echo
if [ $rc -eq 0 ]; then echo "FULL SWEEP PASSED ($w windows)"; else echo "FULL SWEEP FAILED"; fi
exit $rc
