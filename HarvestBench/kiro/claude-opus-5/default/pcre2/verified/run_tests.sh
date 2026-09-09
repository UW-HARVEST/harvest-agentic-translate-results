#!/bin/bash
# Rebuild the cdylib FIRST: `cargo test` does not refresh the cdylib artifact
# that the differential tests dlopen, only the (unused) lib test target.
set -e
cd "$(dirname "$0")"
cargo build --release 2>&1 | grep -Ev '^\s*(Compiling|Finished)' || true
cargo build --release --tests 2>&1 | tail -1
exec cargo test --release "$@"
