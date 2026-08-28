#!/usr/bin/env bash
# Build the C reference library and the Rust cdylib, verify symbol parity, then
# run the full differential suite for every Cargo feature combination and for
# both Rust build profiles.
#
# Usage: scripts/run_all.sh [extra cargo test args...]
set -uo pipefail

CRATE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ROOT="$(cd "$CRATE_DIR/.." && pwd)"
C_DIR="$ROOT/c_src"
C_SO="$C_DIR/build/libdriver.so"
cd "$CRATE_DIR"

fail=0
say() { printf '\n\033[1m== %s\033[0m\n' "$*"; }

# ---------------------------------------------------------------- build the C
say "Building the C reference library"
mkdir -p "$C_DIR/build"
( cd "$C_DIR/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . ) || { echo "C build FAILED"; exit 1; }
test -f "$C_SO" || { echo "missing $C_SO"; exit 1; }

# ------------------------------------------------- enumerate feature combos
# Derived mechanically from Cargo.toml: the crate declares no [features] table,
# so the only combinations are the default one and the explicit no-default /
# all-features spellings of it.
FEATURE_LIST=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/{split($0,a,"=");gsub(/ /,"",a[1]); if (a[1] != "default") print a[1]}' Cargo.toml)
COMBOS=()
if [ -z "$FEATURE_LIST" ]; then
  COMBOS+=("")                       # default
  COMBOS+=("--no-default-features")
  COMBOS+=("--all-features")
else
  COMBOS+=("")
  COMBOS+=("--no-default-features")
  COMBOS+=("--all-features")
  for f in $FEATURE_LIST; do
    COMBOS+=("--no-default-features --features $f")
  done
fi

# ------------------------------------------------------------------ main loop
for profile in release debug; do
  if [ "$profile" = release ]; then
    BUILD_FLAG="--release"
    RUST_SO="$CRATE_DIR/target/release/libdriver.so"
    export DIFFTEST_UB_STRICT=1
  else
    BUILD_FLAG=""
    RUST_SO="$CRATE_DIR/target/debug/libdriver.so"
    # A debug cdylib carries rustc's debug_assertions (e.g. the "null pointer
    # dereference occurred" panic) which the C, where that is plain UB, has no
    # equivalent for. Relax only those UB-path comparisons.
    export DIFFTEST_UB_STRICT=0
  fi

  for combo in "${COMBOS[@]}"; do
    label="profile=$profile features='${combo:-<default>}'"

    say "Building Rust cdylib ($label)"
    # shellcheck disable=SC2086
    cargo build $BUILD_FLAG $combo || { echo "cargo build FAILED ($label)"; fail=1; continue; }

    say "Symbol parity ($label)"
    diff <(nm -D --defined-only "$C_SO"   | awk '{print $3}' | sort) \
         <(nm -D --defined-only "$RUST_SO" | awk '{print $3}' | sort)
    if [ $? -ne 0 ]; then
      echo "SYMBOL PARITY FAILED ($label)"; fail=1
    else
      echo "OK: $(nm -D --defined-only "$C_SO" | wc -l) symbols, 0 missing, 0 extra"
    fi

    say "Differential suite ($label)"
    # The test harness itself is always built in the dev profile so that
    # assertions unwind; only the library under test changes.
    # shellcheck disable=SC2086
    DRIVER_C_SO="$C_SO" DRIVER_RUST_SO="$RUST_SO" \
      cargo test $combo -- --test-threads=1 "$@" \
      || { echo "TESTS FAILED ($label)"; fail=1; }
  done
done

say "SUMMARY"
if [ "$fail" -eq 0 ]; then
  echo "ALL CONFIGURATIONS PASSED"
else
  echo "FAILURES PRESENT"
fi
exit "$fail"
