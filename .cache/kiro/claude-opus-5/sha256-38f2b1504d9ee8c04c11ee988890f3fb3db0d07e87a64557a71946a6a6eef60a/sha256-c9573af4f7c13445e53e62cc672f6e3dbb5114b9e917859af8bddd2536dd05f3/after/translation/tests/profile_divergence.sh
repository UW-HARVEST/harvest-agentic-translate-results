#!/usr/bin/env bash
# Runs the full differential suite against a non-release build of the cdylib to
# document exactly which rows are profile-sensitive.
#
# Rationale: cargo's dev profile enables `-C debug-assertions`, which makes
# rustc insert null/alignment checks on raw-pointer dereferences. The C `.so`
# (built by CMake with no instrumentation) has no such checks, so on the C's
# UB inputs -- append_to_buffer(NULL, ..), perform_operation(.., NULL) -- the
# debug-built Rust aborts (SIGABRT) where the C faults (SIGSEGV). The release
# `.so` is the verified artifact and matches the C on those rows.
set -uo pipefail
cd "$(dirname "$0")/.." || exit 1

cargo build --lib >/dev/null 2>&1 || { echo "dev build failed"; exit 1; }
DEV_SO="$PWD/target/debug/libbuffapp_lib.so"
[ -f "$DEV_SO" ] || { echo "missing $DEV_SO"; exit 1; }

echo "== full suite against the RELEASE cdylib (the verified artifact) =="
cargo test --release -- --test-threads=4 2>&1 | grep -E 'test result|stdout_diff result|FAILED'

echo
echo "== full suite against the DEV cdylib (debug-assertions on) =="
BUFFAPP_RUST_SO="$DEV_SO" cargo test --release -- --test-threads=4 2>&1 \
  | grep -E 'test result|stdout_diff result|^test .*FAILED|^    row|^    generic|^    crash|panicked'
