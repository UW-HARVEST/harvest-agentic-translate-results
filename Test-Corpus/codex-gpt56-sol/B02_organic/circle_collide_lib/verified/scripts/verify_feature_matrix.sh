#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

mapfile -t features < <(
    awk '
        /^\[features\]$/ { in_features = 1; next }
        /^\[/ { in_features = 0 }
        in_features && /^[[:space:]]*[A-Za-z0-9_-]+[[:space:]]*=/ {
            name = $0
            sub(/=.*/, "", name)
            gsub(/[[:space:]]/, "", name)
            if (name != "default") print name
        }
    ' Cargo.toml
)

combination_count=$((1 << ${#features[@]}))
for ((mask = 0; mask < combination_count; mask++)); do
    enabled=()
    for ((index = 0; index < ${#features[@]}; index++)); do
        if ((mask & (1 << index))); then
            enabled+=("${features[index]}")
        fi
    done

    args=(--no-default-features)
    label="<empty>"
    if ((${#enabled[@]})); then
        combo=$(IFS=,; echo "${enabled[*]}")
        args+=(--features "$combo")
        label="$combo"
    fi

    printf 'Verifying feature combination: %s\n' "$label"
    timeout 600 cargo check "${args[@]}"
    timeout 600 cargo build --release "${args[@]}"
    timeout 600 cargo test "${args[@]}" -- --test-threads=1
done
