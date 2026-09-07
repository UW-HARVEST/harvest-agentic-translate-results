#!/usr/bin/env bash
# Full differential verification run: build the C reference .so, build the Rust
# cdylib, then run every phase's tests under every feature combination.
#
# The tests redirect the process-wide stdout fd, so they must run
# single-threaded (RUST_TEST_THREADS=1 + --test-threads=1).
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(dirname "$HERE")"

echo "=== building C reference shared library ==="
mkdir -p "$ROOT/c_src/build"
(cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null)
C_SO="$ROOT/c_src/build/libString_Slice.so"
test -f "$C_SO"
echo "  -> $C_SO"

cd "$HERE"

# Enumerate feature combinations declared in Cargo.toml. This crate declares
# none, so the set is just {default, no-default-features}; the loop is written
# generically so it stays correct if features are ever added.
mapfile -t FEATURES < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/ {split($0,a,"="); gsub(/ /,"",a[1]); if (a[1] != "default") print a[1]}' Cargo.toml
)

COMBOS=("--all-features")
COMBOS+=("--no-default-features")
for f in "${FEATURES[@]:-}"; do
  [ -n "$f" ] && COMBOS+=("--no-default-features --features $f")
done

status=0
for combo in "${COMBOS[@]}"; do
  echo
  echo "=== cargo test $combo ==="
  # shellcheck disable=SC2086
  timeout 600 cargo build $combo >/dev/null 2>&1
  # shellcheck disable=SC2086
  if ! RUST_TEST_THREADS=1 C_SO="$C_SO" timeout 600 cargo test $combo -- --test-threads=1; then
    echo "!!! FAILED for: $combo"
    status=1
  fi
done

echo
echo "=== release build + symbol parity ==="
timeout 600 cargo build --release >/dev/null 2>&1
RUST_SO="$ROOT/translation/target/release/libString_Slice.so"
diff <(nm -D --defined-only "$C_SO"    | awk '{print $2, $3}' | sort) \
     <(nm -D --defined-only "$RUST_SO" | awk '{print $2, $3}' | sort) \
  && echo "symbol diff: EMPTY (parity OK)" \
  || { echo "!!! SYMBOL DIFF NOT EMPTY"; status=1; }

# Also run the tests against the release .so, since panic=abort and
# optimisation could in principle change observable behaviour.
echo
echo "=== differential tests against the RELEASE Rust .so ==="
RUST_TEST_THREADS=1 C_SO="$C_SO" RUST_SO="$RUST_SO" timeout 600 cargo test -- --test-threads=1 \
  || { echo "!!! FAILED against release .so"; status=1; }

exit $status
