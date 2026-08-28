#!/usr/bin/env bash
# Full verification sweep: builds the C reference object and the Rust cdylib,
# then runs every differential test suite for
#   * every Cargo feature combination declared in Cargo.toml (the power set,
#     plus the default and --no-default-features cases), and
#   * every build profile (dev and release), because the shipped artifact is the
#     release one while `cargo test` builds the dev one.
#
# Usage: ./check_all_features.sh        (run from the `translation` directory)
set -uo pipefail

CRATE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(dirname "$CRATE_DIR")"
CARGO_FLAGS="--offline"   # the crates.io mirror is not reachable from here
FAILED=0

say() { printf '\n\033[1m== %s\033[0m\n' "$*"; }
fail() { printf '\033[31mFAIL: %s\033[0m\n' "$*"; FAILED=$((FAILED + 1)); }

# ---------------------------------------------------------------- C reference
say "building the C reference shared object"
mkdir -p "$ROOT/c_src/build"
(
  cd "$ROOT/c_src/build" &&
    cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null &&
    cmake --build . >/dev/null
) || { fail "C build"; exit 1; }
C_SO="$(find "$ROOT/c_src/build" -maxdepth 1 -name '*.so' | sort | head -1)"
[ -n "$C_SO" ] || { fail "no C .so produced"; exit 1; }
echo "C .so: $C_SO"

# ------------------------------------------------------- feature combinations
# Every key of the [features] table, in declaration order.
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inside = 1; next }
    /^\[/           { inside = 0 }
    inside && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      sub(/[[:space:]]*=.*/, ""); print
    }
  ' "$CRATE_DIR/Cargo.toml"
)

COMBOS=()
COMBOS+=("")                          # default features
COMBOS+=("--no-default-features")     # nothing at all
if [ "${#FEATURES[@]}" -gt 0 ]; then
  echo "features found: ${FEATURES[*]}"
  n=${#FEATURES[@]}
  for ((mask = 1; mask < (1 << n); mask++)); do
    set=()
    for ((bit = 0; bit < n; bit++)); do
      if (((mask >> bit) & 1)); then set+=("${FEATURES[$bit]}"); fi
    done
    IFS=,
    COMBOS+=("--no-default-features --features ${set[*]}")
    unset IFS
  done
  COMBOS+=("--all-features")
else
  echo "features found: (none — Cargo.toml declares no [features] table)"
fi

# ------------------------------------------------------------------ the sweep
for combo in "${COMBOS[@]}"; do
  for profile in dev release; do
    label="features='${combo:-<default>}' profile=$profile"
    say "$label"

    if [ "$profile" = release ]; then
      build_flag="--release"
      so="$CRATE_DIR/target/release/libhsv_to_rgb_lib.so"
    else
      build_flag=""
      so="$CRATE_DIR/target/debug/libhsv_to_rgb_lib.so"
    fi

    # shellcheck disable=SC2086
    if ! (cd "$CRATE_DIR" && cargo check $CARGO_FLAGS $build_flag $combo --all-targets \
          >/dev/null 2>&1); then
      fail "cargo check ($label)"
      continue
    fi
    # shellcheck disable=SC2086
    if ! (cd "$CRATE_DIR" && cargo build $CARGO_FLAGS $build_flag $combo >/dev/null 2>&1); then
      fail "cargo build ($label)"
      continue
    fi
    [ -f "$so" ] || { fail "no Rust .so at $so ($label)"; continue; }

    # nm parity, recomputed outside the test harness as well
    c_syms=$(nm -D --defined-only "$C_SO" | awk '$2 ~ /^[TDBRWV]$/ {print $3}' | sort -u)
    r_syms=$(nm -D --defined-only "$so" | awk '$2 ~ /^[TDBRWV]$/ {print $3}' | sort -u)
    missing=$(comm -23 <(echo "$c_syms") <(echo "$r_syms"))
    if [ -n "$missing" ]; then
      fail "symbols missing from the Rust .so ($label): $(echo "$missing" | tr '\n' ' ')"
    else
      echo "symbol parity: OK ($(echo "$c_syms" | wc -l) exported C symbol(s), 0 missing)"
    fi

    # the differential suites, run against this exact .so
    # shellcheck disable=SC2086
    if ! (cd "$CRATE_DIR" &&
          HSV_C_SO="$C_SO" HSV_RUST_SO="$so" \
          timeout 600 cargo test $CARGO_FLAGS $combo --tests -- --test-threads=8); then
      fail "cargo test ($label)"
    fi
  done
done

say "summary"
if [ "$FAILED" -eq 0 ]; then
  echo "ALL COMBINATIONS PASSED"
else
  echo "$FAILED failure(s)"
fi
exit "$FAILED"
