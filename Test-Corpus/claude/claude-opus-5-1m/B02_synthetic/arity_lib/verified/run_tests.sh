#!/usr/bin/env bash
# Full differential verification driver.
#
#   1. build the C shared library (ground truth)
#   2. for EVERY feature combination: build the Rust cdylib, then run the
#      differential test suite against both .so files
#
# Tests are run with --test-threads=1 so that the glibc tcache parity that
# `compare_allocations` observes is exercised deterministically
# (see ERRORS.md Note B).
set -u
cd "$(dirname "$0")" || exit 1

echo "=== [1/2] building C ground-truth library ==============================="
(
  mkdir -p c_src/build && cd c_src/build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build .
) || { echo "C BUILD FAILED"; exit 1; }
ls -l c_src/build/libtranslated_rust.so || exit 1

echo
echo "=== [2/2] differential tests over every feature combination ============="

FEATURES=$(awk '
  /^\[features\]/ {inf=1; next}
  /^\[/           {inf=0}
  inf && /^[a-zA-Z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
  }' Cargo.toml)

COMBOS=("")
for f in $FEATURES; do
  new=()
  for c in "${COMBOS[@]}"; do
    new+=("$c")
    if [ -z "$c" ]; then new+=("$f"); else new+=("$c,$f"); fi
  done
  COMBOS=("${new[@]}")
done

fail=0
run_combo() {
  local label="$1"; shift
  echo
  echo "-----------------------------------------------------------------------"
  echo ">>> FEATURE COMBO: $label   (cargo build/test $*)"
  echo "-----------------------------------------------------------------------"
  # The cdylib must exist on disk before the tests dlopen it.
  if ! timeout 600 cargo build "$@"; then
    echo "--- BUILD FAIL: $label"; fail=1; return
  fi
  if timeout 600 cargo test "$@" -- --test-threads=1; then
    echo "--- PASS: $label"
  else
    echo "--- FAIL: $label"; fail=1
  fi
}

for c in "${COMBOS[@]}"; do
  if [ -z "$c" ]; then
    run_combo "no-default-features (empty set)" --no-default-features
  else
    run_combo "no-default-features + $c" --no-default-features --features "$c"
  fi
done
run_combo "default features"
run_combo "all features" --all-features

echo
echo "======================================================================="
if [ "$fail" -eq 0 ]; then
  echo "RESULT: ALL FEATURE COMBINATIONS PASSED"
else
  echo "RESULT: FAILURES PRESENT"
fi
exit "$fail"
