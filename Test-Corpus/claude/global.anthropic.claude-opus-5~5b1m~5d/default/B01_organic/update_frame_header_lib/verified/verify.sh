#!/usr/bin/env bash
# Full verification sweep: builds the C .so and the Rust .so, then runs the
# differential suite for every feature combination and for both the release and
# the debug Rust cdylib (debug turns Rust's arithmetic overflow checks ON).
set -euo pipefail

cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"
CARGO="cargo --offline"

echo "=== building C shared library ==="
mkdir -p "$ROOT/c_src/build"
(cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null)
C_SO="$(find "$ROOT/c_src/build" -maxdepth 1 -name '*.so' | sort | head -1)"
echo "C .so: $C_SO"

# Enumerate feature combinations from Cargo.toml. This crate declares no
# [features] table, so the only combination is the default (empty) one; the
# loop is written generically so it keeps working if features are added.
mapfile -t FEATURES < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/ {split($0,a,"="); gsub(/ /,"",a[1]); if (a[1] != "default") print a[1]}' Cargo.toml
)
echo "=== declared features: ${#FEATURES[@]} (${FEATURES[*]:-none}) ==="

COMBOS=("")            # default features
if [ "${#FEATURES[@]}" -gt 0 ]; then
  COMBOS+=("--no-default-features")
  for f in "${FEATURES[@]}"; do
    COMBOS+=("--no-default-features --features $f")
  done
  COMBOS+=("--all-features")
fi

FAIL=0
for combo in "${COMBOS[@]}"; do
  label="${combo:-<default features>}"

  echo
  echo "############ $label ############"
  # shellcheck disable=SC2086
  $CARGO check $combo --all-targets 2>&1 | tail -3

  echo "--- building both Rust cdylib profiles ---"
  # shellcheck disable=SC2086
  $CARGO build --release $combo >/dev/null 2>&1
  cp target/release/libupdate_frame_header_lib.so "target/rust-release.so"
  # shellcheck disable=SC2086
  $CARGO build $combo >/dev/null 2>&1
  cp target/debug/libupdate_frame_header_lib.so "target/rust-debug.so"

  echo "--- nm -D symbol diff (C - Rust) must be empty ---"
  diff <(nm -D --defined-only "$C_SO"       | awk '$2 ~ /^[A-Z]$/ {print $3}' | sort -u) \
       <(nm -D --defined-only target/rust-release.so | awk '$2 ~ /^[A-Z]$/ {print $3}' | sort -u) \
       > target/symdiff.txt || true
  if grep -q '^<' target/symdiff.txt; then
    echo "FAIL: C symbols missing from Rust:"; grep '^<' target/symdiff.txt; FAIL=1
  else
    echo "OK: no C symbol is missing from the Rust .so"
  fi

  for prof in release debug; do
    echo "--- tests against the $prof Rust cdylib ---"
    # shellcheck disable=SC2086
    if RUST_SO="$PWD/target/rust-$prof.so" timeout 600 \
         $CARGO test --release $combo 2>&1 | grep -E 'test result|FAILED|panicked' ; then :; fi
    # shellcheck disable=SC2086
    if ! RUST_SO="$PWD/target/rust-$prof.so" timeout 600 \
           $CARGO test --release $combo >/dev/null 2>&1; then
      echo "FAIL: suite failed for [$label] against the $prof cdylib"; FAIL=1
    fi
  done
done

echo
if [ "$FAIL" -eq 0 ]; then
  echo "==== ALL CONFIGURATIONS PASSED ===="
else
  echo "==== FAILURES PRESENT ===="; exit 1
fi
