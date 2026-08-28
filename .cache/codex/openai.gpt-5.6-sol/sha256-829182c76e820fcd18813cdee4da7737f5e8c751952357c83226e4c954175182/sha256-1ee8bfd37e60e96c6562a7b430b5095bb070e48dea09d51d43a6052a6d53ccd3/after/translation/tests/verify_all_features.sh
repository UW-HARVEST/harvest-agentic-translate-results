#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

c_library="../c_src/build/libharvest-work-kx3K47.so"
rust_library="target/release/libwcscat_lib.so"

mapfile -t features < <(
    awk '
        /^\[features\]$/ { in_features = 1; next }
        /^\[/ { in_features = 0 }
        in_features && /^[[:space:]]*[A-Za-z0-9_-]+[[:space:]]*=/ {
            key = $0
            sub(/[[:space:]]*=.*/, "", key)
            gsub(/^[[:space:]]+|[[:space:]]+$/, "", key)
            if (key != "default") print key
        }
    ' Cargo.toml
)

run_configuration() {
    local label=$1
    shift

    printf '\n==> Verifying %s\n' "$label"
    timeout 600 cargo check "$@"
    timeout 600 cargo build --release "$@"
    timeout 600 cargo test --release "$@"

    nm -D --defined-only --format=posix "$c_library" |
        awk '{ print $1 }' | sort -u >target/c-symbols.txt
    nm -D --defined-only --format=posix "$rust_library" |
        awk '{ print $1 }' | sort -u >target/rust-symbols.txt
    comm -23 target/c-symbols.txt target/rust-symbols.txt >target/missing-symbols.txt
    test ! -s target/missing-symbols.txt
}

run_configuration "default features"

feature_count=${#features[@]}
combination_count=$((1 << feature_count))
for ((mask = 0; mask < combination_count; mask++)); do
    selected=()
    for ((index = 0; index < feature_count; index++)); do
        if (((mask & (1 << index)) != 0)); then
            selected+=("${features[index]}")
        fi
    done

    args=(--no-default-features)
    label="no default features"
    if ((${#selected[@]} > 0)); then
        feature_csv=$(IFS=,; printf '%s' "${selected[*]}")
        args+=(--features "$feature_csv")
        label+=" + $feature_csv"
    fi
    run_configuration "$label" "${args[@]}"
done

