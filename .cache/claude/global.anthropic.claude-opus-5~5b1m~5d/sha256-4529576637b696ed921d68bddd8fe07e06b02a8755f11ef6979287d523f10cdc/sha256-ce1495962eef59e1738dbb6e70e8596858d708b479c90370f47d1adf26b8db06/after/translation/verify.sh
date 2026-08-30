#!/usr/bin/env bash
# Full verification run: builds the C reference .so and the Rust cdylib, then
# runs the differential test suite across every feature combination and build
# profile. Exits non-zero on any divergence.
set -uo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(dirname "$HERE")"
fail=0

echo "### 1. build the C reference shared library"
cmake -S "$ROOT/c_src" -B "$ROOT/c_src/build" -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null
cmake --build "$ROOT/c_src/build" >/dev/null
ls -l "$ROOT/c_src/build/libdriver.so"

cd "$HERE"

echo
echo "### 2. cargo check"
cargo check --offline || fail=1

echo
echo "### 3. feature combinations"
# This crate declares no [features], so the only combination is the default.
# Enumerated mechanically so a future feature is picked up automatically.
FEATURES=$(cargo metadata --offline --no-deps --format-version 1 \
  | python3 -c 'import json,sys; m=json.load(sys.stdin); print(" ".join(k for p in m["packages"] for k in p["features"] if k!="default"))')
echo "declared features: [${FEATURES:-<none>}]"

run() { # label, cargo flags...
  local label="$1"; shift
  echo "--- $label"
  cargo build --offline -q "$@" || { echo "BUILD FAIL: $label"; fail=1; return; }
  if cargo test --offline -q "$@" 2>&1 | tail -30 | grep -qE 'FAILED|panicked|error\['; then
    echo "FAIL: $label"; fail=1
  else
    echo "PASS: $label"
  fi
}

run "default features"        # debug cdylib: overflow-checks ON
run "--no-default-features" --no-default-features
run "--all-features" --all-features

echo
echo "### 4. release cdylib (panic=abort, optimised, overflow-checks OFF)"
cargo build --offline -q --release
if RUST_DRIVER_SO="$HERE/target/release/libdriver.so" \
   cargo test --offline -q 2>&1 | tail -30 | grep -qE 'FAILED|panicked|error\['; then
  echo "FAIL: release cdylib"; fail=1
else
  echo "PASS: release cdylib"
fi

echo
echo "### 5. C reference at -O0 / -O2 / -O3 (signed-overflow UB sensitivity)"
TMP="${TMPDIR:-/tmp}"
for opt in O0 O2 O3; do
  cmake -S "$ROOT/c_src" -B "$TMP/cb_$opt" \
    -DCMAKE_POSITION_INDEPENDENT_CODE=ON -DCMAKE_C_FLAGS="-$opt" >/dev/null 2>&1
  cmake --build "$TMP/cb_$opt" >/dev/null 2>&1
  if C_DRIVER_SO="$TMP/cb_$opt/libdriver.so" \
     cargo test --offline -q 2>&1 | tail -30 | grep -qE 'FAILED|panicked'; then
    echo "FAIL: C -$opt"; fail=1
  else
    echo "PASS: C -$opt"
  fi
done

echo
echo "### 6. symbol parity (nm -D)"
diff <(nm -D --defined-only "$ROOT/c_src/build/libdriver.so"      | awk '$2=="T"||$2=="D"||$2=="B"{print $3}' | sort) \
     <(nm -D --defined-only "$HERE/target/release/libdriver.so"    | awk '$2=="T"||$2=="D"||$2=="B"{print $3}' | sort) \
     && echo "PASS: symbol sets identical" \
     || { echo "(diff above: '>' = extra in Rust, '<' = MISSING from Rust)"; \
          comm -23 <(nm -D --defined-only "$ROOT/c_src/build/libdriver.so" | awk '$2=="T"{print $3}' | sort) \
                   <(nm -D --defined-only "$HERE/target/release/libdriver.so" | awk '$2=="T"{print $3}' | sort) \
          | grep . && { echo "FAIL: symbols missing from Rust"; fail=1; } \
          || echo "PASS: no C symbol missing from Rust"; }

echo
echo "=============================================="
if [ "$fail" -eq 0 ]; then echo "VERIFICATION COMPLETE — all checks pass"; else echo "VERIFICATION FAILED"; fi
exit "$fail"
