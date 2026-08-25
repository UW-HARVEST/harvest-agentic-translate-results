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
            print name
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

    args=(--no-default-features)
    label="<empty>"
    if ((${#selected[@]})); then
        combo=$(IFS=,; printf '%s' "${selected[*]}")
        args+=(--features "$combo")
        label="$combo"
    fi

    printf 'Checking feature combination: %s\n' "$label"
    timeout 600 cargo check "${args[@]}"
    timeout 600 cargo test "${args[@]}"
done
