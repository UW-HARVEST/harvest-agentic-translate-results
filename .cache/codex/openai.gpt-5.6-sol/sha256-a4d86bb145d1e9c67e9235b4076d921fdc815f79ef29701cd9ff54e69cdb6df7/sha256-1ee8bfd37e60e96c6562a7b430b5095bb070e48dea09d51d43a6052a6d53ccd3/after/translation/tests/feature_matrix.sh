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
            if (name != "default") print name
        }
    ' Cargo.toml
)

combination_count=$((1 << ${#features[@]}))
for ((mask = 0; mask < combination_count; mask++)); do
    selected=()
    for ((index = 0; index < ${#features[@]}; index++)); do
        if ((mask & (1 << index))); then
            selected+=("${features[index]}")
        fi
    done

    cargo_args=(--no-default-features)
    if ((${#selected[@]})); then
        feature_list=$(IFS=,; printf '%s' "${selected[*]}")
        cargo_args+=(--features "$feature_list")
    else
        feature_list="<none>"
    fi

    printf 'testing feature combination: %s\n' "$feature_list"
    timeout 600 cargo check "${cargo_args[@]}"
    timeout 600 cargo build --release "${cargo_args[@]}"
    timeout 600 cargo test "${cargo_args[@]}" -- --test-threads=1
done
