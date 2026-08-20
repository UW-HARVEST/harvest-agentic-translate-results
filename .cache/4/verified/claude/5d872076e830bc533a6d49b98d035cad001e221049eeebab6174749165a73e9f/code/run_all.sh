#!/usr/bin/env bash
# Full verification driver: builds the C reference library, enumerates every
# valid Cargo feature combination, and runs `cargo check` + the differential
# test suite for each of them, in both the dev and the release profile.
#
# `cargo test` does NOT relink `crate-type = ["cdylib"]` artifacts, so an
# explicit `cargo build` precedes every `cargo test` (the test harness also
# refuses to run against a stale .so — see tests/common/mod.rs::assert_fresh).
set -uo pipefail

cd "$(dirname "$0")"
CARGO_FLAGS="--offline"
fail=0

echo "=============================================================="
echo "0. Build the C reference shared library"
echo "=============================================================="
mkdir -p c_src/build
( cd c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C BUILD FAILED"; exit 1; }
ls -l c_src/build/libdriver.so

echo
echo "=============================================================="
echo "1. Enumerate feature combinations from Cargo.toml"
echo "=============================================================="
# Every feature name declared in the [features] table (if any).
FEATURES=$(awk '
  /^\[features\]/ {inf=1; next}
  /^\[/           {inf=0}
  inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {sub(/[[:space:]]*=.*/,""); print}
' Cargo.toml)

if [ -z "$FEATURES" ]; then
  echo "Cargo.toml declares NO [features] -> exactly one valid combination"
  COMBOS=("")
else
  echo "declared features: $FEATURES"
  # power set of the declared features
  mapfile -t FARR <<< "$FEATURES"
  n=${#FARR[@]}
  COMBOS=()
  for ((mask = 0; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (( mask & (1 << i) )); then
        combo="${combo:+$combo,}${FARR[$i]}"
      fi
    done
    COMBOS+=("$combo")
  done
fi
echo "combinations to verify: ${#COMBOS[@]}"

LOGDIR="$(pwd)/logs"
mkdir -p "$LOGDIR"

run () { # run <label> <cmd...>
  local label="$1"; shift
  echo "---- $label"
  if timeout 600 "$@" > "$LOGDIR/run_all.$$.log" 2>&1; then
    tail -n 3 "$LOGDIR/run_all.$$.log" | sed 's/^/     /'
  else
    echo "     *** FAILED: $*"
    tail -n 40 "$LOGDIR/run_all.$$.log" | sed 's/^/     /'
    fail=1
  fi
}


for combo in "${COMBOS[@]}"; do
  if [ -z "$combo" ]; then
    FEATFLAGS=(--no-default-features)
    label="<no features>"
  else
    FEATFLAGS=(--no-default-features --features "$combo")
    label="$combo"
  fi
  echo
  echo "=============================================================="
  echo "2. Feature combination: $label"
  echo "=============================================================="
  run "cargo check $label"                cargo check $CARGO_FLAGS "${FEATFLAGS[@]}"
  run "cargo check --tests $label"         cargo check $CARGO_FLAGS --tests "${FEATFLAGS[@]}"
  run "cargo build (dev) $label"           cargo build $CARGO_FLAGS "${FEATFLAGS[@]}"
  run "cargo test  (dev) $label"           cargo test  $CARGO_FLAGS "${FEATFLAGS[@]}"
  run "cargo build (release) $label"       cargo build $CARGO_FLAGS --release "${FEATFLAGS[@]}"
  run "cargo test  (release) $label"       cargo test  $CARGO_FLAGS --release "${FEATFLAGS[@]}"
done

# also verify the plain default build and --all-features, which for a crate
# without a [features] table are the same configuration
echo
echo "=============================================================="
echo "3. Default and --all-features builds"
echo "=============================================================="
run "cargo build (default)"      cargo build $CARGO_FLAGS
run "cargo test  (default)"      cargo test  $CARGO_FLAGS
run "cargo build (all-features)" cargo build $CARGO_FLAGS --all-features
run "cargo test  (all-features)" cargo test  $CARGO_FLAGS --all-features

echo
echo "=============================================================="
echo "4. Symbol parity (nm -D)"
echo "=============================================================="
for prof in debug release; do
  so="target/$prof/libdriver.so"
  [ -f "$so" ] || continue
  cdefs=$(nm -D --defined-only c_src/build/libdriver.so | awk '$2 ~ /^[TDBR]$/ {print $3}' | sort -u)
  rdefs=$(nm -D --defined-only "$so"                    | awk '$2 ~ /^[TDBR]$/ {print $3}' | sort -u)
  missing=$(comm -23 <(echo "$cdefs") <(echo "$rdefs"))
  echo "profile=$prof  C exports: $(echo "$cdefs" | wc -l)  Rust exports: $(echo "$rdefs" | wc -l)"
  if [ -n "$missing" ]; then
    echo "  *** MISSING FROM RUST .so:"; echo "$missing" | sed 's/^/      /'
    fail=1
  else
    echo "  symbol diff EMPTY (every C export is present in the Rust .so)"
  fi
done

echo
if [ "$fail" -eq 0 ]; then
  echo "ALL CONFIGURATIONS PASSED"
else
  echo "FAILURES PRESENT (see above)"
fi
exit $fail
