#!/bin/sh
# Differential-test runner.
#
# `cargo test` does not rebuild a cdylib-only lib target for integration tests,
# so the Rust .so must be built explicitly first. Always use this script (never
# a bare `cargo test`) or the tests may load a stale Rust library.
set -e
cd "$(dirname "$0")"
cargo build --offline --release
exec cargo test --offline --release "$@"
