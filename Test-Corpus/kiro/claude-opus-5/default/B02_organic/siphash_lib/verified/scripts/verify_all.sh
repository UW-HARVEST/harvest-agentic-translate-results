#!/usr/bin/env bash
# Build the C .so, the Rust .so, the driver example, then run the differential
# tests across every Cargo feature combination.
#
# `cargo test` does NOT rebuild a cdylib-only lib target, so the explicit
# `cargo build` before each `cargo test` is mandatory, not a convenience.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

echo "=== building C shared library ==="
mkdir -p c_src/build
(cd c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && timeout 600 cmake --build . >/dev/null)
C_SO="$(ls c_src/build/lib*.so | head -1)"
echo "  -> $C_SO"

cd translation

# Enumerate feature combinations from Cargo.toml. If the crate declares no
# [features] section, the only combination is the default one.
mapfile -t FEATURES < <(awk '
  /^\[features\]/ {inf=1; next}
  /^\[/ {inf=0}
  inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {gsub(/[[:space:]]*=.*/,""); print}
' Cargo.toml)

declare -a COMBOS=()
if [ "${#FEATURES[@]}" -eq 0 ]; then
  COMBOS+=("__default__")
else
  COMBOS+=("__default__")
  COMBOS+=("__none__")
  n=${#FEATURES[@]}
  for ((mask=1; mask<(1<<n); mask++)); do
    combo=""
    for ((i=0; i<n; i++)); do
      if (( mask & (1<<i) )); then combo="${combo:+$combo,}${FEATURES[$i]}"; fi
    done
    COMBOS+=("$combo")
  done
fi

echo "=== feature combinations to verify: ${#COMBOS[@]} ==="
printf '  %s\n' "${COMBOS[@]}"

status=0
for combo in "${COMBOS[@]}"; do
  case "$combo" in
    __default__) FLAGS=() ; label="(default features)" ;;
    __none__)    FLAGS=(--no-default-features) ; label="(no default features)" ;;
    *)           FLAGS=(--no-default-features --features "$combo") ; label="--features $combo" ;;
  esac

  echo
  echo "############ $label ############"
  if ! timeout 600 cargo build --release --lib --example siphash_driver "${FLAGS[@]}" 2>&1 | tail -3; then
    echo "BUILD FAILED for $label"; status=1; continue
  fi
  echo "--- nm -D symbol parity ---"
  diff <(nm -D --defined-only "../$C_SO" | awk '{print $3}' | sort) \
       <(nm -D --defined-only target/release/libsiphash_lib.so | awk '{print $3}' | sort) \
    && echo "  symbol sets identical" \
    || { echo "  SYMBOL DIFF for $label"; status=1; }

  if ! timeout 600 cargo test --release "${FLAGS[@]}" 2>&1 | grep -E "test result|^error|FAILED|panicked"; then
    echo "TESTS FAILED for $label"; status=1
  fi

  # Re-run under the dev profile as well: it enables Rust's arithmetic overflow
  # checks and uses panic=unwind, i.e. genuinely different codegen.
  echo "--- dev profile (overflow checks on) ---"
  if ! timeout 600 cargo build --lib --example siphash_driver "${FLAGS[@]}" >/dev/null 2>&1; then
    echo "DEV BUILD FAILED for $label"; status=1; continue
  fi
  if ! timeout 600 cargo test "${FLAGS[@]}" 2>&1 | grep -E "test result|^error|FAILED|panicked"; then
    echo "DEV TESTS FAILED for $label"; status=1
  fi
done

echo
if [ "$status" -eq 0 ]; then echo "ALL FEATURE COMBINATIONS PASSED"; else echo "FAILURES PRESENT"; fi
exit "$status"
