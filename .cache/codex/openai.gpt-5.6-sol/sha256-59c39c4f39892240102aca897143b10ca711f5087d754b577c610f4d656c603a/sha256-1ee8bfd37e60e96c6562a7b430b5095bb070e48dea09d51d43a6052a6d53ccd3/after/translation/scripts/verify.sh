#!/usr/bin/env bash
set -euo pipefail

crate_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$crate_root"

if grep -q '^\[features\]' Cargo.toml; then
    echo "Cargo features were added; enumerate their combinations in scripts/verify.sh." >&2
    exit 1
fi

for mode in default no-default-features; do
    args=()
    if [[ "$mode" == no-default-features ]]; then
        args+=(--no-default-features)
    fi

    echo "== $mode =="
    timeout 600 cargo check --all-targets "${args[@]}"
    timeout 600 cargo build --release "${args[@]}"
    timeout 600 cargo test "${args[@]}" -- --test-threads=1
done
