#!/usr/bin/env bash
# Enumerate every valid feature combination declared in Cargo.toml and run
# `cargo check` + `cargo test` for each one.
#
# Usage: ./verify_all_features.sh [check|test|all]   (default: all)
set -uo pipefail

cd "$(dirname "$0")"
MODE="${1:-all}"
LOG=/tmp/verify_features.log
: > "$LOG"

# --- Enumerate features ----------------------------------------------------
# Read the [features] table from Cargo.toml, ignoring the implicit "default".
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { in_f = 1; next }
    /^\[/           { in_f = 0 }
    in_f && /=/ {
      split($0, a, "=")
      gsub(/[ \t]/, "", a[1])
      if (a[1] != "" && a[1] != "default" && a[1] !~ /^#/) print a[1]
    }
  ' Cargo.toml
)

HAS_DEFAULT=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /^[ \t]*default[ \t]*=/{print "yes"}' Cargo.toml)

n=${#FEATURES[@]}
echo "Declared features (${n}): ${FEATURES[*]:-<none>}"
echo "Has explicit default: ${HAS_DEFAULT:-no}"

# Build the list of combinations to test: the powerset of the declared
# features, always with --no-default-features so the set is exact. Plus the
# plain default configuration.
COMBOS=("<default>")
if (( n > 0 )); then
  total=$(( 1 << n ))
  for (( mask = 0; mask < total; ++mask )); do
    combo=""
    for (( i = 0; i < n; ++i )); do
      if (( mask & (1 << i) )); then
        combo="${combo:+$combo,}${FEATURES[i]}"
      fi
    done
    COMBOS+=("$combo")
  done
fi

echo "Combinations to verify: ${#COMBOS[@]}"

fail=0
run() {
  local desc="$1"; shift
  echo "=== $desc :: $* ===" | tee -a "$LOG"
  if timeout 600 "$@" >>"$LOG" 2>&1; then
    echo "    PASS"
  else
    echo "    FAIL  (see $LOG)"
    fail=1
  fi
}

for combo in "${COMBOS[@]}"; do
  if [[ "$combo" == "<default>" ]]; then
    ARGS=()
    label="default"
  elif [[ -z "$combo" ]]; then
    ARGS=(--no-default-features)
    label="no-default-features (empty)"
  else
    ARGS=(--no-default-features --features "$combo")
    label="no-default-features + $combo"
  fi

  if [[ "$MODE" == "check" || "$MODE" == "all" ]]; then
    run "check [$label]" cargo check --all-targets "${ARGS[@]}"
  fi
  if [[ "$MODE" == "test" || "$MODE" == "all" ]]; then
    run "test  [$label]" cargo test --release "${ARGS[@]}"
    run "test  [$label] (debug)" cargo test "${ARGS[@]}"
  fi
done

if (( fail )); then
  echo "RESULT: FAILURES -- see $LOG"
  exit 1
fi
echo "RESULT: all ${#COMBOS[@]} feature combination(s) OK"
