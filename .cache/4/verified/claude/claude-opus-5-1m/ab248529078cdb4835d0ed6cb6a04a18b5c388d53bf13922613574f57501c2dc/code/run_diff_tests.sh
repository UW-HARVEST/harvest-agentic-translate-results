#!/usr/bin/env bash
# Phase A–D driver: builds the C reference, enumerates every valid cargo feature
# combination, and runs the differential test suite for each of them, in both
# the dev and the release profile (release sets `panic = "abort"`).
#
# Usage: ./run_diff_tests.sh [extra cargo test args...]
set -u -o pipefail

cd "$(dirname "$0")"
CARGO_FLAGS="--offline"
FAILURES=0
step() { printf '\n\033[1m=== %s ===\033[0m\n' "$*"; }
fail() { printf '!!! FAILED: %s\n' "$*"; FAILURES=$((FAILURES + 1)); }

# ---------------------------------------------------------------------------
# 1. Build the C code exactly as c_src/CMakeLists.txt prescribes, plus the
#    shared-library flavour of the same translation unit (c_src is untouched).
# ---------------------------------------------------------------------------
step "Building the C reference (cmake + shared library)"
mkdir -p c_src/build
( cd c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON > cmake.log 2>&1 \
  && cmake --build . >> cmake.log 2>&1 ) || fail "cmake build"
mkdir -p target/cdiff
gcc -shared -fPIC -o target/cdiff/libcdriver.so c_src/src/main.c || fail "gcc -shared"
ls -l c_src/build/driver target/cdiff/libcdriver.so

# ---------------------------------------------------------------------------
# 2. Enumerate every valid feature combination from Cargo.toml (the power set of
#    the [features] table; an empty table yields exactly one combination).
# ---------------------------------------------------------------------------
step "Enumerating cargo feature combinations"
mapfile -t FEATURES < <(awk '
  /^\[features\]/ {inside=1; next}
  /^\[/           {inside=0}
  inside && /^[A-Za-z0-9_-]+[ \t]*=/ {sub(/[ \t]*=.*/, ""); print}
' Cargo.toml)
COMBOS=("")
if [ "${#FEATURES[@]}" -gt 0 ]; then
  n=${#FEATURES[@]}
  for ((mask = 1; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (( (mask >> i) & 1 )); then
        combo="${combo:+$combo,}${FEATURES[$i]}"
      fi
    done
    COMBOS+=("$combo")
  done
fi
printf 'features declared: %s\n' "${FEATURES[*]:-<none>}"
printf 'combination: --no-default-features --features "%s"\n' "${COMBOS[@]}"

# ---------------------------------------------------------------------------
# 3. cargo check / build / test for every combination and both profiles.
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  FEAT_ARGS=(--no-default-features)
  [ -n "$combo" ] && FEAT_ARGS+=(--features "$combo")
  label="features=[${combo:-<default>}]"

  for profile in dev release; do
    PROF_ARGS=()
    [ "$profile" = release ] && PROF_ARGS+=(--release)

    step "cargo check ($label, $profile)"
    cargo check $CARGO_FLAGS --all-targets "${FEAT_ARGS[@]}" "${PROF_ARGS[@]}" \
      || fail "cargo check $label $profile"

    step "cargo build lib+bin ($label, $profile)"
    cargo build $CARGO_FLAGS --lib --bins "${FEAT_ARGS[@]}" "${PROF_ARGS[@]}" \
      || fail "cargo build $label $profile"

    # Symbol parity between the C .so and the freshly built Rust cdylib.
    step "nm -D symbol diff ($label, $profile)"
    dir=target/debug
    [ "$profile" = release ] && dir=target/release
    nm -D --defined-only target/cdiff/libcdriver.so | awk '{print $NF}' | sort -u > target/cdiff/c.syms
    nm -D --defined-only "$dir/libdriver.so"        | awk '{print $NF}' | sort -u > target/cdiff/rust.syms
    echo "C   : $(tr '\n' ' ' < target/cdiff/c.syms)"
    echo "Rust: $(tr '\n' ' ' < target/cdiff/rust.syms)"
    missing=$(comm -23 target/cdiff/c.syms target/cdiff/rust.syms)
    if [ -n "$missing" ]; then
      fail "symbols missing from the Rust .so ($label, $profile): $missing"
    else
      echo "no missing symbols"
    fi

    step "cargo test ($label, $profile)"
    RUST_SO="$PWD/$dir/libdriver.so" RUST_EXE="$PWD/$dir/driver" \
      cargo test $CARGO_FLAGS "${FEAT_ARGS[@]}" "${PROF_ARGS[@]}" -- --test-threads=4 "$@" \
      || fail "cargo test $label $profile"
  done
done

step "SUMMARY"
if [ "$FAILURES" -eq 0 ]; then
  echo "ALL CHECKS PASSED"
else
  echo "$FAILURES step(s) failed"
fi
exit "$FAILURES"
