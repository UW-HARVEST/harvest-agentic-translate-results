#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"

mapfile -t features < <(
    awk '
        /^\[features\]$/ { in_features = 1; next }
        /^\[/ { in_features = 0 }
        in_features && /^[[:space:]]*[A-Za-z0-9_-]+[[:space:]]*=/ {
            name = $0
            sub(/[[:space:]]*=.*/, "", name)
            gsub(/^[[:space:]]+|[[:space:]]+$/, "", name)
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

for combination in "${combinations[@]}"; do
    feature_args=(--no-default-features)
    label="<none>"
    if [[ -n "$combination" ]]; then
        feature_args+=(--features "$combination")
        label="$combination"
    fi

    printf 'Testing feature combination: %s\n' "$label"
    timeout 600 cargo build --release "${feature_args[@]}"
    TFM_RUST_SO="$PWD/target/release/libtfm_lib.so" \
        timeout 600 cargo test "${feature_args[@]}" -- --test-threads=1
done
