#!/usr/bin/env bash
# Phase D: run the full differential suite under EVERY Cargo feature
# combination declared in Cargo.toml.
#
# Usage: scripts/check_features.sh
set -uo pipefail
cd "$(dirname "$0")/.."

# --- build the C ground truth -------------------------------------------------
( cd ../c_src && mkdir -p build && cd build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }

# --- enumerate declared features ---------------------------------------------
# Everything under [features], minus "default".
mapfile -t FEATURES < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/ {split($0,a,"="); gsub(/[ \t"]/,"",a[1]); if (a[1] != "default" && a[1] != "") print a[1]}' Cargo.toml
)

run_combo() {
  local label="$1"; shift
  echo "=============================================================="
  echo "FEATURE COMBO: ${label}"
  echo "=============================================================="
  cargo build --release --offline "$@" || return 1
  cargo test  --release --offline "$@" || return 1
}

FAILED=0

# default feature set
run_combo "<default>" || FAILED=1

if [ "${#FEATURES[@]}" -eq 0 ]; then
  echo
  echo "Cargo.toml declares no [features]; the default (empty) set is the only"
  echo "configuration -- feature matrix is complete."
else
  # no-default-features, then every non-empty subset of the declared features
  run_combo "--no-default-features" --no-default-features || FAILED=1
  n=${#FEATURES[@]}
  total=$((1 << n))
  for ((mask = 1; mask < total; mask++)); do
    combo=()
    for ((i = 0; i < n; i++)); do
      (( (mask >> i) & 1 )) && combo+=("${FEATURES[$i]}")
    done
    joined=$(IFS=,; echo "${combo[*]}")
    run_combo "--no-default-features --features ${joined}" \
      --no-default-features --features "${joined}" || FAILED=1
  done
fi

echo
if [ "$FAILED" -eq 0 ]; then echo "ALL FEATURE COMBINATIONS PASSED"; else echo "SOME COMBINATIONS FAILED"; fi
exit "$FAILED"
