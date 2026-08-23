#!/bin/bash
# Build everything the differential tests need, then run them.
#   ./check.sh                 -- all tests
#   ./check.sh --test errors   -- one test target
set -e
cd "$(dirname "$0")"
if [ ! -f c_src/build/libpng.so ]; then
    mkdir -p c_src/build
    (cd c_src/build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null && cmake --build . -j8 >/dev/null)
fi
cargo build --offline --release
# every test binary appends the diagnostics it observed here; start each run clean
rm -rf target/observed
mkdir -p target/observed
cargo test --offline --release "$@" 2>&1 | tee target/test_output.txt
rc=${PIPESTATUS:-$?}
grep -E '^test result:' target/test_output.txt || true
# whole-suite ERRORS.md coverage (each integration test is its own process)
python3 tools/error_coverage.py || true
# CONFIGS.md row coverage
python3 tools/config_coverage.py || true
# how many differential comparisons the run actually performed
python3 tools/count_cases.py || true
exit $rc
