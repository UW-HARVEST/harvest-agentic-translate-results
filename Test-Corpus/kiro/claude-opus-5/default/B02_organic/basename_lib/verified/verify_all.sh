#!/usr/bin/env bash
# Verify the Rust translation against the C reference for every valid
# build-time configuration.
#
# Cargo.toml declares no [features] and c_src/CMakeLists.txt declares no
# options, so there is exactly one configuration: the default (empty) feature
# set. It is still checked and tested through the same loop so that adding a
# feature later only requires extending FEATURE_COMBOS.
set -euo pipefail

cd "$(dirname "$0")"

# Sanity check: fail loudly if features appear without this script being updated.
if grep -q '^\[features\]' Cargo.toml; then
    echo "Cargo.toml now has a [features] section; update FEATURE_COMBOS." >&2
    exit 1
fi

FEATURE_COMBOS=("")

for combo in "${FEATURE_COMBOS[@]}"; do
    label="${combo:-<none>}"
    echo "=== features: ${label} ==="
    if [[ -n "${combo}" ]]; then
        args=(--no-default-features --features "${combo}")
    else
        args=(--no-default-features)
    fi

    timeout 600 cargo check "${args[@]}"
    timeout 600 cargo build "${args[@]}"
    timeout 600 cargo test "${args[@]}"
    timeout 600 cargo build --release "${args[@]}"
    timeout 600 cargo test --release "${args[@]}"
done

echo "All configurations verified."
