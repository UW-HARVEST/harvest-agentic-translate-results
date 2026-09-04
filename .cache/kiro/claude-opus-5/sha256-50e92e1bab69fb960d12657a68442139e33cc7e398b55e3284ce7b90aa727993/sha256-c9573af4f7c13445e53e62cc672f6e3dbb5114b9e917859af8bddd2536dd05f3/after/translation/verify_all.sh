#!/bin/bash
# Phase D driver: rebuild both shared objects, diff their exported symbols, and
# run the whole differential test suite under EVERY cargo feature combination
# declared in Cargo.toml.
set -uo pipefail
cd "$(dirname "$0")"
ROOT=$(pwd)
C_BUILD=../c_src/build
C_SO=$C_BUILD/libpcre2.so
R_SO=$ROOT/target/release/libpcre2.so
FAIL=0

echo "=== 1. build the C shared library ==========================================="
( cd ../c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && timeout 600 cmake --build . -j8 >/dev/null ) || { echo "C BUILD FAILED"; exit 1; }
echo "ok: $C_SO"

echo
echo "=== 2. enumerate feature combinations ======================================="
# Every feature name declared in [features], plus the default and no-default
# builds. With no [features] table this yields exactly one combination.
FEATURES=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /^[a-zA-Z0-9_-]+[ ]*=/{print $1}' Cargo.toml)
COMBOS=("default")
if [ -n "$FEATURES" ]; then
  COMBOS+=("none")
  for f in $FEATURES; do COMBOS+=("$f"); done
  ALL=$(echo "$FEATURES" | paste -sd,)
  COMBOS+=("$ALL")
fi
echo "features declared: ${FEATURES:-<none>}"
echo "combinations to verify: ${COMBOS[*]}"

for combo in "${COMBOS[@]}"; do
  echo
  echo "############################################################"
  echo "### feature combination: $combo"
  echo "############################################################"
  case "$combo" in
    default) FARGS=() ;;
    none)    FARGS=(--no-default-features) ;;
    *)       FARGS=(--no-default-features --features "$combo") ;;
  esac

  echo "--- cargo check"
  timeout 600 cargo check --release "${FARGS[@]}" 2>&1 | tail -3 || FAIL=1

  echo "--- cargo build (cdylib)"
  timeout 600 cargo build --release "${FARGS[@]}" 2>&1 | tail -3 || { FAIL=1; continue; }

  echo "--- symbol diff (nm -D)"
  nm -D --defined-only "$C_SO" | awk '$2 ~ /^[TDRBW]$/ {print $3}' | sort -u > /tmp/c_syms.txt
  nm -D --defined-only "$R_SO" | awk '$2 ~ /^[TDRBW]$/ {print $3}' | sort -u > /tmp/r_syms.txt
  MISSING=$(comm -23 /tmp/c_syms.txt /tmp/r_syms.txt)
  EXTRA=$(comm -13 /tmp/c_syms.txt /tmp/r_syms.txt)
  echo "C exports: $(wc -l < /tmp/c_syms.txt)  Rust exports: $(wc -l < /tmp/r_syms.txt)"
  if [ -n "$MISSING" ]; then
    echo "MISSING from Rust:"; echo "$MISSING"; FAIL=1
  else
    echo "missing from Rust: 0"
  fi
  if [ -n "$EXTRA" ]; then echo "extra in Rust:"; echo "$EXTRA"; fi
  UNDEF=$(nm -D --undefined-only "$R_SO" | awk '{print $2}' \
          | grep -v '@GLIBC\|@GCC\|^_ITM_\|^__gmon_start__\|^_Unwind_' || true)
  if [ -n "$UNDEF" ]; then
    echo "NOTE: unresolved non-libc symbols in the Rust .so:"; echo "$UNDEF"
  else
    echo "unresolved non-libc symbols: 0"
  fi

  echo "--- differential test suite"
  timeout 600 cargo test --release "${FARGS[@]}" -- --test-threads=1 2>&1 \
    | grep -E '^test |^running|test result|DIVERGENCE|panicked' || FAIL=1
  if [ "${PIPESTATUS[0]}" != "0" ]; then FAIL=1; fi
done

echo
if [ "$FAIL" = "0" ]; then
  echo "ALL FEATURE COMBINATIONS PASSED"
else
  echo "FAILURES DETECTED"
fi
exit $FAIL
