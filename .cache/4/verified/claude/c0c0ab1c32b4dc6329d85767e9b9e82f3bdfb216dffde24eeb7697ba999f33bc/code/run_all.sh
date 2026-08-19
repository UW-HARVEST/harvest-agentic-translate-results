#!/usr/bin/env bash
# Full differential verification run.
#
#   ./run_all.sh [extra cargo test args...]
#
# 1. builds the C program *and* the C shared object with the flags CMake uses
# 2. enumerates every Cargo feature combination and `cargo check`s each one
# 3. builds the Rust cdylib + bin + so_runner example (cargo test does NOT build
#    a cdylib, so this has to happen up front)
# 4. runs the whole differential suite for every feature combination
set -euo pipefail

cd "$(dirname "$0")"
ROOT=$PWD
CARGO_FLAGS=(--offline)

echo "=== [1/5] building the C reference ==================================="
mkdir -p c_src/build
(
  cd c_src/build
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null
  cmake --build . >/dev/null
  # Same translation unit, same flags (CMake uses no -O and no -D), as a .so.
  gcc -fPIC -shared -o libcdriver.so ../src/main.c
)
ls -l c_src/build/driver c_src/build/libcdriver.so

echo
echo "=== [2/5] feature combinations ======================================="
# Every combination of the crate's optional features (the powerset).  This crate
# declares no [features], so the only combination is the empty one.
mapfile -t FEATURES < <(awk '
  /^\[features\]/ {inside=1; next}
  /^\[/           {inside=0}
  inside && /^[A-Za-z0-9_-]+[[:space:]]*=/ {print $1}
' Cargo.toml)
echo "declared features: ${#FEATURES[@]} (${FEATURES[*]:-none})"

COMBOS=("")
for f in "${FEATURES[@]}"; do
  for existing in "${COMBOS[@]}"; do
    if [ -z "$existing" ]; then COMBOS+=("$f"); else COMBOS+=("$existing,$f"); fi
  done
done
echo "combinations to verify: ${#COMBOS[@]}"

for combo in "${COMBOS[@]}"; do
  echo "--- cargo check --no-default-features --features '$combo'"
  cargo check "${CARGO_FLAGS[@]}" --all-targets --no-default-features --features "$combo"
done
echo "--- cargo check (default features)"
cargo check "${CARGO_FLAGS[@]}" --all-targets

echo
echo "=== [3/5] building the Rust cdylib / bin / test helper ==============="
for combo in "${COMBOS[@]}"; do
  cargo build "${CARGO_FLAGS[@]}" --no-default-features --features "$combo" --lib --bins --examples
done
cargo build "${CARGO_FLAGS[@]}" --lib --bins --examples
ls -l target/debug/libdriver.so target/debug/driver target/debug/examples/so_runner

echo
echo "=== [4/5] differential test suite ===================================="
for combo in "${COMBOS[@]}"; do
  echo "--- cargo test --no-default-features --features '$combo'"
  # rebuild the cdylib for this combination before testing it
  cargo build "${CARGO_FLAGS[@]}" --no-default-features --features "$combo" --lib --bins --examples
  cargo test "${CARGO_FLAGS[@]}" --no-default-features --features "$combo" "$@"
done
echo "--- cargo test (default features)"
cargo build "${CARGO_FLAGS[@]}" --lib --bins --examples
cargo test "${CARGO_FLAGS[@]}" "$@"

echo
echo "=== [5/5] cross-checks against the other build configurations ========="
# a) the *release* Rust artefacts (optimised, panic=abort, debug-assertions off)
cargo build "${CARGO_FLAGS[@]}" --release --lib --bins --examples
DIFF_RUST_SO=target/release/libdriver.so \
DIFF_RUST_EXE=target/release/driver \
  cargo test "${CARGO_FLAGS[@]}" "$@"

# b) the same C translation unit compiled with -O2 (signed overflow in
#    `mul1[i]*mul2[i]` is UB, so this proves the Rust matches the optimiser's
#    two's-complement wraparound as well as -O0's)
mkdir -p target/copt
gcc -O2 -fPIC -shared -o target/copt/libcdriver_O2.so c_src/src/main.c
gcc -O2 -o target/copt/driver_O2 c_src/src/main.c
DIFF_C_SO=target/copt/libcdriver_O2.so \
DIFF_C_EXE=target/copt/driver_O2 \
  cargo test "${CARGO_FLAGS[@]}" "$@"

echo
echo "ALL CONFIGURATIONS PASSED"
