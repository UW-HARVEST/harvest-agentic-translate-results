#!/usr/bin/env bash
# Enumerates every valid Cargo feature combination and runs `cargo check` plus
# the C-vs-Rust parity tests for each one.
#
# Usage: ./verify_all_features.sh          (from translation/)
set -uo pipefail

cd "$(dirname "$0")"

# --- discover features -------------------------------------------------------
# Reads the [features] table from Cargo.toml. "default" is excluded from the
# powerset since it is expressed via --no-default-features.
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

n=${#FEATURES[@]}
echo "Discovered ${n} optional feature(s): ${FEATURES[*]:-<none>}"

# --- build the C reference library ------------------------------------------
echo "== building C reference library =="
( cd ../c_src && mkdir -p build && cd build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }

# --- enumerate the powerset of features -------------------------------------
combos=()
for ((mask = 0; mask < (1 << n); mask++)); do
  combo=()
  for ((i = 0; i < n; i++)); do
    (((mask >> i) & 1)) && combo+=("${FEATURES[i]}")
  done
  combos+=("$(IFS=,; echo "${combo[*]}")")
done
# Also cover the crate's declared defaults.
combos+=("__DEFAULT__")

status=0
for combo in "${combos[@]}"; do
  if [[ $combo == "__DEFAULT__" ]]; then
    label="default features"
    args=()
  elif [[ -z $combo ]]; then
    label="--no-default-features"
    args=(--no-default-features)
  else
    label="--no-default-features --features $combo"
    args=(--no-default-features --features "$combo")
  fi

  echo
  echo "===== $label ====="
  if ! timeout 600 cargo check "${args[@]}" 2>&1 | tail -n 5; then
    echo "CHECK FAILED: $label"; status=1; continue
  fi
  if ! timeout 600 cargo test "${args[@]}" 2>&1 | tail -n 20; then
    echo "TEST FAILED: $label"; status=1; continue
  fi
done

echo
if ((status == 0)); then
  echo "ALL FEATURE COMBINATIONS PASSED (${#combos[@]} total)"
else
  echo "FAILURES PRESENT"
fi
exit $status
