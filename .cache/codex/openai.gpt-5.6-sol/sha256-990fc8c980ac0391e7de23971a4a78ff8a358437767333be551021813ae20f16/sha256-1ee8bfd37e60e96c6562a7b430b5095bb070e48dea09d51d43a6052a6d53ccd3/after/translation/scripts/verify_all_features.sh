#!/usr/bin/env bash
set -euo pipefail

crate_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$crate_dir"

mapfile -t features < <(
    awk '
        /^\[features\]$/ { in_features = 1; next }
        /^\[/ { in_features = 0 }
        in_features && /^[[:space:]]*[A-Za-z0-9_-]+[[:space:]]*=/ {
            name = $0
            sub(/^[[:space:]]*/, "", name)
            sub(/[[:space:]]*=.*/, "", name)
            if (name != "default") print name
        }
    ' Cargo.toml
)

run_configuration() {
    local label="$1"
    shift
    printf 'Verifying %s\n' "$label"
    timeout 600 cargo check "$@"
    timeout 600 cargo build --release "$@"
    timeout 600 cargo test "$@"
}

run_configuration "default features"

feature_count="${#features[@]}"
combination_count=$((1 << feature_count))
for ((mask = 0; mask < combination_count; mask++)); do
    selected=()
    for ((index = 0; index < feature_count; index++)); do
        if ((mask & (1 << index))); then
            selected+=("${features[index]}")
        fi
    done

    args=(--no-default-features)
    label="no default features"
    if ((${#selected[@]})); then
        feature_csv="$(IFS=,; printf '%s' "${selected[*]}")"
        args+=(--features "$feature_csv")
        label+=" + $feature_csv"
    fi
    run_configuration "$label" "${args[@]}"
done
