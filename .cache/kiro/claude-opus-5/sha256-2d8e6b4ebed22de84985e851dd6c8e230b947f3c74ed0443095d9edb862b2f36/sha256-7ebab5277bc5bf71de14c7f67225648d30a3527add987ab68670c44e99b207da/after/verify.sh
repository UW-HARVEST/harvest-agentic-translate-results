#!/usr/bin/env bash
# Full verification: enumerate every build configuration, build both libraries,
# compare exported symbols and run the whole differential test suite.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT"

fail=0

echo "=============================================================="
echo "1. Build-time configurations"
echo "=============================================================="
# Feature combinations come from [features] in Cargo.toml. There is no
# [features] table, and c_src/CMakeLists.txt exposes no options either, so the
# configuration space is a single point: the default.
if grep -q '^\[features\]' translation/Cargo.toml; then
    echo "ERROR: Cargo.toml grew a [features] table; this script needs updating."
    exit 1
fi
COMBOS=("<default>")
echo "feature combinations found: ${#COMBOS[@]} (${COMBOS[*]})"
echo
echo "cargo check --no-default-features (the only combination):"
( cd translation && timeout 600 cargo check --no-default-features --all-targets 2>&1 | tail -3 ) || fail=1
echo

echo "=============================================================="
echo "2. Build the C reference shared library"
echo "=============================================================="
mkdir -p c_src/build
( cd c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON > /tmp/jansson_cmake.log 2>&1 \
  && cmake --build . > /tmp/jansson_cbuild.log 2>&1 ) || { echo "C build FAILED"; tail -20 /tmp/jansson_cbuild.log; exit 1; }
ls -l c_src/build/libjansson.so
echo

echo "=============================================================="
echo "3. Build the Rust shared library"
echo "=============================================================="
( cd translation && timeout 600 cargo build --release --no-default-features 2>&1 | tail -3 ) || { echo "Rust build FAILED"; exit 1; }
ls -l translation/target/release/libjansson.so
echo

echo "=============================================================="
echo "4. Exported symbol parity (nm -D)"
echo "=============================================================="
nm -D --defined-only c_src/build/libjansson.so | awk '{print $3}' | sort -u > /tmp/jansson_c_syms.txt
nm -D --defined-only translation/target/release/libjansson.so | awk '{print $3}' | sort -u > /tmp/jansson_r_syms.txt
echo "C exports:    $(wc -l < /tmp/jansson_c_syms.txt)"
echo "Rust exports: $(wc -l < /tmp/jansson_r_syms.txt)"
missing=$(comm -23 /tmp/jansson_c_syms.txt /tmp/jansson_r_syms.txt)
if [ -n "$missing" ]; then
    echo "MISSING FROM THE RUST .so:"
    echo "$missing" | sed 's/^/  /'
    fail=1
else
    echo "OK: every symbol the C .so exports is exported by the Rust .so."
fi
extra=$(comm -13 /tmp/jansson_c_syms.txt /tmp/jansson_r_syms.txt)
if [ -n "$extra" ]; then
    echo "(informational) extra symbols in the Rust .so:"
    echo "$extra" | sed 's/^/  /'
fi
echo

echo "=============================================================="
echo "5. Differential tests (C .so vs Rust .so, both via libloading)"
echo "=============================================================="
for combo in "${COMBOS[@]}"; do
    echo "--- feature combination: $combo ---"
    ( cd translation && timeout 600 cargo test --release --no-default-features -- --test-threads=1 2>&1 \
        | grep -E "^(running|test |test result|error)" ) || fail=1
done
echo

if [ "$fail" -ne 0 ]; then
    echo "RESULT: FAILURES (see above)"
    exit 1
fi
echo "RESULT: all configurations verified"
