#!/usr/bin/env bash
# Enumerate every Cargo feature combination and run the whole differential
# suite under each one.  Derived from Cargo.toml, not hard-coded.
set -uo pipefail
cd "$(dirname "$0")/.."

mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf=1; next }
    /^\[/           { inf=0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ { sub(/[[:space:]]*=.*/,""); print }
  ' Cargo.toml
)

echo "features declared in Cargo.toml: ${#FEATURES[@]} ${FEATURES[*]-（none)}"

COMBOS=()
if [ "${#FEATURES[@]}" -eq 0 ]; then
  # No [features] table => the only build is the default one.  Prove that the
  # three feature-related invocations really are the same build.
  COMBOS+=("--no-default-features" "--all-features" "")
else
  n=${#FEATURES[@]}
  for ((mask=0; mask<(1<<n); mask++)); do
    sel=()
    for ((i=0; i<n; i++)); do
      (( mask & (1<<i) )) && sel+=("${FEATURES[$i]}")
    done
    if [ "${#sel[@]}" -eq 0 ]; then
      COMBOS+=("--no-default-features")
    else
      COMBOS+=("--no-default-features --features $(IFS=,; echo "${sel[*]}")")
    fi
  done
  COMBOS+=("--all-features" "")
fi

fail=0
for combo in "${COMBOS[@]}"; do
  label="${combo:-<default>}"
  echo "=============================================================="
  echo "### cargo test --release $combo"
  echo "=============================================================="
  # shellcheck disable=SC2086
  if ! cargo build --release $combo >/dev/null 2>&1; then
    echo "BUILD FAILED for $label"; fail=1; continue
  fi
  for profile in release debug; do
    flag="--release"; [ "$profile" = debug ] && flag=""
    # shellcheck disable=SC2086
    out=$(cargo test $flag $combo 2>&1)
    echo "$out" | grep -E '^(test result|     Running)'
    if echo "$out" | grep -qE 'test result: FAILED|error\[|error:'; then
      echo "TESTS FAILED for $label [$profile]"; fail=1
    else
      echo "OK: $label [$profile]"
    fi
  done
done

echo
if [ "$fail" -eq 0 ]; then echo "ALL FEATURE COMBINATIONS PASSED"; else echo "SOME COMBINATIONS FAILED"; exit 1; fi
