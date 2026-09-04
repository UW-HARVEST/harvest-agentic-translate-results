#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"

# Cargo.toml declares no named features, so the complete feature matrix is the
# default configuration and the equivalent no-default-features configuration.
for profile in debug release; do
    profile_args=()
    if [[ "$profile" == release ]]; then
        profile_args=(--release)
    fi

    for feature_set in default no-default-features; do
        feature_args=()
        if [[ "$feature_set" == no-default-features ]]; then
            feature_args=(--no-default-features)
        fi

        timeout 600 cargo build "${profile_args[@]}" "${feature_args[@]}"
        timeout 600 cargo test "${profile_args[@]}" "${feature_args[@]}" -- --test-threads=1
    done
done
