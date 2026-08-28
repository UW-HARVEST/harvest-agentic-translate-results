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

combinations=("")
for feature in "${features[@]}"; do
    existing=("${combinations[@]}")
    for combination in "${existing[@]}"; do
        if [[ -n "$combination" ]]; then
            combinations+=("$combination,$feature")
        else
            combinations+=("$feature")
        fi
    done
done

run_configuration() {
    local label=$1
    shift
    printf '==> %s\n' "$label"
    timeout 600 cargo check "$@"
    timeout 600 cargo build --release "$@"
    timeout 600 cargo test "$@"
}

run_configuration "default features"
for combination in "${combinations[@]}"; do
    args=(--no-default-features)
    label="no default features"
    if [[ -n "$combination" ]]; then
        args+=(--features "$combination")
        label+=" + $combination"
    fi
    run_configuration "$label" "${args[@]}"
done
