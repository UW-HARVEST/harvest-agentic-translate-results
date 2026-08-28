#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

c_so="../c_src/build/libharvest-work-cstVVS.so"
rust_so="target/release/libima_parse_lib.so"

if [[ ! -f "$c_so" ]]; then
    printf 'C shared object is missing: %s\n' "$c_so" >&2
    exit 1
fi

run_combo() {
    local label=$1
    shift
    local cargo_args=("$@")

    printf '== feature combination: %s ==\n' "$label"
    timeout 600 cargo check "${cargo_args[@]}"
    timeout 600 cargo build --release "${cargo_args[@]}"
    timeout 600 cargo test "${cargo_args[@]}" -- --test-threads=1
}

run_combo default

mapfile -t features < <(
    awk '
        /^\[features\]$/ { in_features = 1; next }
        /^\[/ { in_features = 0 }
        in_features && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
            sub(/[[:space:]]*=.*/, "")
            if ($0 != "default") print
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
        feature_csv=$(IFS=,; printf '%s' "${selected[*]}")
        args+=(--features "$feature_csv")
        label=$feature_csv
    fi
    run_combo "$label" "${args[@]}"
done

c_symbols=$(mktemp)
rust_symbols=$(mktemp)
missing_symbols=$(mktemp)
relocations=$(mktemp)
trap 'rm -f "$c_symbols" "$rust_symbols" "$missing_symbols" "$relocations"' EXIT

nm -D --defined-only "$c_so" | awk '{print $3}' | sort -u >"$c_symbols"
nm -D --defined-only "$rust_so" | awk '{print $3}' | sort -u >"$rust_symbols"
comm -23 "$c_symbols" "$rust_symbols" >"$missing_symbols"

if [[ -s "$missing_symbols" ]]; then
    printf 'Rust is missing C dynamic symbols:\n' >&2
    cat "$missing_symbols" >&2
    exit 1
fi

ldd -r "$rust_so" >"$relocations" 2>&1
if grep -q 'undefined symbol' "$relocations"; then
    cat "$relocations" >&2
    exit 1
fi

printf 'symbol parity: 0 missing\n'
printf 'runtime relocations: all resolved\n'
