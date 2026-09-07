#!/usr/bin/env bash
# Full verification run: builds both libraries, diffs exported symbols, and runs
# the differential test suite across every feature combination and both cdylib
# profiles. Idempotent; safe to re-run.
set -euo pipefail

CRATE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ROOT="$(cd "$CRATE_DIR/.." && pwd)"
cd "$CRATE_DIR"

echo "== 1. build C shared library =="
mkdir -p "$ROOT/c_src/build"
(
  cd "$ROOT/c_src/build"
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null
  cmake --build . >/dev/null
)
C_SO="$ROOT/c_src/build/libdriver.so"
ls -l "$C_SO"

echo
echo "== 2. build Rust cdylib (release + debug) =="
timeout 600 cargo build --release
timeout 600 cargo build

echo
echo "== 3. symbol parity (nm -D) =="
nm -D --defined-only "$C_SO" | awk '{print $NF}' | sort -u >/tmp/driver_syms_c.txt
nm -D --defined-only target/release/libdriver.so | awk '{print $NF}' | sort -u >/tmp/driver_syms_rs.txt
echo "C exports:    $(tr '\n' ' ' </tmp/driver_syms_c.txt)"
echo "Rust exports: $(tr '\n' ' ' </tmp/driver_syms_rs.txt)"
MISSING="$(comm -23 /tmp/driver_syms_c.txt /tmp/driver_syms_rs.txt)"
if [ -n "$MISSING" ]; then
  echo "FAIL: symbols exported by C but missing from Rust:"
  echo "$MISSING"
  exit 1
fi
echo "OK: symbol diff (C - Rust) is empty"

echo
echo "== 4. feature combinations =="
# Mechanically extract declared features (excluding "default") from Cargo.toml.
FEATURES="$(awk '
  /^\[features\]/ {inside=1; next}
  /^\[/           {inside=0}
  inside && /^[A-Za-z0-9_-]+[ \t]*=/ {
    split($0, a, "="); gsub(/[ \t]/, "", a[1]);
    if (a[1] != "default") print a[1]
  }' Cargo.toml || true)"

COMBOS=("default")
if [ -n "$FEATURES" ]; then
  COMBOS+=("none")
  for f in $FEATURES; do COMBOS+=("$f"); done
  COMBOS+=("all")
fi
echo "declared features: ${FEATURES:-<none>}"
echo "combinations to run: ${COMBOS[*]}"

run_suite() {
  local label="$1"; shift
  local rust_so="$1"; shift
  echo
  echo "---- tests [$label] against $(basename "$(dirname "$rust_so")")/libdriver.so ----"
  DRIVER_C_SO="$C_SO" DRIVER_RUST_SO="$rust_so" \
    timeout 600 cargo test --release "$@" -- --test-threads=1 2>&1 |
    grep -E "^(test |error|warning: unused|test result)" | tail -40
}

for combo in "${COMBOS[@]}"; do
  case "$combo" in
    default) args=() ;;
    none)    args=(--no-default-features) ;;
    all)     args=(--all-features) ;;
    *)       args=(--no-default-features --features "$combo") ;;
  esac
  # Exercise the exported symbols of BOTH cdylib profiles: the release build has
  # panic="abort", the debug build does not.
  run_suite "$combo" "$CRATE_DIR/target/release/libdriver.so" "${args[@]}"
  run_suite "$combo" "$CRATE_DIR/target/debug/libdriver.so" "${args[@]}"
done

echo
echo "== ALL VERIFICATION STEPS PASSED =="
