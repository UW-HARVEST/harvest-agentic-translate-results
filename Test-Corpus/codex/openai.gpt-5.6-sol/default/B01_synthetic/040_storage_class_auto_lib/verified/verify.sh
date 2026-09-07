#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"

c_so="../c_src/build/libdriver.so"
rust_so="target/release/libdriver.so"

if [[ ! -f "$c_so" ]]; then
    echo "missing C shared library: $c_so" >&2
    exit 1
fi

run_configuration() {
    local name="$1"
    shift

    echo "verifying Cargo configuration: $name"
    timeout 600 cargo check "$@"
    timeout 600 cargo build --release "$@"
    timeout 600 cargo test --release "$@"
}

run_configuration default
run_configuration no-default-features --no-default-features

c_symbols="$(mktemp)"
rust_symbols="$(mktemp)"
missing_symbols="$(mktemp)"
ldd_output="$(mktemp)"
trap 'rm -f "$c_symbols" "$rust_symbols" "$missing_symbols" "$ldd_output"' EXIT

timeout 600 nm -D --defined-only "$c_so" |
    awk '{print $3}' |
    sort -u >"$c_symbols"
timeout 600 nm -D --defined-only "$rust_so" |
    awk '{print $3}' |
    sort -u >"$rust_symbols"
comm -23 "$c_symbols" "$rust_symbols" >"$missing_symbols"

if [[ -s "$missing_symbols" ]]; then
    echo "Rust shared library is missing C exports:" >&2
    sed 's/^/  /' "$missing_symbols" >&2
    exit 1
fi

timeout 600 ldd -r "$rust_so" >"$ldd_output" 2>&1
if grep -q 'undefined symbol' "$ldd_output"; then
    echo "Rust shared library has unresolved dynamic symbols" >&2
    cat "$ldd_output" >&2
    exit 1
fi

echo "all configurations, differential tests, and symbol checks passed"
