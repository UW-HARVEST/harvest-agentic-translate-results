#!/usr/bin/env bash
set -euo pipefail

crate_directory="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "${crate_directory}"

c_library="${crate_directory}/../c_src/build/libdriver.so"
rust_library="${crate_directory}/target/release/libdriver.so"

if [[ ! -f "${c_library}" ]]; then
    printf 'C shared library is missing: %s\n' "${c_library}" >&2
    exit 1
fi

mapfile -t named_features < <(
    timeout 600 cargo metadata --no-deps --format-version 1 |
        jq -r '.packages[] | select(.name == "driver") | .features | keys[] | select(. != "default")'
)

verify_current_build() {
    local missing_symbols
    local unresolved_symbols

    missing_symbols="$(
        comm -23 \
            <(nm -D --defined-only "${c_library}" | awk '{print $3}' | sort -u) \
            <(nm -D --defined-only "${rust_library}" | awk '{print $3}' | sort -u)
    )"
    if [[ -n "${missing_symbols}" ]]; then
        printf 'Rust shared library is missing C exports:\n%s\n' "${missing_symbols}" >&2
        exit 1
    fi

    unresolved_symbols="$(ldd -r "${rust_library}" 2>&1 | rg 'undefined symbol' || true)"
    if [[ -n "${unresolved_symbols}" ]]; then
        printf 'Rust shared library has unresolved symbols:\n%s\n' "${unresolved_symbols}" >&2
        exit 1
    fi
}

run_configuration() {
    local label="$1"
    shift
    local cargo_arguments=("$@")

    printf 'Verifying feature configuration: %s\n' "${label}"
    timeout 600 cargo check "${cargo_arguments[@]}"
    timeout 600 cargo build --release "${cargo_arguments[@]}"
    timeout 600 cargo test "${cargo_arguments[@]}" -- --test-threads=1
    verify_current_build
}

run_configuration default

combination_count=$((1 << ${#named_features[@]}))
for ((mask = 0; mask < combination_count; mask++)); do
    enabled_features=()
    for ((index = 0; index < ${#named_features[@]}; index++)); do
        if (((mask & (1 << index)) != 0)); then
            enabled_features+=("${named_features[index]}")
        fi
    done

    feature_arguments=(--no-default-features)
    feature_label="no-default"
    if ((${#enabled_features[@]} > 0)); then
        feature_specification="$(
            IFS=,
            printf '%s' "${enabled_features[*]}"
        )"
        feature_arguments+=(--features "${feature_specification}")
        feature_label="${feature_label}+${feature_specification}"
    fi

    run_configuration "${feature_label}" "${feature_arguments[@]}"
done
