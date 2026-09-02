#!/usr/bin/env bash
# Reproduces the "which C build is the reference" measurement.
#
# `c_src/CMakeLists.txt` sets no CMAKE_BUILD_TYPE, so the documented build is
# -O0. This script builds the SAME, UNMODIFIED c_src a second time out-of-tree
# with optimisation enabled and runs the fuzz row of the differential suite
# against both artifacts, showing that the two C builds are not bit-identical to
# each other and that this crate matches the documented (-O0) one.
#
# Nothing inside c_src/ is modified: the optimized build tree lives in $TMPDIR.
set -uo pipefail

cd "$(dirname "$0")" || exit 1
CSRC="$(cd ../c_src && pwd)"
ALT="${TMPDIR:-/tmp}/c_src_optimized_build.$$"

cleanup() { rm -rf "$ALT"; }
trap cleanup EXIT

echo "=== documented build (no CMAKE_BUILD_TYPE => -O0) ==="
(
    cd "$CSRC" && mkdir -p build && cd build \
        && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
        && cmake --build . >/dev/null
) || { echo "documented C build FAILED"; exit 1; }
DOC_SO="$(find "$CSRC/build" -maxdepth 1 -name 'lib*.so' | head -n1)"
echo "  -> $DOC_SO"

echo "=== optimized build (CMAKE_BUILD_TYPE=Release, out-of-tree) ==="
cmake -S "$CSRC" -B "$ALT" -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
    -DCMAKE_BUILD_TYPE=Release >/dev/null || { echo "optimized configure FAILED"; exit 1; }
cmake --build "$ALT" >/dev/null || { echo "optimized build FAILED"; exit 1; }
ALT_SO="$(find "$ALT" -maxdepth 1 -name 'lib*.so' | head -n1)"
echo "  -> $ALT_SO"

timeout 600 cargo build --release >/dev/null 2>&1 || { echo "cargo build FAILED"; exit 1; }

echo
echo "=== Rust .so vs the DOCUMENTED (-O0) C build — must be 0 divergences ==="
if C_SO="$DOC_SO" timeout 600 cargo test --release --test phase_b_valid c20 2>&1 \
        | grep -E 'diverged|test result'; then :; fi

echo
echo "=== Rust .so vs the OPTIMIZED C build — expected to diverge ==="
echo "    (NaN-payload provenance only; the two C builds disagree with each other,"
echo "     so no single implementation can match both)"
if C_SO="$ALT_SO" timeout 600 cargo test --release --test phase_b_valid c20 2>&1 \
        | grep -E 'diverged|test result'; then :; fi

echo
echo "=== operand order actually emitted, for the record ==="
for pair in "documented(-O0):$DOC_SO" "optimized:$ALT_SO"; do
    label="${pair%%:*}"; so="${pair#*:}"
    echo "--- $label ---"
    objdump -d --no-show-raw-insn "$so" \
        | sed -n '/<to_barycentric>:/,/^$/p;/<lm_dot2>:/,/^$/p' \
        | grep -E 'mulss|addss|subss|divss' | head -40
done
