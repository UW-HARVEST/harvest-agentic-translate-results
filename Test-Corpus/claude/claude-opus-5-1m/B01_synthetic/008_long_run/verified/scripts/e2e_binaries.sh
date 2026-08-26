#!/usr/bin/env bash
# CONFIGS.md rows 25/26: end-to-end differential run of the REAL artefacts —
# the CMake-built C executable vs the cargo-built Rust binary — comparing
# stdout, stderr and exit status byte-for-byte.
#
# Each run performs ITERATIONS * ARRAY_SIZE * 100 = 5.2e10 arithmetic steps
# (~4-5 minutes), so the runs are launched in parallel.
#
# usage: scripts/e2e_binaries.sh [seed ...]
set -uo pipefail
cd "$(dirname "$0")/.."

SEEDS=("$@")
if [ "${#SEEDS[@]}" -eq 0 ]; then SEEDS=(42 1 0); fi

C_DRIVER="c_src/build/driver"
if [ ! -x "$C_DRIVER" ]; then
  echo "building the C executable with CMake"
  (mkdir -p c_src/build && cd c_src/build &&
    cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null &&
    cmake --build . >/dev/null)
fi
cargo build --release >/dev/null 2>&1
R_DRIVER="target/release/driver"

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

pids=()
for seed in "${SEEDS[@]}"; do
  ("$C_DRIVER" "$seed" >"$work/c.$seed.out" 2>"$work/c.$seed.err"
   echo $? >"$work/c.$seed.status") &
  pids+=($!)
  ("$R_DRIVER" "$seed" >"$work/r.$seed.out" 2>"$work/r.$seed.err"
   echo $? >"$work/r.$seed.status") &
  pids+=($!)
done
echo "launched ${#pids[@]} runs for seeds: ${SEEDS[*]} (this takes ~5 minutes)"
for p in "${pids[@]}"; do wait "$p"; done

fail=0
for seed in "${SEEDS[@]}"; do
  ok=1
  for kind in out err status; do
    if ! cmp -s "$work/c.$seed.$kind" "$work/r.$seed.$kind"; then
      echo "MISMATCH seed=$seed ($kind):"
      echo "  C   : $(head -c 200 "$work/c.$seed.$kind")"
      echo "  Rust: $(head -c 200 "$work/r.$seed.$kind")"
      ok=0
      fail=1
    fi
  done
  if [ "$ok" -eq 1 ]; then
    printf 'seed %-12s identical: status=%s stdout=%s\n' \
      "$seed" "$(cat "$work/c.$seed.status")" "$(cat "$work/c.$seed.out" | tr -d '\n')"
  fi
done

# argv[0] is part of the usage message, so compare it with a matched argv[0].
for args in "" "a b" "abc" "-1" "4294967296" ""; do
  # shellcheck disable=SC2086
  c_out=$( (exec -a driver "$C_DRIVER" $args) 2>&1; echo "status=$?")
  # shellcheck disable=SC2086
  r_out=$( (exec -a driver "$R_DRIVER" $args) 2>&1; echo "status=$?")
  if [ "$c_out" != "$r_out" ]; then
    echo "MISMATCH argv=[$args]: C=[$c_out] Rust=[$r_out]"
    fail=1
  else
    echo "argv=[$args] identical: $(echo "$c_out" | tr '\n' '|')"
  fi
done

if [ "$fail" -ne 0 ]; then
  echo "E2E BINARY DIFFERENTIAL FAILED"
  exit 1
fi
echo "E2E BINARY DIFFERENTIAL PASSED"
