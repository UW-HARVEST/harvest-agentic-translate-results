#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

# Cargo.toml has no [features] table, so the powerset contains only the empty set.
feature_combinations=("")

for features in "${feature_combinations[@]}"; do
    args=(--no-default-features)
    if [[ -n "$features" ]]; then
        args+=(--features "$features")
    fi

    timeout 600 cargo check "${args[@]}"
    # cargo test compiles an rlib dependency but does not refresh the cdylib.
    timeout 600 cargo build "${args[@]}"
    timeout 600 cargo test "${args[@]}" -- --test-threads=1
done
