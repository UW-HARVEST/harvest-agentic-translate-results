#!/usr/bin/env bash
# Enumerate every valid feature combination declared in translation/Cargo.toml
# and run `cargo check` + `cargo test` for each one.
#
# Usage: ./check_all_features.sh [check|test]   (default: test)
set -uo pipefail

MODE="${1:-test}"
cd "$(dirname "$0")" || exit 1

# --- enumerate features ----------------------------------------------------
# Pull the names out of the [features] table (ignoring "default").
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inblock = 1; next }
    /^\[/           { inblock = 0 }
    inblock && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)

N=${#FEATURES[@]}
echo "Declared features (${N}): ${FEATURES[*]:-<none>}"

# All 2^N subsets. With N == 0 this yields exactly one combination: the empty
# one, i.e. --no-default-features with nothing enabled.
COMBOS=()
for ((mask = 0; mask < (1 << N); mask++)); do
  combo=""
  for ((i = 0; i < N; i++)); do
    if ((mask & (1 << i))); then
      combo="${combo:+$combo,}${FEATURES[i]}"
    fi
  done
  COMBOS+=("$combo")
done

# The default feature set is also a valid build-time configuration.
COMBOS+=("__default__")

FAILED=0
for combo in "${COMBOS[@]}"; do
  if [[ "$combo" == "__default__" ]]; then
    label="(default features)"
    args=()
  else
    label="--no-default-features --features '${combo}'"
    args=(--no-default-features --features "$combo")
  fi

  echo
  echo "=============================================================="
  echo ">>> cargo ${MODE} ${label}"
  echo "=============================================================="
  if ! timeout 600 cargo "$MODE" "${args[@]}" 2>&1 | tail -n 25; then
    echo "!!! FAILED: ${label}"
    FAILED=1
  fi
done

echo
if ((FAILED)); then
  echo "RESULT: at least one configuration failed"
  exit 1
fi
echo "RESULT: all ${#COMBOS[@]} configuration(s) passed cargo ${MODE}"
