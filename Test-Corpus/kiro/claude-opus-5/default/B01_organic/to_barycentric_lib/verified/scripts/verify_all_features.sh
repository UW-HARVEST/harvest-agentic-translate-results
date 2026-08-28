#!/usr/bin/env bash
# Enumerate every valid feature combination from translation/Cargo.toml and run
# `cargo check` + `cargo test` for each one.
#
# Usage: ./verify_all_features.sh [check|test]   (default: test)
set -uo pipefail

MODE="${1:-test}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRATE_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$CRATE_DIR" || exit 1

# --- enumerate features -----------------------------------------------------
# Read the [features] table, ignoring the implicit "default" entry.
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { in_f = 1; next }
    /^\[/           { in_f = 0 }
    in_f && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, kv, "=")
      gsub(/[[:space:]]/, "", kv[1])
      if (kv[1] != "default") print kv[1]
    }
  ' Cargo.toml
)

HAS_DEFAULT=$(awk '
  /^\[features\]/ { in_f = 1; next }
  /^\[/           { in_f = 0 }
  in_f && /^default[[:space:]]*=/ { print "yes" }
' Cargo.toml)

N=${#FEATURES[@]}
echo "features declared: $N ${FEATURES[*]:-(none)}"

# --- build the list of combinations ----------------------------------------
COMBOS=()
if [[ -n "$HAS_DEFAULT" ]]; then
  COMBOS+=("__default__")   # marker: build with default features
fi
COMBOS+=("")                # --no-default-features, nothing enabled

if (( N > 0 )); then
  for (( mask = 1; mask < (1 << N); mask++ )); do
    combo=""
    for (( i = 0; i < N; i++ )); do
      if (( mask & (1 << i) )); then
        combo+="${combo:+,}${FEATURES[i]}"
      fi
    done
    COMBOS+=("$combo")
  done
fi

echo "combinations to verify: ${#COMBOS[@]}"

# --- run ---------------------------------------------------------------------
FAILED=()
for combo in "${COMBOS[@]}"; do
  if [[ "$combo" == "__default__" ]]; then
    label="(default features)"
    args=()
  elif [[ -z "$combo" ]]; then
    label="(no default features, none enabled)"
    args=(--no-default-features)
  else
    label="$combo"
    args=(--no-default-features --features "$combo")
  fi

  log="/tmp/verify_${MODE}_$(echo "${combo:-none}" | tr ',' '_').log"

  echo "=== $MODE $label ==="
  if [[ "$MODE" == "check" ]]; then
    timeout 600 cargo check --all-targets "${args[@]}" >"$log" 2>&1
  else
    timeout 600 cargo test --release "${args[@]}" >"$log" 2>&1
  fi
  status=$?

  if (( status == 0 )); then
    echo "    OK  (log: $log)"
  else
    echo "    FAILED (exit $status, log: $log)"
    tail -n 25 "$log" | sed 's/^/    | /'
    FAILED+=("$label")
  fi
done

echo
if (( ${#FAILED[@]} == 0 )); then
  echo "ALL ${#COMBOS[@]} COMBINATION(S) PASSED ($MODE)"
  exit 0
fi
echo "FAILURES in: ${FAILED[*]}"
exit 1
