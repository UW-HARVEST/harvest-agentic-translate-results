#!/usr/bin/env bash
# Phase D completion-gate driver.
#
# Rebuilds the C .so and the Rust cdylib, then runs the whole differential suite
# once per (feature combination x rust build profile). Feature combinations are
# extracted from Cargo.toml rather than hardcoded.
set -uo pipefail

cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"
FAIL=0

run() {
  local label="$1"; shift
  echo "=== $label ==="
  if timeout 600 "$@" > /tmp/dt.log 2>&1; then
    grep -E '^test result:' /tmp/dt.log | sed 's/^/    /'
  else
    echo "    FAILED"
    tail -n 40 /tmp/dt.log | sed 's/^/    /'
    FAIL=1
  fi
}

echo "### building C shared library"
( mkdir -p "$ROOT/c_src/build" && cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build failed"; exit 1; }
C_SO=$(ls "$ROOT"/c_src/build/lib*.so)
echo "    $C_SO"

# ---- enumerate feature combinations from Cargo.toml -------------------------
FEATURES=$(cargo metadata --no-deps --format-version 1 2>/dev/null \
  | python3 -c 'import json,sys; print(" ".join(sorted(json.load(sys.stdin)["packages"][0]["features"])))')
echo "### declared features: [${FEATURES:-none}]"

COMBOS=()
if [ -z "$FEATURES" ]; then
  # No [features] table: the only configurations are the implicit default and
  # --no-default-features (identical here, but both are exercised).
  COMBOS+=("default:")
  COMBOS+=("no-default:--no-default-features")
else
  COMBOS+=("default:")
  COMBOS+=("no-default:--no-default-features")
  for f in $FEATURES; do
    COMBOS+=("$f:--no-default-features --features $f")
  done
  ALL=$(echo "$FEATURES" | tr ' ' ',')
  COMBOS+=("all:--no-default-features --features $ALL")
fi

for combo in "${COMBOS[@]}"; do
  name="${combo%%:*}"
  flags="${combo#*:}"
  echo
  echo "########## feature combination: $name  [$flags]"
  # shellcheck disable=SC2086
  run "cargo check ($name)" cargo check $flags

  for profile in debug release; do
    if [ "$profile" = release ]; then
      # shellcheck disable=SC2086
      timeout 600 cargo build --release $flags >/dev/null 2>&1 || { echo "release build failed"; FAIL=1; continue; }
      SO="$ROOT/translation/target/release/libintput_lib.so"
    else
      # shellcheck disable=SC2086
      timeout 600 cargo build $flags >/dev/null 2>&1 || { echo "debug build failed"; FAIL=1; continue; }
      SO="$ROOT/translation/target/debug/libintput_lib.so"
    fi
    # shellcheck disable=SC2086
    DIFFTEST_RUST_SO="$SO" run "tests: $name / $profile rust .so" cargo test $flags
  done
done

echo
if [ "$FAIL" = 0 ]; then
  echo "ALL COMBINATIONS PASSED"
else
  echo "FAILURES PRESENT"
fi
exit $FAIL
