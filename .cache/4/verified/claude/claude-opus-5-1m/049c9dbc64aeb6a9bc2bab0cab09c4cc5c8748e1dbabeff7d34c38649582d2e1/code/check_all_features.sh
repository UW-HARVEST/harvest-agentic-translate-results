#!/usr/bin/env bash
# Enumerate every valid Cargo feature combination and run `cargo check` /
# `cargo test` for each one.
#
# The crate declares NO `[features]` section, so the only valid configuration is
# the empty feature set (`--no-default-features`). The script derives that
# mechanically instead of hard-coding it, so it keeps working if features are
# ever added.
set -uo pipefail

cd "$(dirname "$0")" || exit 1

CMD=${1:-check}

# --- extract feature names from the [features] table in Cargo.toml -----------
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

n=${#FEATURES[@]}
echo "features found: $n ${FEATURES[*]:-(none)}"

status=0
total=$(( 1 << n ))
for (( mask = 0; mask < total; mask++ )); do
  combo=""
  for (( i = 0; i < n; i++ )); do
    if (( mask & (1 << i) )); then
      combo="${combo:+$combo,}${FEATURES[$i]}"
    fi
  done
  label="${combo:-<empty>}"
  echo "=============================================================="
  echo "### cargo $CMD --no-default-features --features '$combo'  [$label]"
  echo "=============================================================="
  if [[ "$CMD" == "test" ]]; then
    timeout 600 cargo test --no-default-features --features "$combo" -- --test-threads=1
  else
    timeout 600 cargo "$CMD" --no-default-features --features "$combo"
  fi
  rc=$?
  if (( rc != 0 )); then
    echo "!!! FAILED (rc=$rc) for combo [$label]"
    status=1
  else
    echo ">>> OK for combo [$label]"
  fi
done

exit $status
