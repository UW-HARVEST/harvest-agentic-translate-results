#!/bin/bash
# Run the FFI differential tests for one feature combination (default: the
# Cargo default features).  Builds the C libraries and the Rust cdylib first so
# that the test harness has both shared objects to dlopen.
#
# The Rust cdylib is built with --release (the C side is built with -O3) so the
# higher-level tests finish in reasonable time; the test harness itself is a
# normal debug build and reaches the library only through dlopen/dlsym.
#
# Usage: ./run_tests.sh blake simple 128f [extra cargo test args...]
set -u
ROOT="$(cd "$(dirname "$0")" && pwd)"
backend="${1:-blake}"; thash="${2:-simple}"; secpar="${3:-128f}"
shift 3 2>/dev/null || true

"$ROOT/build_c.sh" "$backend" "$secpar" "$thash" >/dev/null || {
  echo "C build failed for ${backend}_${secpar}_${thash}"; exit 1; }

cd "$ROOT/translation"
feat="${backend},${thash},${secpar}${EXTRA_FEATURES:-}"
timeout 600 cargo build --release --no-default-features --features "$feat" > /tmp/rustbuild.log 2>&1 || {
  echo "cargo build failed for $feat"; tail -30 /tmp/rustbuild.log; exit 1; }
export SPHINCS_RUST_SO="$ROOT/translation/target/release/libsphincsplus.so"
exec timeout 600 cargo test --no-default-features --features "$feat" "$@"
