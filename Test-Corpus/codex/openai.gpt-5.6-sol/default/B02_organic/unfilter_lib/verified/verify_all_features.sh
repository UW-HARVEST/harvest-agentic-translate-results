#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"

c_so="$(find ../c_src/build -maxdepth 1 -type f -name '*.so' -print -quit)"
rust_so="target/release/libunfilter_lib.so"
if [[ -z "$c_so" ]]; then
    echo "C shared library is missing; build c_src first" >&2
    exit 1
fi

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

feature_count="${#features[@]}"
combination_count=$((1 << feature_count))

for ((mask = 0; mask < combination_count; mask++)); do
    selected=()
    for ((index = 0; index < feature_count; index++)); do
        if ((mask & (1 << index))); then
            selected+=("${features[index]}")
        fi
    done

    cargo_args=(--no-default-features)
    label="<none>"
    if ((${#selected[@]})); then
        feature_list="$(IFS=,; echo "${selected[*]}")"
        cargo_args+=(--features "$feature_list")
        label="$feature_list"
    fi

    echo "== feature combination: $label =="
    timeout 600 cargo check "${cargo_args[@]}"
    timeout 600 cargo build --release "${cargo_args[@]}"
    timeout 600 cargo test "${cargo_args[@]}" -- --test-threads=1

    missing="$(
        comm -23 \
            <(nm -D --defined-only "$c_so" | awk '{print $3}' | sort -u) \
            <(nm -D --defined-only "$rust_so" | awk '{print $3}' | sort -u)
    )"
    if [[ -n "$missing" ]]; then
        echo "Rust shared library is missing C symbols:" >&2
        echo "$missing" >&2
        exit 1
    fi
done

echo "verified $combination_count feature combination(s)"
