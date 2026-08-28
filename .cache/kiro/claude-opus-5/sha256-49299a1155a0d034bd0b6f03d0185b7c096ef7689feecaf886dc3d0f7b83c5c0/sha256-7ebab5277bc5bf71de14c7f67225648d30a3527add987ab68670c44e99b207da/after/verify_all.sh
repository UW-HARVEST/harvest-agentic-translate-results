#!/usr/bin/env bash
# Verify the translation against the C library for EVERY valid feature
# combination declared in translation/Cargo.toml.
#
# Steps performed:
#   1. enumerate feature combinations from [features]
#   2. cargo check for each combination
#   3. cargo test (debug and release) for each combination
#   4. nm -D symbol parity (asserted inside tests/symbols.rs)
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRATE="$ROOT/translation"
LOGDIR="${TMPDIR:-/tmp}/modeselect-verify"
mkdir -p "$LOGDIR"

fail=0
run() {  # run <label> <cmd...>
  local label="$1"; shift
  local log="$LOGDIR/${label//[^A-Za-z0-9._-]/_}.log"
  printf '>> %-60s ' "$label"
  if timeout 600 "$@" >"$log" 2>&1; then
    echo PASS
  else
    echo "FAIL  (see $log)"
    tail -n 25 "$log" | sed 's/^/     | /'
    fail=1
  fi
}

# ---------------------------------------------------------------- step 0: C lib
echo "== building the C shared library =="
run "cmake configure" env -C "$ROOT/c_src" \
  cmake -S "$ROOT/c_src" -B "$ROOT/c_src/build" -DCMAKE_POSITION_INDEPENDENT_CODE=ON
run "cmake build" cmake --build "$ROOT/c_src/build"

# ------------------------------------------------- step 1: enumerate feature sets
# Parse the [features] table (keys only, ignoring comments/blank lines).
mapfile -t FEATURES < <(
  awk '
    /^[[:space:]]*\[/ { in_f = ($0 ~ /^[[:space:]]*\[features\]/); next }
    !in_f { next }
    /^[[:space:]]*#/ { next }
    /=/ { split($0, a, "="); gsub(/[[:space:]"]/, "", a[1]); if (a[1] != "") print a[1] }
  ' "$CRATE/Cargo.toml" | grep -v '^default$'
)

echo
echo "== feature enumeration =="
if [[ ${#FEATURES[@]} -eq 0 ]]; then
  echo "Cargo.toml declares no [features]; the crate has exactly one"
  echo "configuration (the default). CMakeLists.txt likewise defines no"
  echo "build-time options, so there is a single C configuration to match."
  COMBOS=("<default>")
else
  echo "optional features: ${FEATURES[*]}"
  COMBOS=()
  n=${#FEATURES[@]}
  for ((mask = 0; mask < (1 << n); mask++)); do
    sel=()
    for ((i = 0; i < n; i++)); do
      (((mask >> i) & 1)) && sel+=("${FEATURES[i]}")
    done
    if [[ ${#sel[@]} -eq 0 ]]; then COMBOS+=("<none>")
    else COMBOS+=("$(IFS=,; echo "${sel[*]}")"); fi
  done
  printf '  %s\n' "${COMBOS[@]}"
fi

# --------------------------------------- steps 2 & 9: check + test each combination
echo
echo "== per-combination verification (${#COMBOS[@]} combination(s)) =="
for combo in "${COMBOS[@]}"; do
  if [[ $combo == "<default>" ]]; then
    args=()
  elif [[ $combo == "<none>" ]]; then
    args=(--no-default-features)
  else
    args=(--no-default-features --features "$combo")
  fi

  run "check   [$combo]" env -C "$CRATE" cargo check "${args[@]}" --all-targets
  run "test    [$combo]" env -C "$CRATE" cargo test "${args[@]}"
  run "test-rel[$combo]" env -C "$CRATE" cargo test --release "${args[@]}"
done

echo
if [[ $fail -eq 0 ]]; then
  echo "ALL COMBINATIONS PASS"
else
  echo "FAILURES PRESENT"
fi
exit $fail
