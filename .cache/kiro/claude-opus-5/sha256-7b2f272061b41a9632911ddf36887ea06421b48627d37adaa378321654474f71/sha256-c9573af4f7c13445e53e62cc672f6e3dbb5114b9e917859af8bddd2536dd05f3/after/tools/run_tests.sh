#!/usr/bin/env bash
# Rebuild BOTH shared objects, then run the differential test suite.
# `cargo test` does not necessarily refresh the cdylib artifact that the tests
# dlopen, so the explicit `cargo build --release` is required.
set -euo pipefail
cd "$(dirname "$0")/.."
ROOT=$(pwd)

# C reference .so
if [ ! -f c_src/build/libsodium.so ]; then
    mkdir -p c_src/build
    (cd c_src/build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null && cmake --build . -j8 >/dev/null)
fi

cd translation
timeout 600 cargo build --release 2>&1 | grep -E '^(error|warning: unused)' || true
if [ ! -f target/release/liblibsodium.so ]; then
    echo "FATAL: Rust .so not built" >&2
    exit 1
fi

# Symbol parity gate
nm -D --defined-only "$ROOT/c_src/build/libsodium.so" | awk '{print $3}' | sort -u > /tmp/gate_c.txt
nm -D --defined-only target/release/liblibsodium.so    | awk '{print $3}' | sort -u > /tmp/gate_r.txt
MISSING=$(comm -23 /tmp/gate_c.txt /tmp/gate_r.txt | wc -l)
echo "symbols: C=$(wc -l < /tmp/gate_c.txt) Rust=$(wc -l < /tmp/gate_r.txt) missing=$MISSING"
if [ "$MISSING" != "0" ]; then
    echo "MISSING SYMBOLS:"; comm -23 /tmp/gate_c.txt /tmp/gate_r.txt
fi

# --test-threads=1: the abort/misuse checks fork(), and fork() in a
# multithreaded process can inherit a held malloc/loader lock and wedge the
# child (see the comment on harness::fork_run). One thread removes the hazard.
if [ $# -gt 0 ]; then
    timeout 600 cargo test --release "$@" -- --test-threads=1
else
    timeout 600 cargo test --release -- --test-threads=1
fi
