#!/usr/bin/env bash
# Full differential verification driver.
#
#   ./run_tests.sh
#
# `cargo test` does NOT build a cdylib-only lib target, so the Rust `.so` must
# be produced explicitly with `cargo build` before the tests run. The harness
# also refuses to run against a `.so` older than `src/lib.rs`.
set -uo pipefail

cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"
FAIL=0

step() { printf '\n\033[1m== %s\033[0m\n' "$*"; }
ok()   { printf '   \033[32mPASS\033[0m %s\n' "$*"; }
bad()  { printf '   \033[31mFAIL\033[0m %s\n' "$*"; FAIL=1; }

# ---------------------------------------------------------------------------
step "1/5  build the C shared library"
( cd "$ROOT/c_src" && mkdir -p build && cd build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null ) && ok "c_src" || bad "c_src build"
C_SO="$(find "$ROOT/c_src/build" -maxdepth 1 -name '*.so' | sort | head -1)"
echo "   C .so: $C_SO"

# ---------------------------------------------------------------------------
# Enumerate the feature combinations declared in Cargo.toml. This crate has no
# [features] section, so the only combination is the default (empty) one — but
# the loop is derived mechanically so it stays correct if features are added.
step "2/5  enumerate cargo feature combinations"
mapfile -t FEATURES < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /^[A-Za-z0-9_-]+[[:space:]]*=/ {print $1}' Cargo.toml
)
echo "   declared features: ${#FEATURES[@]} (${FEATURES[*]:-none})"

COMBOS=()
if [ "${#FEATURES[@]}" -eq 0 ]; then
  COMBOS+=("")                       # default build
  COMBOS+=("--no-default-features")  # provably identical here, but checked
else
  n=${#FEATURES[@]}
  for ((mask=0; mask<(1<<n); mask++)); do
    sel=""
    for ((i=0; i<n; i++)); do
      if (( mask & (1<<i) )); then sel="${sel:+$sel,}${FEATURES[$i]}"; fi
    done
    if [ -z "$sel" ]; then COMBOS+=("--no-default-features")
    else COMBOS+=("--no-default-features --features $sel"); fi
  done
  COMBOS+=("")                       # plus the default feature set
fi
printf '   %s combination(s)\n' "${#COMBOS[@]}"

# ---------------------------------------------------------------------------
step "3/5  cargo check for every combination"
for c in "${COMBOS[@]}"; do
  if timeout 300 cargo check --all-targets $c >/dev/null 2>&1; then
    ok "cargo check ${c:-<default>}"
  else
    bad "cargo check ${c:-<default>}"
  fi
done

# ---------------------------------------------------------------------------
step "4/5  build the cdylib + run the differential suite (dev and release)"
for prof in dev release; do
  if [ "$prof" = release ]; then BUILD="--release"; TEST="--release"; else BUILD=""; TEST=""; fi
  for c in "${COMBOS[@]}"; do
    label="${prof} ${c:-<default>}"
    if ! timeout 600 cargo build $BUILD $c >/dev/null 2>&1; then
      bad "cargo build $label"; continue
    fi
    if timeout 600 cargo test $TEST $c -- --test-threads=4 >/tmp/ima_test_$$.log 2>&1; then
      ok "cargo test $label ($(grep -c '^test .* ok$' /tmp/ima_test_$$.log) tests)"
    else
      bad "cargo test $label"
      tail -40 /tmp/ima_test_$$.log
    fi
  done
done
rm -f /tmp/ima_test_$$.log

# ---------------------------------------------------------------------------
step "5/5  nm -D symbol diff (must be empty)"
./check_symbols.sh && ok "symbol parity" || bad "symbol parity"

# ---------------------------------------------------------------------------
if [ "$FAIL" -eq 0 ]; then
  printf '\n\033[32mALL CHECKS PASSED\033[0m\n'
else
  printf '\n\033[31mSOME CHECKS FAILED\033[0m\n'
fi
exit "$FAIL"
