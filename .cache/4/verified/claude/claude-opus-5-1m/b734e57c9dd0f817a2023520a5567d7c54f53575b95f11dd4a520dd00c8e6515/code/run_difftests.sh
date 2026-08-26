#!/usr/bin/env bash
# Build both shared objects, then run the C-vs-Rust differential test suite.
#
#   ./run_difftests.sh                  # default feature set, release profile
#   ./run_difftests.sh --test t20_frame # only that integration-test binary
#   PROFILE=dev ./run_difftests.sh      # debug profile (overflow checks ON)
#
# `cargo test --test <x>` does NOT rebuild the cdylib, so building it here
# explicitly is load-bearing; tests/common/mod.rs also refuses to run against a
# .so older than src/.
set -euo pipefail
cd "$(dirname "$0")"

PROFILE="${PROFILE:-release}"
FEATURES="${FEATURES:-}"
CARGO_FLAGS=(--offline --no-default-features)
if [ -n "$FEATURES" ]; then
  CARGO_FLAGS+=(--features "$FEATURES")
fi
if [ "$PROFILE" = "release" ]; then
  CARGO_FLAGS+=(--release)
  SO_DIR=target/release
else
  SO_DIR=target/debug
fi

echo "=== building C shared object ==="
mkdir -p c_src/build
( cd c_src/build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null && cmake --build . -j "$(nproc)" >/dev/null )
test -f c_src/build/libzstd.so

echo "=== building Rust cdylib (profile=$PROFILE features='${FEATURES:-<none>}') ==="
cargo build "${CARGO_FLAGS[@]}" 2>&1 | tail -3
test -f "$SO_DIR/libzstd.so"

export ZSTD_C_SO="$PWD/c_src/build/libzstd.so"
export ZSTD_RUST_SO="$PWD/$SO_DIR/libzstd.so"

TD="${TMPDIR:-/tmp}"
mkdir -p "$TD"
echo "=== symbol parity ==="
nm -D --defined-only "$ZSTD_C_SO"    | awk '{print $2" "$3}' | sort > "$TD/c_syms.txt"
nm -D --defined-only "$ZSTD_RUST_SO" | awk '{print $2" "$3}' | sort > "$TD/r_syms.txt"
MISSING=$(comm -23 "$TD/c_syms.txt" "$TD/r_syms.txt" | wc -l)
EXTRA=$(comm -13 "$TD/c_syms.txt" "$TD/r_syms.txt" | wc -l)
echo "C: $(wc -l < "$TD/c_syms.txt")  Rust: $(wc -l < "$TD/r_syms.txt")  missing: $MISSING  extra: $EXTRA"
if [ "$MISSING" != "0" ] || [ "$EXTRA" != "0" ]; then
  echo "--- missing in Rust ---"; comm -23 "$TD/c_syms.txt" "$TD/r_syms.txt"
  echo "--- extra in Rust ---";   comm -13 "$TD/c_syms.txt" "$TD/r_syms.txt"
  exit 1
fi

echo "=== differential tests ==="
# Arguments are forwarded to cargo, so e.g.
#   ./run_difftests.sh --test t10_entropy
# runs just that integration-test binary.
#
# Row-coverage tags are wiped first and only folded back into CONFIGS.md /
# ERRORS.md if the whole run succeeded, so a check-box can never be set by a test
# that recorded its tags and then failed.
if [ $# -eq 0 ]; then
  rm -rf target/difftest-coverage
fi
STATUS=0
cargo test "${CARGO_FLAGS[@]}" "$@" -- --test-threads="$(nproc)" || STATUS=$?

if [ "$STATUS" != "0" ]; then
  echo "=== TESTS FAILED (exit $STATUS) — coverage NOT updated ==="
  exit "$STATUS"
fi

if [ $# -eq 0 ]; then
  echo "=== row coverage ==="
  ./coverage.py
fi
