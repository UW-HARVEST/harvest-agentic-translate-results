#!/usr/bin/env bash
# Drives tests/exhaustive.rs over EVERY residue class so that the union of the
# runs covers all 2^32 bit patterns of the swept component, with no sampling.
#
# Each chunk is a separate process (the prebuilt test binary is invoked directly,
# so there is no cargo lock contention) and JOBS chunks run concurrently, which
# keeps every individual chunk far inside the 600 s budget.
#
#   ./sweep.sh                 # full exhaustive sweep of h, s and l
#   ./sweep.sh hue             # only the three hue sweeps
#   STRIDE=64 ./sweep.sh hue   # finer chunking
set -uo pipefail
cd "$(dirname "$0")"

STRIDE=${STRIDE:-16}
JOBS=${JOBS:-8}
WHAT=${1:-all}
LOGDIR=${LOGDIR:-target/sweep-logs}
# Start from a clean slate so the coverage accounting below cannot be polluted by
# a previous run that used a different stride.
rm -rf "$LOGDIR"
mkdir -p "$LOGDIR"

echo "building test binaries..."
cargo test --offline --release --test exhaustive --no-run 2>&1 | tail -2
BIN=$(ls -t target/release/deps/exhaustive-* | grep -v '\.d$' | head -1)
echo "test binary: $BIN"

# Warm the harness's nested cdylib builds once, serially, so the parallel chunks
# below never race on cargo's build lock.
HSL_SWEEP_STRIDE=100000000 "$BIN" --exact exhaustive_hue_sweep_c_is_one_m_is_zero \
  --nocapture >/dev/null 2>&1 || { echo "warm-up FAILED"; exit 1; }

case "$WHAT" in
  hue) TESTS=(exhaustive_hue_sweep_c_is_one_m_is_zero
              exhaustive_hue_sweep_awkward_c_and_m
              exhaustive_hue_sweep_nan_chroma) ;;
  sl)  TESTS=(exhaustive_saturation_sweep exhaustive_lightness_sweep) ;;
  *)   TESTS=(exhaustive_hue_sweep_c_is_one_m_is_zero
              exhaustive_hue_sweep_awkward_c_and_m
              exhaustive_hue_sweep_nan_chroma
              exhaustive_saturation_sweep
              exhaustive_lightness_sweep) ;;
esac

FAIL=0
for t in "${TESTS[@]}"; do
  echo
  echo "=== $t : stride $STRIDE, offsets 0..$((STRIDE-1)), $JOBS at a time ==="
  # The s/l sweeps collapse their outer h/(h,s) loops so that a complete
  # stride-1 union over the swept component stays affordable.
  single=0
  case "$t" in exhaustive_saturation_sweep|exhaustive_lightness_sweep) single=1 ;; esac

  pids=()
  offs=()
  start=$(date +%s)
  for ((o = 0; o < STRIDE; o++)); do
    log="$LOGDIR/$t.$o.log"
    HSL_SWEEP_STRIDE=$STRIDE HSL_SWEEP_OFFSET=$o HSL_SWEEP_SINGLE=$single \
      timeout 590 "$BIN" --exact "$t" --nocapture >"$log" 2>&1 &
    pids+=($!)
    offs+=("$o")
    if (( ${#pids[@]} >= JOBS )); then
      for i in "${!pids[@]}"; do
        wait "${pids[$i]}" || { echo "  offset ${offs[$i]} FAILED (see $LOGDIR/$t.${offs[$i]}.log)"; FAIL=1; }
      done
      pids=(); offs=()
    fi
  done
  for i in "${!pids[@]}"; do
    wait "${pids[$i]}" || { echo "  offset ${offs[$i]} FAILED (see $LOGDIR/$t.${offs[$i]}.log)"; FAIL=1; }
  done
  end=$(date +%s)

  total=$(grep -h '^swept ' "$LOGDIR/$t".*.log 2>/dev/null | awk '{s+=$2} END {print s+0}')
  echo "  covered $total bit patterns in $((end-start))s"
  # The union of all STRIDE residue classes is exactly the whole 32-bit space, so
  # the counts must add up to 2^32 (times the number of outer h/(h,s)
  # configurations the test iterates). Anything less means a chunk was lost.
  if [ "$single" -eq 1 ]; then expect=4294967296; else
    case "$t" in
      exhaustive_hue_sweep_*) expect=4294967296 ;;
      *) expect=0 ;;   # non-single s/l sweeps iterate several configs; skip
    esac
  fi
  if [ "$expect" -ne 0 ] && [ "$total" -ne "$expect" ]; then
    echo "  COVERAGE SHORTFALL: got $total, expected $expect (2^32)"
    FAIL=1
  elif [ "$expect" -ne 0 ]; then
    echo "  coverage: COMPLETE (2^32 = $expect)"
  fi
done

echo
if [ "$FAIL" -eq 0 ]; then echo "EXHAUSTIVE SWEEP: ALL PASSED"; else echo "EXHAUSTIVE SWEEP: FAILURES"; fi
exit "$FAIL"
