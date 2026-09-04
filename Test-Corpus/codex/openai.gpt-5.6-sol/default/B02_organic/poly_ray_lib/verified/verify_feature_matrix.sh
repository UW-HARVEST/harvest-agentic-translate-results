#!/usr/bin/env bash
set -euo pipefail

run_combo() {
    local label=$1
    shift
    echo "== ${label}: check =="
    timeout 600 cargo check "$@"
    echo "== ${label}: release cdylib =="
    timeout 600 cargo build --release "$@"
    echo "== ${label}: differential tests =="
    timeout 600 cargo test "$@" --test differential
}

run_combo default
run_combo no-default-features --no-default-features
