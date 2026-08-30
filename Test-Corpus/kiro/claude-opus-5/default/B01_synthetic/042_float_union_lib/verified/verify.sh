#!/usr/bin/env bash
# Verify the Rust translation against the C reference for every valid
# build-time configuration.
#
# Feature combinations are derived from the `[features]` table in Cargo.toml
# (the powerset of all declared features, plus the no-feature and default
# builds), so no combination has to be listed by hand. The crate currently
# declares no features, which means there is a single configuration:
# `--no-default-features`.
#
# Usage: ./verify.sh            # cargo check + cargo test for every combo
#        ./verify.sh check      # cargo check only
set -uo pipefail

cd "$(dirname "$0")" || exit 1

MODE="${1:-all}"
TIMEOUT=600

# --- enumerate features -------------------------------------------------------
# Feature names are the keys of the [features] table; `default` is handled
# separately because it is not an independent knob.
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { in_f = 1; next }
    /^\[/           { in_f = 0 }
    in_f && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)

COMBOS=()
n=${#FEATURES[@]}
for ((mask = 0; mask < (1 << n); mask++)); do
  combo=()
  for ((i = 0; i < n; i++)); do
    (((mask >> i) & 1)) && combo+=("${FEATURES[$i]}")
  done
  COMBOS+=("$(
    IFS=,
    echo "${combo[*]}"
  )")
done
# Also exercise the default feature set if one is declared.
if grep -qE '^default[[:space:]]*=' Cargo.toml; then
  COMBOS+=("__default__")
fi

echo "Declared features: ${FEATURES[*]:-<none>}"
echo "Configurations to verify: ${#COMBOS[@]}"
echo

# --- build the C reference once ----------------------------------------------
echo "== building C reference =="
(
  cd ../c_src && mkdir -p build && cd build &&
    cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null &&
    cmake --build . >/dev/null
) || {
  echo "C build FAILED"
  exit 1
}
nm -D --defined-only ../c_src/build/libdriver.so
echo

# --- per-configuration verification ------------------------------------------
fail=0
for combo in "${COMBOS[@]}"; do
  if [[ $combo == "__default__" ]]; then
    args=()
    label="default features"
  elif [[ -z $combo ]]; then
    args=(--no-default-features)
    label="no features"
  else
    args=(--no-default-features --features "$combo")
    label="features: $combo"
  fi

  echo "===================================================================="
  echo "== $label"
  echo "===================================================================="

  for step in "check --all-targets" "build --release" "test"; do
    read -r -a stepargs <<<"$step"
    echo "-- cargo ${stepargs[*]} ${args[*]}"
    if ! timeout "$TIMEOUT" cargo "${stepargs[@]}" "${args[@]}" 2>&1 | tail -n 15; then
      echo "!! FAILED: cargo ${stepargs[*]} ${args[*]}"
      fail=1
    fi
    [[ $MODE == check ]] && break
  done

  echo "-- exported symbols (Rust, $label)"
  nm -D --defined-only target/release/libdriver.so 2>/dev/null
  echo
done

if ((fail)); then
  echo "RESULT: FAILURES"
  exit 1
fi
echo "RESULT: all ${#COMBOS[@]} configuration(s) verified against C"
