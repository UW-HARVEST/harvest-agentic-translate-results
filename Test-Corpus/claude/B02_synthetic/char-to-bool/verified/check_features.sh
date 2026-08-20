#!/usr/bin/env bash
# Phase A / Phase D: mechanically enumerate every valid cargo feature
# combination from Cargo.toml and run `cargo check` + the full differential
# test suite for each one.
#
# Usage: ./check_features.sh [--check-only]
set -uo pipefail

ROOT=$(cd "$(dirname "$0")" && pwd)
cd "$ROOT"

CHECK_ONLY=0
[ "${1:-}" = "--check-only" ] && CHECK_ONLY=1

# ---------------------------------------------------------------------------
# Extract the feature names declared under [features], excluding `default`
# (which is applied via the presence/absence of --no-default-features).
# ---------------------------------------------------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inside = 1; next }
    /^\[/           { inside = 0 }
    inside && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "=");
      gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1];
    }
  ' Cargo.toml
)

N=${#FEATURES[@]}
echo "==> declared non-default features (${N}): ${FEATURES[*]:-<none>}"

# ---------------------------------------------------------------------------
# Build the list of combinations: the full power set of the non-default
# features (always with --no-default-features), plus the plain default build.
# ---------------------------------------------------------------------------
COMBOS=()
TOTAL=$((1 << N))
for ((mask = 0; mask < TOTAL; mask++)); do
  combo=""
  for ((bit = 0; bit < N; bit++)); do
    if (((mask >> bit) & 1)); then
      combo="${combo:+$combo,}${FEATURES[bit]}"
    fi
  done
  COMBOS+=("--no-default-features${combo:+ --features $combo}")
done
# The default feature set (identical to the empty set here, but verified
# explicitly so a future non-empty `default` list is covered too).
COMBOS+=("")

echo "==> ${#COMBOS[@]} feature combination(s) to verify"

FAILED=0
for combo in "${COMBOS[@]}"; do
  label="${combo:-<default features>}"
  echo
  echo "############################################################"
  echo "# cargo check --offline $label"
  echo "############################################################"
  # shellcheck disable=SC2086
  if ! timeout 600 cargo check --offline --all-targets $combo 2>&1 | tail -n 15; then
    echo "!! cargo check FAILED for: $label"
    FAILED=1
    continue
  fi

  if [ "$CHECK_ONLY" -eq 1 ]; then
    continue
  fi

  echo "--- cargo build (cdylib + bin) $label"
  # shellcheck disable=SC2086
  if ! timeout 600 cargo build --offline $combo 2>&1 | tail -n 15; then
    echo "!! cargo build FAILED for: $label"
    FAILED=1
    continue
  fi

  echo "--- cargo test $label"
  # shellcheck disable=SC2086
  if ! timeout 600 cargo test --offline $combo 2>&1 | tail -n 25; then
    echo "!! cargo test FAILED for: $label"
    FAILED=1
  fi
done

echo
if [ "$FAILED" -eq 0 ]; then
  echo "==> ALL feature combinations passed"
else
  echo "==> FAILURES detected"
fi
exit "$FAILED"
