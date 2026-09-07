#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"

for op in add sub mul; do
    for repeat in 0 1 2 3 4 5 6 7; do
        features="${op},${repeat}"
        echo "=== checking ${features} ==="
        timeout 600 cargo check --no-default-features --features "${features}"

        # cargo test builds the library in test mode only. Build the cdylib
        # first so the integration test can load it as an external caller.
        echo "=== building ${features} cdylib ==="
        timeout 600 cargo build --no-default-features --features "${features}"

        echo "=== testing ${features} ==="
        timeout 600 cargo test --no-default-features --features "${features}" -- --test-threads=1
    done
done

echo "=== rebuilding default release artifacts ==="
timeout 600 cargo build --release

echo "=== testing default release artifacts ==="
timeout 600 cargo test --release --no-default-features --features "add,5" -- --test-threads=1
