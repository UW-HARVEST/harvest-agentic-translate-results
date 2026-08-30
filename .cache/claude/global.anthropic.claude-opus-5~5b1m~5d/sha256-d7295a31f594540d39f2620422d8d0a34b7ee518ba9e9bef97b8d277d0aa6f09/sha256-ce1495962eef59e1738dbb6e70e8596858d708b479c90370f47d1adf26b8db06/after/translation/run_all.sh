#!/usr/bin/env bash
# Full verification sweep: build the C ground-truth library, then run the
# differential test suite for every feature combination x profile.
#
# `cargo build` is always run BEFORE `cargo test` because cargo does not
# reliably rebuild/uplift a `crate-type = ["cdylib"]` library for a test target
# that does not link it -- the tests would otherwise load a stale `.so`. The
# harness also asserts freshness (see `assert_not_stale`), so a missed rebuild
# fails loudly instead of passing vacuously.
set -uo pipefail

cd "$(dirname "$0")"
CRATE_DIR="$PWD"
C_SRC="$(cd .. && pwd)/c_src"

CARGO_FLAGS=("--offline")   # this sandbox has no crates.io access
TIMEOUT="${TIMEOUT:-600}"

fail=0
step() { printf '\n\033[1m== %s\033[0m\n' "$*"; }

# --------------------------------------------------------------------------
# 1. Build the C shared library (ground truth)
# --------------------------------------------------------------------------
step "Building C shared library"
mkdir -p "$C_SRC/build"
( cd "$C_SRC/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . ) || { echo "C build FAILED"; exit 1; }
ls -l "$C_SRC/build/libdriver.so"

# --------------------------------------------------------------------------
# 2. Enumerate feature combinations from Cargo.toml
# --------------------------------------------------------------------------
FEATURES=$(awk '
  /^\[features\]/      { inf=1; next }
  /^\[/                { inf=0 }
  inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ { gsub(/[[:space:]]*=.*/,""); if ($0 != "default") print }
' Cargo.toml)

# Build the list of `cargo` feature argument sets to test.
COMBOS=()
COMBOS+=("")                          # default features
COMBOS+=("--no-default-features")     # nothing enabled
if [ -n "$FEATURES" ]; then
  # every single feature on its own, plus all of them together
  while read -r f; do
    [ -z "$f" ] && continue
    COMBOS+=("--no-default-features --features $f")
  done <<< "$FEATURES"
  ALL=$(echo "$FEATURES" | paste -sd, -)
  COMBOS+=("--no-default-features --features $ALL")
  COMBOS+=("--all-features")
fi

step "Feature combinations to verify (${#COMBOS[@]})"
for c in "${COMBOS[@]}"; do echo "  cargo test ${c:-<default features>}"; done

# --------------------------------------------------------------------------
# 3. cargo check / build / test for each combination x profile
# --------------------------------------------------------------------------
for profile_flag in "--release" ""; do
  pname=${profile_flag:-debug}
  for combo in "${COMBOS[@]}"; do
    label="profile=${pname#--} features=${combo:-<default>}"
    step "$label"

    # shellcheck disable=SC2086
    if ! timeout "$TIMEOUT" cargo check "${CARGO_FLAGS[@]}" $profile_flag $combo \
         --all-targets 2>&1 | tail -3; then
      echo "  cargo check FAILED for $label"; fail=1; continue
    fi

    # shellcheck disable=SC2086
    if ! timeout "$TIMEOUT" cargo build "${CARGO_FLAGS[@]}" $profile_flag $combo 2>&1 | tail -2; then
      echo "  cargo build FAILED for $label"; fail=1; continue
    fi

    # shellcheck disable=SC2086
    timeout "$TIMEOUT" cargo test "${CARGO_FLAGS[@]}" $profile_flag $combo 2>&1 | tail -45
    # shellcheck disable=SC2086
    if [ "${PIPESTATUS[0]}" -ne 0 ]; then
      echo "  TESTS FAILED for $label"; fail=1
    fi
  done
done

# --------------------------------------------------------------------------
# 4. Symbol parity report
# --------------------------------------------------------------------------
step "nm -D symbol diff (C vs Rust)"
c_syms=$(nm -D --defined-only --format=posix "$C_SRC/build/libdriver.so" \
         | awk '$2 ~ /^[TtDBRWVi]$/ {print $1}' | sort -u)
r_syms=$(nm -D --defined-only --format=posix "$CRATE_DIR/target/release/libdriver.so" \
         | awk '$2 ~ /^[TtDBRWVi]$/ {print $1}' | sort -u)
echo "C   exports: $(echo "$c_syms" | tr '\n' ' ')"
echo "Rust exports: $(echo "$r_syms" | tr '\n' ' ')"
missing=$(comm -23 <(echo "$c_syms") <(echo "$r_syms"))
if [ -n "$missing" ]; then
  echo "MISSING from Rust .so:"; echo "$missing"; fail=1
else
  echo "Symbol diff is EMPTY (0 missing)."
fi

step "SUMMARY"
if [ "$fail" -eq 0 ]; then
  echo "ALL CHECKS PASSED"
else
  echo "FAILURES PRESENT"
fi
exit "$fail"
