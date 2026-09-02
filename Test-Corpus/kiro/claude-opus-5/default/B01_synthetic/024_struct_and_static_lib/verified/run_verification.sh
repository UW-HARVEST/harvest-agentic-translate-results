#!/usr/bin/env bash
# End-to-end verification runner: builds the C shared library, builds the Rust
# cdylib, diffs the exported symbol sets, and runs every differential test under
# every feature combination.
#
#   ./run_verification.sh
set -uo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(dirname "$HERE")"
C_SO="$ROOT/c_src/build/libdriver.so"
RUST_SO="$HERE/target/release/libdriver.so"

step() { echo; echo "### $*"; }

step "1/5  build the C shared library"
mkdir -p "$ROOT/c_src/build" || exit 1
( cd "$ROOT/c_src/build" \
  && timeout 300 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && timeout 300 cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
ls -l "$C_SO" || exit 1

step "2/5  cargo check"
( cd "$HERE" && timeout 300 cargo check 2>&1 | tail -5 ) || exit 1

step "3/5  build the Rust cdylib (release — this is what the tests dlopen)"
( cd "$HERE" && timeout 300 cargo build --release 2>&1 | tail -3 ) || exit 1
ls -l "$RUST_SO" || exit 1

step "4/5  exported symbol diff (must be empty)"
filter() { nm -D --defined-only "$1" | awk '{print $NF}' \
    | grep -vE '^(__|_ITM_|_Z)|^_(init|fini|edata|end|DYNAMIC|GLOBAL_OFFSET_TABLE_)$' \
    | sort -u; }
echo "-- C exports --";    filter "$C_SO"
echo "-- Rust exports --"; filter "$RUST_SO"
echo "-- diff (C vs Rust) --"
if diff <(filter "$C_SO") <(filter "$RUST_SO"); then
  echo "symbol sets identical"
else
  echo "SYMBOL DIFF NOT EMPTY"; exit 1
fi

step "5/5  differential tests across every feature combination"
"$HERE/check_all_features.sh" || exit 1

echo
echo "VERIFICATION COMPLETE"
