#!/usr/bin/env bash
set -euo pipefail

crate_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$crate_root"

test -f ../c_src/build/libdriver.so

run_configuration() {
    local name="$1"
    shift

    printf 'Testing debug feature configuration: %s\n' "$name"
    timeout 600 cargo build "$@"
    timeout 600 cargo test "$@" -- --test-threads=1

    printf 'Testing release feature configuration: %s\n' "$name"
    timeout 600 cargo build --release "$@"
    timeout 600 cargo test --release "$@" -- --test-threads=1
}

# Cargo.toml has no [features] table, so these are the only distinct Cargo
# invocations. They are behaviorally equivalent but both completion modes are
# exercised explicitly.
run_configuration default
run_configuration no-default-features --no-default-features
