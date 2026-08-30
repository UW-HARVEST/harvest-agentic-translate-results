#!/usr/bin/env bash
set -euo pipefail

crate_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
root_dir=$(cd "$crate_dir/.." && pwd)
c_dir="$root_dir/c_src"
c_so="$c_dir/build/libdriver.so"
rust_so="$crate_dir/target/release/libdriver.so"

timeout 600 cmake -S "$c_dir" -B "$c_dir/build" \
    -DCMAKE_POSITION_INDEPENDENT_CODE=ON
timeout 600 cmake --build "$c_dir/build"

mapfile -t features < <(
    awk '
        /^\[features\][[:space:]]*$/ { in_features = 1; next }
        /^\[/ { in_features = 0 }
        in_features && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
            name = $0
            sub(/[[:space:]]*=.*/, "", name)
            if (name != "default") print name
        }
    ' "$crate_dir/Cargo.toml"
)

cd "$crate_dir"
combination_count=$((1 << ${#features[@]}))
for ((mask = 0; mask < combination_count; mask++)); do
    selected=()
    for ((index = 0; index < ${#features[@]}; index++)); do
        if (((mask & (1 << index)) != 0)); then
            selected+=("${features[index]}")
        fi
    done

    cargo_args=(--no-default-features)
    if ((${#selected[@]} > 0)); then
        feature_csv=$(IFS=,; printf '%s' "${selected[*]}")
        cargo_args+=(--features "$feature_csv")
    fi

    printf 'Verifying feature combination: %s\n' \
        "${feature_csv:-<none>}"
    timeout 600 cargo check --tests "${cargo_args[@]}"
    timeout 600 cargo build --release "${cargo_args[@]}"
    DRIVER_C_SO="$c_so" DRIVER_RUST_SO="$rust_so" \
        timeout 600 cargo test "${cargo_args[@]}" -- --test-threads=1
    unset feature_csv
done

c_symbols=$(mktemp)
rust_symbols=$(mktemp)
missing_symbols=$(mktemp)
trap 'rm -f "$c_symbols" "$rust_symbols" "$missing_symbols"' EXIT

nm -D --defined-only "$c_so" | awk '{ print $3 }' | sort -u >"$c_symbols"
nm -D --defined-only "$rust_so" | awk '{ print $3 }' | sort -u >"$rust_symbols"
comm -23 "$c_symbols" "$rust_symbols" >"$missing_symbols"

if [[ -s "$missing_symbols" ]]; then
    printf 'Rust shared object is missing C exports:\n' >&2
    cat "$missing_symbols" >&2
    exit 1
fi

printf 'Symbol parity passed: %s C exports, 0 missing in Rust.\n' \
    "$(wc -l <"$c_symbols")"
