#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"

feature_count="$(
    timeout 600 cargo metadata --no-deps --format-version 1 |
        python3 -c 'import json,sys; print(len(json.load(sys.stdin)["packages"][0].get("features", {})))'
)"

if [[ "$feature_count" != "0" ]]; then
    echo "Cargo features were added; extend this script to enumerate their valid combinations." >&2
    exit 1
fi

for mode in default no-default-features; do
    args=()
    if [[ "$mode" == "no-default-features" ]]; then
        args+=(--no-default-features)
    fi

    timeout 600 cargo check "${args[@]}"
    timeout 600 cargo build --release "${args[@]}"
    timeout 600 cargo test "${args[@]}" -- --test-threads=1
done
