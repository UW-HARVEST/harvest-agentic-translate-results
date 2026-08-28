#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"

c_library="../c_src/build/libharvest-work-tgoXJn.so"
rust_library="target/release/libhdr_bitrate_lib.so"

if [[ ! -f "$c_library" ]]; then
    printf 'missing C shared library: %s\n' "$c_library" >&2
    exit 1
fi

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

run_configuration() {
    local label=$1
    shift
    printf '\n== feature configuration: %s ==\n' "$label"
    timeout 600 cargo check "$@"
    timeout 600 cargo build --release "$@"
    HDR_BITRATE_RUST_LIBRARY="$rust_library" timeout 600 cargo test "$@" -- --nocapture
}

run_configuration default

combination_count=$((1 << ${#features[@]}))
for ((mask = 0; mask < combination_count; mask++)); do
    enabled=()
    for ((index = 0; index < ${#features[@]}; index++)); do
        if ((mask & (1 << index))); then
            enabled+=("${features[index]}")
        fi
    done

    args=(--no-default-features)
    label=no-default-features
    if ((${#enabled[@]})); then
        joined=$(IFS=,; printf '%s' "${enabled[*]}")
        args+=(--features "$joined")
        label+=" + $joined"
    fi
    run_configuration "$label" "${args[@]}"
done

c_symbols=$(mktemp)
rust_symbols=$(mktemp)
missing_symbols=$(mktemp)
trap 'rm -f "$c_symbols" "$rust_symbols" "$missing_symbols"' EXIT

nm -D --defined-only "$c_library" | awk '{print $3}' | sort -u >"$c_symbols"
nm -D --defined-only "$rust_library" | awk '{print $3}' | sort -u >"$rust_symbols"
comm -23 "$c_symbols" "$rust_symbols" >"$missing_symbols"

if [[ -s "$missing_symbols" ]]; then
    printf '\nC symbols missing from Rust:\n' >&2
    cat "$missing_symbols" >&2
    exit 1
fi

printf '\nAll feature configurations and dynamic symbols verified.\n'
