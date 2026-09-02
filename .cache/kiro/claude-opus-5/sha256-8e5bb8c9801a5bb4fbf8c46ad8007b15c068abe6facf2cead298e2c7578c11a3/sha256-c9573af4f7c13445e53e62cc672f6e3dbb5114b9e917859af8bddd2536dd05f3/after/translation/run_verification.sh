#!/usr/bin/env bash
# End-to-end verification: builds the C reference library and the Rust cdylib,
# then runs symbol/codegen parity, the differential suite across every feature
# combination and both profiles, and the mutation check.
set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(dirname "$here")"

echo "### 1/4  building the C reference library"
( cd "$root/c_src" && mkdir -p build && cd build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON > /dev/null \
    && cmake --build . > /dev/null ) || { echo "C build failed" >&2; exit 2; }
echo "     $root/c_src/build/libdriver.so"

echo
echo "### 2/4  building the Rust cdylib (debug + release)"
( cd "$here" && timeout 300 cargo build 2>&1 | tail -1 \
    && timeout 300 cargo build --release 2>&1 | tail -1 ) \
    || { echo "Rust build failed" >&2; exit 2; }

rc=0

echo
echo "### 3/4  symbol + codegen parity, all feature combinations, both profiles"
( cd "$here" && timeout 1200 ./check_features.sh ) || rc=1

echo
echo "### 4/4  mutation check (the suite must reject plausible mistranslations)"
( cd "$here" && timeout 1200 ./check_mutations.sh 2>&1 \
    | grep -E 'baseline|detected|NOT DETECTED|ALL MUTATIONS|UNDETECTED' ) || rc=1

echo
if (( rc == 0 )); then
    echo "=== VERIFICATION COMPLETE: all phases passed ==="
else
    echo "=== VERIFICATION FAILED ===" >&2
fi
exit $rc
