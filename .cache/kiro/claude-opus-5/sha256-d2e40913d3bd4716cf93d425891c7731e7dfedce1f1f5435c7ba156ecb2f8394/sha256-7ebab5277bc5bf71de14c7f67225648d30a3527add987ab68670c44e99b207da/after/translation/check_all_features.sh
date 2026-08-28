#!/usr/bin/env bash
# Enumerate every valid feature combination declared in Cargo.toml and run
# `cargo check` (and optionally `cargo test`) against each one.
#
#   ./check_all_features.sh          # cargo check for every combination
#   ./check_all_features.sh test     # cargo test for every combination
set -uo pipefail
cd "$(dirname "$0")"

ACTION="${1:-check}"

# Feature names from the [features] table, excluding "default".
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { in_f=1; next }
    /^\[/           { in_f=0 }
    in_f && /^[[:space:]]*[A-Za-z0-9_-]+[[:space:]]*=/ {
      sub(/[[:space:]]*=.*/, "");
      gsub(/[[:space:]]/, "");
      if ($0 != "default" && $0 != "") print
    }
  ' Cargo.toml
)

N=${#FEATURES[@]}
echo "Declared features (${N}): ${FEATURES[*]:-<none>}"

run_combo() {
  local label="$1"; shift
  echo "=== cargo ${ACTION} --no-default-features $* (${label}) ==="
  if ! timeout 600 cargo "${ACTION}" --no-default-features "$@" 2>&1 | tail -n 15; then
    echo "FAILED: ${label}"
    return 1
  fi
}

FAIL=0
if (( N == 0 )); then
  # No features declared: the empty set is the only valid configuration.
  run_combo "no features" || FAIL=1
  echo "=== cargo ${ACTION} (default) ==="
  timeout 600 cargo "${ACTION}" 2>&1 | tail -n 15 || FAIL=1
else
  for (( mask=0; mask < (1<<N); mask++ )); do
    combo=()
    for (( i=0; i<N; i++ )); do
      (( mask & (1<<i) )) && combo+=("${FEATURES[$i]}")
    done
    if (( ${#combo[@]} == 0 )); then
      run_combo "empty" || FAIL=1
    else
      joined=$(IFS=,; echo "${combo[*]}")
      TEST_FEATURES="$joined" run_combo "$joined" --features "$joined" || FAIL=1
    fi
  done
fi

if (( FAIL )); then
  echo "RESULT: one or more combinations failed"
  exit 1
fi
echo "RESULT: all combinations passed"
