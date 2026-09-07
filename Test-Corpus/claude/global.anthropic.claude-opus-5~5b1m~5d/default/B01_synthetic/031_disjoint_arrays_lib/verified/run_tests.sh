#!/usr/bin/env bash
# Differential test driver: C `.so` vs Rust `.so`, loaded side by side via libloading.
#
#   ./run_tests.sh            # every feature combo x debug + release Rust .so
#   ./run_tests.sh quick      # default features, debug .so only
#
# NOTE: the `driver` tests capture file descriptor 1 to compare `printf` output,
# so they MUST run single-threaded -- libtest's own progress output also goes to
# fd 1 and would otherwise be interleaved into the capture.
set -uo pipefail

cd "$(dirname "$0")"
ROOT="$(pwd)"
C_BUILD="$ROOT/../c_src/build"
CARGO="cargo"
OFFLINE="--offline"
FAILED=0

say() { printf '\n\033[1m== %s\033[0m\n' "$*"; }

# --- 1. Build the C shared library -----------------------------------------
say "Building C shared library"
mkdir -p "$C_BUILD"
( cd "$C_BUILD" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
ls -l "$C_BUILD/libdriver.so" || exit 1

# --- 2. Enumerate feature combinations -------------------------------------
# Read the [features] table out of Cargo.toml. This crate has none, so the list
# collapses to the default build plus an explicit --no-default-features run.
FEATURES=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/ {split($0,a,"="); gsub(/ /,"",a[1]); if (a[1] != "default") print a[1]}' Cargo.toml)
if [ -n "$FEATURES" ]; then
  echo "declared features: $FEATURES"
else
  echo "no [features] table in Cargo.toml -> single configuration"
fi

COMBOS=("")                       # default features
COMBOS+=("--no-default-features") # explicit minimal build
for f in $FEATURES; do
  COMBOS+=("--no-default-features --features $f")
  COMBOS+=("--features $f")
done
if [ -n "$FEATURES" ]; then
  ALL=$(echo "$FEATURES" | tr '\n' ',' | sed 's/,$//')
  COMBOS+=("--no-default-features --features $ALL")
fi

if [ "${1:-}" = "quick" ]; then
  COMBOS=("")
  PROFILES=(debug)
else
  PROFILES=(debug release)
fi

# --- 3. Run every combo against both the debug and the release Rust .so -----
for combo in "${COMBOS[@]}"; do
  for prof in "${PROFILES[@]}"; do
    label="features='${combo:-<default>}' rust-so=$prof"
    say "$label"

    if [ "$prof" = "release" ]; then
      BUILD_FLAGS="--release"
      SO="$ROOT/target/release/libdriver.so"
    else
      BUILD_FLAGS=""
      SO="$ROOT/target/debug/libdriver.so"
    fi

    # `cargo test` does not emit cdylib artifacts, so build them explicitly.
    # shellcheck disable=SC2086
    $CARGO build $OFFLINE $BUILD_FLAGS $combo >/dev/null 2>&1 \
      || { echo "cargo build FAILED for $label"; FAILED=1; continue; }
    [ -f "$SO" ] || { echo "missing $SO"; FAILED=1; continue; }

    # shellcheck disable=SC2086
    DRIVER_RUST_SO="$SO" DRIVER_C_SO="$C_BUILD/libdriver.so" \
      timeout 600 $CARGO test $OFFLINE $combo -- --test-threads=1 2>&1 \
      | grep -E 'Running|test result|FAILED|panicked|^error|assertion' \
      | sed 's/^/    /'
    rc=${PIPESTATUS[0]}
    if [ "$rc" -ne 0 ]; then
      echo "    >>> TESTS FAILED ($label)"
      FAILED=1
    else
      echo "    >>> all tests passed ($label)"
    fi
  done
done

say "SUMMARY"
if [ "$FAILED" -eq 0 ]; then
  echo "ALL CONFIGURATIONS PASSED"
else
  echo "SOME CONFIGURATIONS FAILED"
fi
exit "$FAILED"
