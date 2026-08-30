#!/usr/bin/env bash
# Phase D — build + run the differential suite under EVERY cargo feature
# combination (the powerset of the features declared in Cargo.toml).
#
# Usage: scripts/check_all_features.sh
set -uo pipefail

CRATE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REPO_ROOT="$(cd "$CRATE_DIR/.." && pwd)"
cd "$CRATE_DIR"

# Cargo cannot reach crates.io in this environment; the deps are vendored in the
# local registry cache, so run offline. Harmless when the network is available.
OFFLINE="--offline"

fail=0

echo "== building the C reference shared library =="
mkdir -p "$REPO_ROOT/c_src/build"
(
  cd "$REPO_ROOT/c_src/build" \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null
) || { echo "FATAL: C build failed"; exit 1; }
C_SO="$REPO_ROOT/c_src/build/libSimpleList.so"
[ -f "$C_SO" ] || { echo "FATAL: $C_SO not produced"; exit 1; }

# --- enumerate features from Cargo.toml ------------------------------------
# Everything in the [features] table except the "default" key itself.
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf=1; next }
    /^\[/           { inf=0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)

n=${#FEATURES[@]}
echo "== declared features: ${n} ${FEATURES[*]:-(none)} =="

run_case() {
  local label="$1"; shift
  echo
  echo "---------------------------------------------------------------"
  echo "== CONFIG: $label =="
  echo "---------------------------------------------------------------"
  # The .so under test must be rebuilt for this feature set before the tests
  # dlopen it, otherwise a stale artifact gets verified.
  if ! cargo build --release $OFFLINE "$@" 2>&1 | tail -3; then
    echo "RESULT[$label]: BUILD FAILED"; fail=1; return
  fi
  if cargo test --release $OFFLINE --no-fail-fast "$@" 2>&1 | tail -25; then
    echo "RESULT[$label]: PASS"
  else
    echo "RESULT[$label]: FAIL"; fail=1
  fi
}

# Always cover these three canonical configurations.
run_case "default features"
run_case "--no-default-features"   --no-default-features
run_case "--all-features"          --all-features

# Powerset of the explicitly declared features (skipped when there are none).
if (( n > 0 )); then
  for (( mask=0; mask < (1<<n); mask++ )); do
    combo=()
    for (( i=0; i<n; i++ )); do
      (( mask & (1<<i) )) && combo+=("${FEATURES[$i]}")
    done
    joined=$(IFS=,; echo "${combo[*]:-}")
    run_case "--no-default-features --features ${joined:-<empty>}" \
      --no-default-features --features "$joined"
  done
else
  echo
  echo "== no [features] declared: the three canonical configurations above"
  echo "   constitute the complete feature-combination space =="
fi

echo
echo "==============================================================="
if (( fail )); then
  echo "OVERALL: FAIL"
  exit 1
fi
echo "OVERALL: PASS (all feature combinations)"
