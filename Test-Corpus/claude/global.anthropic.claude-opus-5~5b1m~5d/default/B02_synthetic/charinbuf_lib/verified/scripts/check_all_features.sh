#!/usr/bin/env bash
# Phase D — run the whole differential suite under every cargo feature
# combination and under both profiles.
#
# The feature list is extracted from Cargo.toml rather than hard-coded, so the
# sweep stays correct if features are ever added.
set -uo pipefail

cd "$(dirname "$0")/.." || exit 1

# --- enumerate the declared features -----------------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ {inside=1; next}
    /^\[/           {inside=0}
    inside && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)

echo "declared features: ${FEATURES[*]:-<none>}"

# Build the list of feature sets to test: the empty set (with and without
# default features), every single feature, and the power set when the number of
# features is small enough to enumerate exhaustively.
COMBOS=()
COMBOS+=("")                         # default features
COMBOS+=("--no-default-features")    # nothing enabled
n=${#FEATURES[@]}
if (( n > 0 && n <= 12 )); then
  for (( mask=1; mask < (1<<n); mask++ )); do
    sel=()
    for (( i=0; i<n; i++ )); do
      (( mask & (1<<i) )) && sel+=("${FEATURES[i]}")
    done
    joined=$(IFS=,; echo "${sel[*]}")
    COMBOS+=("--no-default-features --features $joined")
    COMBOS+=("--features $joined")
  done
elif (( n > 12 )); then
  echo "too many features for an exhaustive sweep; testing each individually"
  for f in "${FEATURES[@]}"; do
    COMBOS+=("--no-default-features --features $f")
  done
  COMBOS+=("--all-features")
fi
COMBOS+=("--all-features")

status=0
for profile in dev release; do
  prof_flag=""
  [[ $profile == release ]] && prof_flag="--release"
  for combo in "${COMBOS[@]}"; do
    args=$combo
    printf '\n=== profile=%s features=[%s] ===\n' "$profile" "${args:-default}"

    # The cdylib must be rebuilt for this configuration before the tests load it
    # (`cargo test` never builds the cdylib artifact itself).
    # shellcheck disable=SC2086
    if ! timeout 600 cargo build --lib $prof_flag $args >/dev/null 2>&1; then
      echo "BUILD FAILED"; status=1; continue
    fi
    # shellcheck disable=SC2086
    if timeout 600 cargo test $prof_flag $args 2>&1 | tee /dev/stderr \
        | grep -qE '^test result: FAILED|error\[|^error:'; then
      echo "TESTS FAILED"; status=1
    else
      echo "OK"
    fi
  done
done

echo
if (( status == 0 )); then
  echo "ALL FEATURE COMBINATIONS PASSED"
else
  echo "FAILURES PRESENT"
fi
exit $status
