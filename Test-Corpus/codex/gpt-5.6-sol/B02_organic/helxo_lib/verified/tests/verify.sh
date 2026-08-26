#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

# Cargo.toml and CMakeLists.txt define no feature/configuration axes.
feature_combinations=("")

timeout 600 cmake -S c_src -B c_src/build -DCMAKE_POSITION_INDEPENDENT_CODE=ON
timeout 600 cmake --build c_src/build

for features in "${feature_combinations[@]}"; do
    timeout 600 cargo check --no-default-features --features "$features"
    timeout 600 cargo build --no-default-features --features "$features"
    timeout 600 cargo test --no-default-features --features "$features"
    timeout 600 cargo build --release --no-default-features --features "$features"
    timeout 600 cargo test --release --no-default-features --features "$features"
done

c_symbols="$(mktemp)"
rust_symbols="$(mktemp)"
missing_symbols="$(mktemp)"
trap 'rm -f "$c_symbols" "$rust_symbols" "$missing_symbols"' EXIT

nm -D --defined-only c_src/build/libtranslated_rust.so |
    awk '{print $3}' | sort -u >"$c_symbols"
nm -D --defined-only target/release/libhelxo_lib.so |
    awk '{print $3}' | sort -u >"$rust_symbols"
comm -23 "$c_symbols" "$rust_symbols" >"$missing_symbols"

if [[ -s "$missing_symbols" ]]; then
    echo "Rust shared object is missing C symbols:" >&2
    cat "$missing_symbols" >&2
    exit 1
fi

test "$(grep -Ec '^\| [0-9]+ \| `[^`]+` \|' SYMBOLS.md)" -eq 16
test "$(grep -c '\[x\]' ERRORS.md)" -eq 19
test "$(grep -c '\[ \]' ERRORS.md || true)" -eq 0
test "$(grep -c '\[x\]' CONFIGS.md)" -eq 53
test "$(grep -c '\[ \]' CONFIGS.md || true)" -eq 0

echo "verification complete: 1 feature combination, 16 symbols, 52 runtime configurations, 19 error rows"
