#!/usr/bin/env bash
# Completeness evidence for Phases B and C: measure how much of the C source the
# differential suite actually executes.
#
# Builds a gcov-instrumented COPY of c_src/src/lib.c (c_src itself is never
# touched), points the harness at it with HARVEST_C_SO, runs the whole
# differential suite through it, and prints gcov's line/branch summary.
set -uo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
cd "$here" || exit 1

cov="$here/target/cov"
rm -rf "$cov"; mkdir -p "$cov"
cp ../c_src/src/lib.c "$cov/" || exit 1

( cd "$cov" && gcc -fPIC -shared --coverage -o libcov.so lib.c ) || {
    echo "FAIL: could not build the instrumented copy"; exit 1; }

# symbol_parity is skipped: the instrumented .so exports extra gcov symbols.
HARVEST_C_SO="$cov/libcov.so" cargo test --offline --release \
    --test valid_paths --test error_paths --test null_pointer --test smoke \
    > "$cov/test.log" 2>&1
status=$?
grep -E '^test result:' "$cov/test.log" | sed 's/^/  /'
if (( status != 0 )); then
    echo "FAIL: suite failed against the instrumented C library"
    grep -E 'FAILED|panicked' "$cov/test.log" | head -20
    exit 1
fi

( cd "$cov" && gcov -b -c -o libcov.so-lib.gcno lib.c ) | sed 's/^/  /'
echo
echo "branch directions never taken:"
grep -n 'branch' "$cov/lib.c.gcov" | grep -v 'taken [1-9]' | sed 's/^/  /' \
    || echo "  (none)"
echo
echo "(the only expected gap is lib.c:145 \`if (*name_ptr)\` FALSE -- provably"
echo " unreachable: maxnmin re-seeds six builtins whose names are all non-empty;"
echo " see ERRORS.md row E29)"
