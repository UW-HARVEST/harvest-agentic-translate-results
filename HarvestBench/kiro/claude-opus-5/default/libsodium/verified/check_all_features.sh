#!/usr/bin/env bash
# Enumerate every valid feature combination declared in Cargo.toml and run
# `cargo check` (and optionally `cargo test`) for each one.
#
#   ./check_all_features.sh          # cargo check for every combination
#   ./check_all_features.sh test     # cargo check + cargo test for every combination
set -uo pipefail
cd "$(dirname "$0")"

MODE="${1:-check}"

# Extract the feature names from the [features] section, ignoring "default".
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

echo "features declared in Cargo.toml: ${#FEATURES[@]} ${FEATURES[*]:-(none)}"

# Build the list of combinations: the empty set plus every subset.
COMBOS=("")
n=${#FEATURES[@]}
if (( n > 0 )); then
  for (( mask = 1; mask < (1 << n); mask++ )); do
    combo=""
    for (( i = 0; i < n; i++ )); do
      if (( mask & (1 << i) )); then
        combo="${combo:+$combo,}${FEATURES[$i]}"
      fi
    done
    COMBOS+=("$combo")
  done
fi

# Also cover the crate's own defaults (identical to "" when no default feature
# set is declared, but kept explicit so the matrix is honest).
echo "combinations to verify: ${#COMBOS[@]} (plus the default feature set)"

fail=0
run() {
  local desc="$1"; shift
  echo "=== $desc ==="
  if ! timeout 600 "$@" > /tmp/feat_build.log 2>&1; then
    echo "FAILED: $desc"
    tail -n 40 /tmp/feat_build.log
    fail=1
  else
    echo "ok"
  fi
}

for combo in "${COMBOS[@]}"; do
  if [[ -z "$combo" ]]; then
    run "cargo check --no-default-features" \
      cargo check --release --no-default-features
    [[ "$MODE" == test ]] && run "cargo test --no-default-features" \
      cargo test --release --no-default-features
  else
    run "cargo check --no-default-features --features $combo" \
      cargo check --release --no-default-features --features "$combo"
    [[ "$MODE" == test ]] && run "cargo test --no-default-features --features $combo" \
      cargo test --release --no-default-features --features "$combo"
  fi
done

run "cargo check (default features)" cargo check --release
run "cargo check --all-features" cargo check --release --all-features
if [[ "$MODE" == test ]]; then
  run "cargo test (default features)" cargo test --release
  run "cargo test --all-features" cargo test --release --all-features
fi

if (( fail )); then
  echo "one or more configurations failed"
  exit 1
fi
echo "all configurations OK"
