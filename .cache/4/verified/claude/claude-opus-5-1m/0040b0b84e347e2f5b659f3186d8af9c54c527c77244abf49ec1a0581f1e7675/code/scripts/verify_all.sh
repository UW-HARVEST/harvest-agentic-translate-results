#!/usr/bin/env bash
# One-shot reproduction of the whole verification (Phases A–D).
#
#   scripts/verify_all.sh [--with-mutation]
#
# Steps:
#   1. build the C executable with CMake and the C shared library with gcc
#   2. build the Rust cdylib, bin and the libloading test host
#   3. Phase D symbol parity: nm -D diff must be empty
#   4. Phase B+C: the differential suite, dev and release, for every feature combo
#   5. optional: harness self-validation via mutation testing
set -uo pipefail
cd "$(dirname "$0")/.."
fail=0
step() { echo; echo "### $*"; }

step "1a. C executable via CMake (c_src/CMakeLists.txt, unmodified)"
mkdir -p c_src/build
(cd c_src/build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null && cmake --build . ) \
    | tail -2 || fail=1

step "1b. C shared library (same TU, same -fno-strict-aliasing flag)"
mkdir -p cbuild
gcc -shared -fPIC -fno-strict-aliasing -o cbuild/libdriver_c.so c_src/src/main.c || fail=1
ls -l cbuild/libdriver_c.so

step "2. Rust artifacts (cdylib + bin + libloading host)"
cargo build --offline --all-targets 2>&1 | tail -2 || fail=1

step "3. Phase D — nm -D export parity (C .so vs Rust .so)"
diff <(nm -D --defined-only cbuild/libdriver_c.so     | awk '{print $3}' | sort) \
     <(nm -D --defined-only target/debug/libdriver.so | awk '{print $3}' | sort) \
    && echo "symbol diff EMPTY (ok)" || { echo "SYMBOL DIFF NOT EMPTY"; fail=1; }
echo "C   exports: $(nm -D --defined-only cbuild/libdriver_c.so | awk '{print $3}' | sort | tr '\n' ' ')"
echo "Rust exports: $(nm -D --defined-only target/debug/libdriver.so | awk '{print $3}' | sort | tr '\n' ' ')"
echo "unresolved deps in the Rust .so:"; ldd target/debug/libdriver.so | grep -c "not found"

step "4. Phases B+C — differential suite over every feature combination and profile"
./scripts/check_all_features.sh || fail=1

if [ "${1:-}" = "--with-mutation" ]; then
    step "5. Harness self-validation (mutation testing)"
    python3 scripts/mutation_check.py || fail=1
fi

echo
if [ "$fail" -ne 0 ]; then
    echo "VERIFY_ALL: FAILURES PRESENT"
    exit 1
fi
echo "VERIFY_ALL: everything passed"
