#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

mapfile -t features < <(
    awk '
        /^\[features\]$/ { in_features = 1; next }
        /^\[/ { in_features = 0 }
        in_features && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
            name = $0
            sub(/[[:space:]]*=.*/, "", name)
            if (name != "default") {
                print name
            }
        }
    ' Cargo.toml
)

timeout 600 cargo check
timeout 600 cargo build --release

combinations=$((1 << ${#features[@]}))
for ((mask = 0; mask < combinations; mask++)); do
    selected=()
    for ((index = 0; index < ${#features[@]}; index++)); do
        if ((mask & (1 << index))); then
            selected+=("${features[index]}")
        fi
    done
    if ((${#selected[@]} == 0)); then
        timeout 600 cargo test --no-default-features -- --test-threads=1
    else
        feature_list=$(IFS=,; printf '%s' "${selected[*]}")
        timeout 600 cargo test --no-default-features --features "$feature_list" -- --test-threads=1
    fi
done
