#!/usr/bin/env bash
# Verify the Rust translation against the C reference for every build
# configuration: every valid Cargo feature combination x every profile.
#
# Usage: ./verify_all.sh        (run from translation/)
set -uo pipefail

cd "$(dirname "$0")"

# --- Enumerate feature combinations from Cargo.toml --------------------------
# Collect the feature names declared under [features] (excluding "default").
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

COMBOS=("")   # the empty combination == --no-default-features
n=${#FEATURES[@]}
if (( n > 0 )); then
  for (( mask = 1; mask < (1 << n); mask++ )); do
    combo=""
    for (( i = 0; i < n; i++ )); do
      if (( mask & (1 << i) )); then
        combo+="${FEATURES[i]},"
      fi
    done
    COMBOS+=("${combo%,}")
  done
fi

echo "Declared features: ${n} -> ${#COMBOS[@]} combination(s)"

# --- Step 1: cargo check every combination ----------------------------------
fail=0
for combo in "${COMBOS[@]}"; do
  label="${combo:-<none>}"
  echo "=== cargo check --no-default-features --features '${label}' ==="
  if [[ -z "$combo" ]]; then
    timeout 600 cargo check --no-default-features --all-targets || fail=1
  else
    timeout 600 cargo check --no-default-features --features "$combo" --all-targets || fail=1
  fi
done

# --- Step 2: differential tests, every combination x every profile ----------
for profile in dev release; do
  for combo in "${COMBOS[@]}"; do
    label="${combo:-<none>}"
    echo "=== cargo test  profile=${profile}  features='${label}' ==="
    if [[ -z "$combo" ]]; then
      DRIVER_TEST_PROFILE="$profile" DRIVER_TEST_FEATURES="" \
        timeout 600 cargo test --no-default-features -- --test-threads=1 || fail=1
    else
      DRIVER_TEST_PROFILE="$profile" DRIVER_TEST_FEATURES="$combo" \
        timeout 600 cargo test --no-default-features --features "$combo" -- --test-threads=1 || fail=1
    fi
  done
done

if (( fail )); then
  echo "RESULT: FAILURES"
  exit 1
fi
echo "RESULT: all configurations verified"
