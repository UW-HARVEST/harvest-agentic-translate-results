#!/usr/bin/env bash
# Enumerate every Cargo feature combination and run the full differential suite
# for each. Feature names are read out of Cargo.toml via `cargo metadata`, so no
# combination can be forgotten by hand.
#
# Usage: scripts/check_features.sh [extra cargo args...]
set -uo pipefail

cd "$(dirname "$0")/.." || exit 1
EXTRA=("$@")
CARGO_FLAGS=(--offline)

# --- discover features -------------------------------------------------------
FEATURES=$(cargo metadata "${CARGO_FLAGS[@]}" --no-deps --format-version 1 2>/dev/null \
  | tr ',' '\n' | grep -o '"features":{[^}]*}' \
  | sed 's/.*{//' | grep -o '"[^"]*":' | tr -d '":' | sort -u)

if [ -z "${FEATURES}" ]; then
  echo "Cargo.toml declares no [features]; the only configuration is the default."
  COMBOS=("<default>" "--no-default-features")
else
  echo "features found: ${FEATURES}"
  # Build the power set of the feature list.
  mapfile -t FLIST <<<"${FEATURES}"
  n=${#FLIST[@]}
  COMBOS=("<default>")
  for ((mask = 0; mask < (1 << n); mask++)); do
    sel=()
    for ((i = 0; i < n; i++)); do
      (((mask >> i) & 1)) && sel+=("${FLIST[$i]}")
    done
    if [ ${#sel[@]} -eq 0 ]; then
      COMBOS+=("--no-default-features")
    else
      COMBOS+=("--no-default-features --features $(
        IFS=,
        echo "${sel[*]}"
      )")
    fi
  done
fi

# --- C reference library -----------------------------------------------------
if [ ! -f ../c_src/build/libdriver.so ]; then
  echo "building the C reference library..."
  (mkdir -p ../c_src/build && cd ../c_src/build &&
    cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null &&
    cmake --build . >/dev/null) || exit 1
fi

# --- run every combination ---------------------------------------------------
rc=0
for combo in "${COMBOS[@]}"; do
  for profile in "" "--release"; do
    label="${combo} ${profile:-<debug>}"
    args=()
    [ "${combo}" != "<default>" ] && read -r -a args <<<"${combo}"
    [ -n "${profile}" ] && args+=("${profile}")

    echo ""
    echo "=============================================================="
    echo "== cargo test ${args[*]} ${EXTRA[*]}"
    echo "=============================================================="
    # The cdylib must exist for the profile the tests are about to load from.
    cargo build "${CARGO_FLAGS[@]}" "${args[@]}" >/dev/null 2>&1
    if ! cargo test "${CARGO_FLAGS[@]}" "${args[@]}" "${EXTRA[@]}"; then
      echo "FAILED: ${label}"
      rc=1
    fi
  done
done

echo ""
if [ $rc -eq 0 ]; then
  echo "ALL FEATURE COMBINATIONS PASSED (${#COMBOS[@]} combos x 2 profiles)"
else
  echo "SOME FEATURE COMBINATIONS FAILED"
fi
exit $rc
