#!/bin/sh
set -eu

cd "$(dirname "$0")"

run_mode() {
    timeout 600 cargo check "$@"
    timeout 600 cargo build --release "$@"
    timeout 600 cargo test "$@" -- --nocapture
}

run_mode
run_mode --no-default-features
