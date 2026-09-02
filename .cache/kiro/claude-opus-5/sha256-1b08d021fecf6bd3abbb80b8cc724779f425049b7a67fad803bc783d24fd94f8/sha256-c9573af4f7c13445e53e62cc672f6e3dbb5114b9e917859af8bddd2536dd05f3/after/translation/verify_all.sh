#!/usr/bin/env bash
# Phase D driver: verify every feature combination.
#
# Extracts the feature list from Cargo.toml, enumerates the combinations to
# test, and for each one runs `cargo check`, `cargo build`, the full test suite,
# and the `nm -D` symbol diff against the C .so. Nothing is repeated by hand.
#
# Usage: ./verify_all.sh [--quick]
set -uo pipefail

cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"
TIMEOUT=${TIMEOUT:-600}
FAIL=0

say() { printf '\n=== %s ===\n' "$*"; }

# --------------------------------------------------------------------------
# 0. The C reference .so must exist.
# --------------------------------------------------------------------------
say "C reference library"
if ! ls "$ROOT"/c_src/build/lib*.so >/dev/null 2>&1; then
  echo "building the C shared library"
  ( cd "$ROOT/c_src" && mkdir -p build && cd build \
      && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
      && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
fi
C_SO="$(ls "$ROOT"/c_src/build/lib*.so | head -1)"
echo "C .so: $C_SO"
nm -D --defined-only "$C_SO" | awk '{print $3}' | sort -u > /tmp/verify_c_syms.txt
echo "exports: $(wc -l < /tmp/verify_c_syms.txt)"

# --------------------------------------------------------------------------
# 1. Enumerate feature combinations from Cargo.toml.
# --------------------------------------------------------------------------
say "feature enumeration"
FEATURES=$(awk '
  /^\[features\]/      { inf=1; next }
  /^\[/                { inf=0 }
  inf && /^[a-zA-Z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
  }' Cargo.toml | sort -u)

if [ -z "$FEATURES" ]; then
  echo "Cargo.toml declares no [features]; the only configuration is the default."
  # Still exercise the flags that change feature resolution, so the claim is checked
  # rather than assumed.
  COMBOS=("" "--no-default-features" "--all-features")
else
  echo "features: $FEATURES"
  COMBOS=("" "--no-default-features" "--all-features")
  for f in $FEATURES; do
    COMBOS+=("--no-default-features --features $f")
    COMBOS+=("--features $f")
  done
  # every pair
  for a in $FEATURES; do
    for b in $FEATURES; do
      [ "$a" \< "$b" ] && COMBOS+=("--no-default-features --features $a,$b")
    done
  done
  # all at once, explicitly
  ALL=$(echo "$FEATURES" | paste -sd,)
  COMBOS+=("--no-default-features --features $ALL")
fi
echo "${#COMBOS[@]} combination(s) to verify"

# --------------------------------------------------------------------------
# 2. For each combination: check, build, symbol-diff, test.
# --------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  label="${combo:-<default>}"
  say "combination: $label"

  if ! timeout "$TIMEOUT" cargo check --release $combo >/tmp/verify_check.log 2>&1; then
    echo "FAIL cargo check"; tail -25 /tmp/verify_check.log; FAIL=1; continue
  fi
  echo "ok  cargo check"

  if ! timeout "$TIMEOUT" cargo build --release $combo >/tmp/verify_build.log 2>&1; then
    echo "FAIL cargo build"; tail -25 /tmp/verify_build.log; FAIL=1; continue
  fi
  echo "ok  cargo build"

  R_SO=target/release/libspec_ray_lib.so
  nm -D --defined-only "$R_SO" | awk '{print $3}' | sort -u > /tmp/verify_r_syms.txt
  missing=$(comm -23 /tmp/verify_c_syms.txt /tmp/verify_r_syms.txt)
  extra=$(comm -13 /tmp/verify_c_syms.txt /tmp/verify_r_syms.txt)
  if [ -n "$missing" ] || [ -n "$extra" ]; then
    echo "FAIL symbol diff"
    [ -n "$missing" ] && echo "  missing from Rust: $missing"
    [ -n "$extra" ]   && echo "  extra in Rust:     $extra"
    FAIL=1
  else
    echo "ok  symbol parity ($(wc -l < /tmp/verify_r_syms.txt) exports, 0 diff)"
  fi

  if ! timeout "$TIMEOUT" cargo test --release --no-fail-fast $combo >/tmp/verify_test.log 2>&1; then
    echo "FAIL cargo test"
    grep -E "^(test .*FAILED|failures:|thread)" /tmp/verify_test.log | head -30
    grep -A 12 "^---- " /tmp/verify_test.log | head -60
    FAIL=1
  else
    echo "ok  cargo test  ($(grep -c '^test .* ok$' /tmp/verify_test.log) tests passed)"
  fi
done

# --------------------------------------------------------------------------
# 3. Also verify the debug profile (different codegen, same .so contract).
# --------------------------------------------------------------------------
say "debug profile"
if timeout "$TIMEOUT" cargo test --no-fail-fast >/tmp/verify_debug.log 2>&1; then
  echo "ok  cargo test (debug), $(grep -c '^test .* ok$' /tmp/verify_debug.log) tests passed"
else
  echo "FAIL cargo test (debug)"
  grep -E "^(test .*FAILED|failures:|thread)" /tmp/verify_debug.log | head -30
  grep -A 12 "^---- " /tmp/verify_debug.log | head -60
  FAIL=1
fi

say "RESULT"
if [ "$FAIL" -eq 0 ]; then
  echo "ALL COMBINATIONS PASSED"
else
  echo "THERE WERE FAILURES"
fi
exit "$FAIL"
