#!/usr/bin/env bash
set -euo pipefail

crate_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
c_library="$crate_dir/../c_src/build/libharvest-work-d9nDAf.so"
rust_library="$crate_dir/target/release/libflac_validate_lib.so"

cd "$crate_dir"

check_symbols() {
    local missing
    missing="$(
        comm -23 \
            <(nm -D --defined-only "$c_library" |
                awk '$2 ~ /^[A-Z]$/ { print $3 }' | sort -u) \
            <(nm -D --defined-only "$rust_library" |
                awk '$2 ~ /^[A-Z]$/ { print $3 }' | sort -u)
    )"
    if [[ -n "$missing" ]]; then
        printf 'Missing Rust exports:\n%s\n' "$missing" >&2
        return 1
    fi
}

run_configuration() {
    local label="$1"
    shift
    printf 'Verifying Cargo configuration: %s\n' "$label"
    timeout 600 cargo check --all-targets --release "$@"
    timeout 600 cargo build --release "$@"
    check_symbols
    timeout 600 cargo test --release "$@" -- --nocapture
}

run_configuration default

mapfile -t features < <(
    awk '
        /^\[features\]$/ { in_features = 1; next }
        /^\[/ { in_features = 0 }
        in_features && /^[[:space:]]*[A-Za-z0-9_-]+[[:space:]]*=/ {
            name = $0
            sub(/[[:space:]]*=.*/, "", name)
            sub(/^[[:space:]]*/, "", name)
            if (name != "default") print name
        }
    ' Cargo.toml
)

if (( ${#features[@]} >= 20 )); then
    printf 'Refusing impractical feature power set of %d features\n' \
        "${#features[@]}" >&2
    exit 1
fi

combination_count=$((1 << ${#features[@]}))
for ((mask = 0; mask < combination_count; mask++)); do
    selected=()
    for ((index = 0; index < ${#features[@]}; index++)); do
        if ((mask & (1 << index))); then
            selected+=("${features[index]}")
        fi
    done

    if (( ${#selected[@]} == 0 )); then
        run_configuration no-default --no-default-features
    else
        combo="$(IFS=,; printf '%s' "${selected[*]}")"
        run_configuration "no-default+$combo" \
            --no-default-features --features "$combo"
    fi
done

printf 'All symbols, tests, and Cargo feature combinations passed.\n'
