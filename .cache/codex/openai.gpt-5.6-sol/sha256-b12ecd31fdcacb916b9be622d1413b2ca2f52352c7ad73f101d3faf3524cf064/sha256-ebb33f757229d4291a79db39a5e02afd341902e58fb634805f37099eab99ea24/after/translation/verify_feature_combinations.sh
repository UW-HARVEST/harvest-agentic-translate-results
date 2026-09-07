#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"

run_combination() {
    local label="$1"
    shift
    echo "== $label =="
    timeout 600 cargo check "$@"
    timeout 600 cargo build --release "$@"
    timeout 600 cargo test --release "$@" -- --test-threads=1
}

run_combination default
run_combination no-default-features --no-default-features
