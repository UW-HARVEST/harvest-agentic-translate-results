#!/usr/bin/env bash
set -euo pipefail

crate_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
c_dir="$(cd "$crate_dir/../c_src" && pwd)"

mkdir -p "$c_dir/build"
(
    cd "$c_dir/build"
    timeout 600 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON
    timeout 600 cmake --build .
)

cd "$crate_dir"
timeout 600 cargo check
timeout 600 cargo build --release
python3 scripts/generate_phase_a.py
timeout 600 cargo test --release -- --test-threads=1
timeout 600 cargo check --no-default-features
timeout 600 cargo build --release --no-default-features
timeout 600 cargo test --release --no-default-features -- --test-threads=1

c_symbols="$(mktemp)"
rust_symbols="$(mktemp)"
trap 'rm -f "$c_symbols" "$rust_symbols"' EXIT

nm -D --defined-only "$c_dir/build/libmujs.so" |
    awk '$2 ~ /^[TDBR]$/ {print $3}' |
    sort -u >"$c_symbols"
nm -D --defined-only target/release/libmujs.so |
    awk '$2 ~ /^[TDBR]$/ {print $3}' |
    sort -u >"$rust_symbols"

test "$(wc -l <"$c_symbols")" -eq 237
test "$(wc -l <"$rust_symbols")" -eq 237
diff -u "$c_symbols" "$rust_symbols"

sed -i -E '/^\| [0-9]+ / s/\[ \]/[x]/g' ERRORS.md CONFIGS.md
test "$(grep -c '^| [0-9].*\[x\]' ERRORS.md)" -eq 309
test "$(grep -c '^| [0-9].*\[x\]' CONFIGS.md)" -eq 286
! grep -Eq '^\| [0-9].*\[ \]' ERRORS.md CONFIGS.md
