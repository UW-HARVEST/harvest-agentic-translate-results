#!/usr/bin/env bash
# Full verification run: build both libraries, diff their exported symbols, then
# run the differential test suite under every feature combination.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$HERE/.." && pwd)"

echo "== building C shared library =="
mkdir -p "$ROOT/c_src/build"
(cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null)
C_SO="$ROOT/c_src/build/libdriver.so"

echo "== enumerating feature combinations from Cargo.toml =="
# No [features] section => the only configuration is the default (empty) set.
if grep -q '^\[features\]' "$HERE/Cargo.toml"; then
  echo "!! Cargo.toml grew a [features] section; extend this script." >&2
  exit 1
fi
COMBOS=("default" "no-default")

for combo in "${COMBOS[@]}"; do
  echo
  echo "===================== feature combo: $combo ====================="
  if [ "$combo" = "default" ]; then
    FLAGS=()
  else
    FLAGS=(--no-default-features)
  fi

  echo "-- cargo check --"
  (cd "$HERE" && cargo check --offline "${FLAGS[@]}")

  echo "-- building Rust cdylib --"
  (cd "$HERE" && cargo build --release --offline "${FLAGS[@]}")
  RUST_SO="$HERE/target/release/libdriver.so"

  echo "-- symbol parity (nm -D) --"
  diff <(nm -D --defined-only "$C_SO"    | awk '{print $NF}' | sort) \
       <(nm -D --defined-only "$RUST_SO" | awk '{print $NF}' | sort) \
    && echo "   symbol diff EMPTY: OK"

  echo "-- differential tests --"
  (cd "$HERE" && cargo test --offline "${FLAGS[@]}")
done

echo
echo "ALL FEATURE COMBINATIONS PASSED"
