#!/bin/bash
# Differential test driver.
#
# IMPORTANT: `cargo test` alone is NOT sufficient. The crate is `crate-type =
# ["cdylib"]`, and an integration test cannot link a cdylib, so Cargo never
# builds the `.so` during `cargo test`. Without the explicit `cargo build` below
# the tests would dlopen a stale artifact from a previous build and pass
# vacuously. `tests/common/mod.rs` also guards against this at runtime.
set -u
cd "$(dirname "$0")" || exit 1

# Cargo.toml has no [features], so the complete feature matrix is a single
# empty-feature build. Enumerated explicitly so adding features later is caught.
COMBOS=("")

# 1. C reference library.
if [ ! -d c_src/build ]; then
  mkdir -p c_src/build
  (cd c_src/build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON) >/dev/null || exit 1
fi
(cd c_src/build && cmake --build .) >/dev/null || { echo "C build FAILED"; exit 1; }
echo "C reference library built."

rc_total=0
for combo in "${COMBOS[@]}"; do
  if [ -z "$combo" ]; then
    label="<no features>"
    featargs=(--no-default-features)
  else
    label="$combo"
    featargs=(--no-default-features --features "$combo")
  fi
  echo
  echo "=============================================================="
  echo " feature combination: $label"
  echo "=============================================================="

  timeout 600 cargo check "${featargs[@]}" || { echo "cargo check FAILED"; rc_total=1; continue; }
  # MUST come before `cargo test` so the dlopen'd cdylib is current.
  timeout 600 cargo build "${featargs[@]}" || { echo "cargo build FAILED"; rc_total=1; continue; }
  timeout 600 cargo test  "${featargs[@]}" || rc_total=1
done

echo
echo "=============================================================="
echo " release cdylib (optimized, panic=abort) via CRC16_RUST_SO"
echo "=============================================================="
if timeout 600 cargo build --release --no-default-features; then
  CRC16_RUST_SO="$PWD/target/release/libcrc16_lib.so" \
    timeout 600 cargo test --no-default-features || rc_total=1
else
  echo "release build FAILED"; rc_total=1
fi

echo
if [ $rc_total -eq 0 ]; then
  echo "ALL FEATURE COMBINATIONS PASSED"
else
  echo "FAILURES PRESENT"
fi
exit $rc_total
