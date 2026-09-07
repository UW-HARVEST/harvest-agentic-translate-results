#!/usr/bin/env bash
# Differential verification driver.
#
# IMPORTANT: `cargo test` does NOT rebuild a `crate-type = ["cdylib"]` lib
# target, so the Rust .so must be built explicitly first — otherwise the tests
# load a stale .so and pass vacuously. Always go through this script.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(dirname "$here")"

# 1. Build the C shared library.
mkdir -p "$root/c_src/build"
( cd "$root/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null )

# 2. Enumerate feature combinations from Cargo.toml. This crate declares no
#    [features] table, so the set is just the default (empty) configuration;
#    the loop is written generically so it keeps working if features are added.
features=$(awk '
  /^\[features\]/ {inf=1; next}
  /^\[/           {inf=0}
  inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {gsub(/[[:space:]]*=.*/,""); print}
' "$here/Cargo.toml" | grep -v '^default$' || true)

combos=("")                       # default features
if [ -n "$features" ]; then
  combos+=("__nodefault__")       # --no-default-features, no features
  while read -r f; do
    [ -n "$f" ] && combos+=("$f")
  done <<< "$features"
  combos+=("$(echo "$features" | paste -sd, -)")   # all features at once
fi

for combo in "${combos[@]}"; do
  case "$combo" in
    "")            args=();                                              label="default" ;;
    "__nodefault__") args=(--no-default-features);                       label="no-default-features" ;;
    *)             args=(--no-default-features --features "$combo");     label="features=$combo" ;;
  esac

  echo "=============================================================="
  echo "### feature combination: $label"
  echo "=============================================================="

  ( cd "$here" && timeout 600 cargo build --release "${args[@]}" )
  ( cd "$here" && timeout 600 cargo check "${args[@]}" )
  ( cd "$here" && timeout 600 cargo test --release "${args[@]}" -- --test-threads="$(nproc)" )

  # Symbol parity check, independent of the in-test assertion.
  c_so=$(ls "$root"/c_src/build/*.so | head -1)
  nm -D --defined-only "$c_so" | awk '{print $3}' | sort -u > /tmp/c_syms.txt
  nm -D --defined-only "$here/target/release/libflac_validate_lib.so" \
    | awk '{print $3}' | sort -u > /tmp/rust_syms.txt
  missing=$(comm -23 /tmp/c_syms.txt /tmp/rust_syms.txt || true)
  if [ -n "$missing" ]; then
    echo "FAIL: symbols in C .so missing from Rust .so under $label:"
    echo "$missing"
    exit 1
  fi
  echo "symbol parity OK under $label (0 missing)"
done

echo
echo "ALL FEATURE COMBINATIONS PASSED"
