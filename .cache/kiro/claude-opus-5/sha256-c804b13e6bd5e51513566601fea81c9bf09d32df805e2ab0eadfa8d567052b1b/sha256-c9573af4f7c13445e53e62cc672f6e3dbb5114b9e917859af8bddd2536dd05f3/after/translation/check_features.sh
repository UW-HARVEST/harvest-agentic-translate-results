#!/usr/bin/env bash
# Enumerate every feature combination declared in Cargo.toml and run
# `cargo check` + the full differential suite for each. Phase D automation.
set -uo pipefail
cd "$(dirname "$0")"

# Mechanically extract feature names from the [features] table (if any).
FEATURES=$(awk '
  /^\[features\]/ {inside=1; next}
  /^\[/           {inside=0}
  inside && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]); print a[1]
  }' Cargo.toml)

echo "== declared features =="
if [ -z "${FEATURES}" ]; then
  echo "(none -- Cargo.toml has no [features] table)"
else
  echo "${FEATURES}"
fi

# Build the list of combinations to test. With no declared features the only
# distinct configurations are the default build and --no-default-features,
# which are identical here but both are exercised anyway.
COMBOS=()
COMBOS+=("")                          # default
COMBOS+=("--no-default-features")

if [ -n "${FEATURES}" ]; then
  mapfile -t FARR <<<"${FEATURES}"
  n=${#FARR[@]}
  # full power set of the declared features, with default features off
  for ((mask = 0; mask < (1 << n); mask++)); do
    sel=""
    for ((i = 0; i < n; i++)); do
      if (((mask >> i) & 1)); then sel="${sel}${FARR[$i]},"; fi
    done
    sel="${sel%,}"
    if [ -n "${sel}" ]; then
      COMBOS+=("--no-default-features --features ${sel}")
    fi
  done
  COMBOS+=("--all-features")
fi

fail=0
for combo in "${COMBOS[@]}"; do
  label="${combo:-<default>}"
  echo
  echo "=============================================================="
  echo "== combination: ${label}"
  echo "=============================================================="

  # shellcheck disable=SC2086
  if ! timeout 600 cargo check ${combo} 2>&1 | tail -3; then
    echo "CHECK FAILED: ${label}"; fail=1; continue
  fi
  # Rebuild BOTH cdylib profiles under this combination so the tests load the
  # .so actually produced by this feature set.
  # shellcheck disable=SC2086
  timeout 600 cargo build --release ${combo} >/dev/null 2>&1 || { echo "RELEASE BUILD FAILED: ${label}"; fail=1; continue; }
  # shellcheck disable=SC2086
  timeout 600 cargo build ${combo} >/dev/null 2>&1 || { echo "DEBUG BUILD FAILED: ${label}"; fail=1; continue; }

  # shellcheck disable=SC2086
  out=$(timeout 600 cargo test --release ${combo} 2>&1)
  echo "${out}" | grep -E "^test result:|FAILED|error\[" || true
  if ! echo "${out}" | grep -qE "^test result: ok\."; then
    echo "TESTS FAILED: ${label}"; fail=1
  fi
done

echo
if [ "${fail}" -eq 0 ]; then
  echo "ALL FEATURE COMBINATIONS PASSED"
else
  echo "SOME FEATURE COMBINATIONS FAILED"
fi
exit "${fail}"
