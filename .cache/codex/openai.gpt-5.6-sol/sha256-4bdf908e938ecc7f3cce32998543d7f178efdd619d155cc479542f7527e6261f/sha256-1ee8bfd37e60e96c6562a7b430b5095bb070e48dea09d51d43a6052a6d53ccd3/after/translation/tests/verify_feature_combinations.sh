#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

run_configuration() {
    local label=$1
    shift
    printf 'Verifying feature configuration: %s\n' "$label"
    timeout 600 cargo build --release "$@"
    timeout 600 cargo test "$@" -- --test-threads=1
}

run_configuration default

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

    args=(--no-default-features)
    label=no-default-features
    if ((${#selected[@]})); then
        joined=$(IFS=,; printf '%s' "${selected[*]}")
        args+=(--features "$joined")
        label="$label+$joined"
    fi
    run_configuration "$label" "${args[@]}"
done
