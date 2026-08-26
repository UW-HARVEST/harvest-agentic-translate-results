#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
c_library="$root/c_src/build/libSimpleList.so"
rust_library="$root/target/debug/libSimpleList.so"

timeout 600 cmake -S "$root/c_src" -B "$root/c_src/build" \
    -DCMAKE_POSITION_INDEPENDENT_CODE=ON
timeout 600 cmake --build "$root/c_src/build"

# Cargo.toml has no feature declarations, so this is the complete combination set.
feature_combinations=("")
for combination in "${feature_combinations[@]}"; do
    cargo_args=(--no-default-features)
    if [[ -n "$combination" ]]; then
        cargo_args+=(--features "$combination")
    fi

    (
        cd "$root"
        timeout 600 cargo check "${cargo_args[@]}"
        # cargo test builds a test harness rather than the cdylib artifact.
        timeout 600 cargo build "${cargo_args[@]}"
        timeout 600 cargo test "${cargo_args[@]}" -- --test-threads=1
    )
done

missing_symbols="$(
    comm -23 \
        <(nm -D --defined-only --format=posix "$c_library" | awk '{print $1}' | sort -u) \
        <(nm -D --defined-only --format=posix "$rust_library" | awk '{print $1}' | sort -u)
)"

if [[ -n "$missing_symbols" ]]; then
    printf 'Rust shared library is missing C exports:\n%s\n' "$missing_symbols" >&2
    exit 1
fi

printf 'All feature combinations, differential tests, and symbols passed.\n'
