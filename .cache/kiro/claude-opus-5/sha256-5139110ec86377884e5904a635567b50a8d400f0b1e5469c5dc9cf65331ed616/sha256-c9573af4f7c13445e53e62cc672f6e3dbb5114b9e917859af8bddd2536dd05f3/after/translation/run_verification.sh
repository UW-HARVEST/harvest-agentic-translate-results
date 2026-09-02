#!/usr/bin/env bash
# Full differential verification runner.
#
# Deliberately deletes the artifacts before rebuilding: cargo's mtime-based
# fingerprinting can consider a same-second source edit "up to date", which
# silently tests a STALE .so. Never trust `cargo test` alone here.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CRATE="$ROOT/translation"

echo "=== 1. Rebuild the C shared library ==="
mkdir -p "$ROOT/c_src/build"
(cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . --clean-first >/dev/null)
C_SO="$ROOT/c_src/build/libdriver.so"
test -f "$C_SO"

echo "=== 2. Rebuild the Rust cdylib (forced) ==="
cd "$CRATE"
rm -f target/release/libdriver.so target/debug/libdriver.so
touch src/lib.rs
timeout 600 cargo build --release >/dev/null
RUST_SO="$CRATE/target/release/libdriver.so"
test -f "$RUST_SO"

echo "=== 3. Symbol parity (Phase A / D) ==="
nm -D --defined-only "$C_SO"    | grep -v ' [a-z] ' | awk '{print $3}' | sort > /tmp/c_syms.txt
nm -D --defined-only "$RUST_SO" | grep -v ' [a-z] ' | awk '{print $3}' | sort > /tmp/r_syms.txt
echo "--- C exports:";    cat /tmp/c_syms.txt
echo "--- Rust exports:"; cat /tmp/r_syms.txt
MISSING=$(comm -23 /tmp/c_syms.txt /tmp/r_syms.txt || true)
if [ -n "$MISSING" ]; then
  echo "FAIL: symbols exported by C but MISSING from Rust:"; echo "$MISSING"; exit 1
fi
echo "OK: 0 symbols missing from the Rust .so"

echo "=== 4. Feature combinations (Phase D) ==="
"$CRATE/check_features.sh"

echo "=== 5. Differential tests against the RELEASE .so ==="
RUST_SO="$RUST_SO" C_SO="$C_SO" timeout 600 cargo test --release -- --test-threads=4

echo "=== 6. Differential tests against the DEBUG .so (overflow checks on) ==="
rm -f target/debug/libdriver.so
touch src/lib.rs
timeout 600 cargo build >/dev/null
RUST_SO="$CRATE/target/debug/libdriver.so" C_SO="$C_SO" \
  timeout 600 cargo test --release -- --test-threads=4

echo
echo "=== ALL PHASES PASSED ==="
