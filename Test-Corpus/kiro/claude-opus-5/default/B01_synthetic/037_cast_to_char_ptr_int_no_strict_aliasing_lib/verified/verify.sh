#!/usr/bin/env bash
# Full verification driver: builds the C .so and the Rust .so (both profiles),
# diffs the exported symbol sets, and runs the differential suite under every
# feature combination declared in Cargo.toml.
#
# Usage: ./verify.sh          (run from the `translation/` directory)
set -uo pipefail

cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"
FAIL=0
step() { printf '\n=== %s ===\n' "$*"; }

step "Build C shared library"
( cd "$ROOT/c_src" && mkdir -p build && cd build \
  && timeout 600 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && timeout 600 cmake --build . ) || { echo "C build FAILED"; exit 1; }
C_SO="$ROOT/c_src/build/libdriver.so"
ls -l "$C_SO"

step "Enumerate feature combinations from Cargo.toml"
# No [features] section and no optional dependencies => exactly one combination.
if grep -q '^\[features\]' Cargo.toml; then
    FEATURES=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/{print $1}' Cargo.toml)
else
    FEATURES=""
fi
if [ -z "$FEATURES" ]; then
    echo "no [features] declared -> combinations: {default} and {--no-default-features}"
    COMBOS=("" "--no-default-features")
else
    echo "declared features: $FEATURES"
    COMBOS=("" "--no-default-features")
    for f in $FEATURES; do COMBOS+=("--no-default-features --features $f"); done
    COMBOS+=("--all-features")
fi

step "Build Rust .so, diff symbols, and run the suite per combination"
for combo in "${COMBOS[@]}"; do
    label="${combo:-<default features>}"
    printf '\n----- combination: %s -----\n' "$label"

    # shellcheck disable=SC2086
    timeout 600 cargo build $combo         >/dev/null 2>&1 || { echo "debug build FAILED ($label)";   FAIL=1; continue; }
    # shellcheck disable=SC2086
    timeout 600 cargo build --release $combo >/dev/null 2>&1 || { echo "release build FAILED ($label)"; FAIL=1; continue; }

    for prof in debug release; do
        R_SO="target/$prof/libdriver.so"
        missing=$(comm -23 \
            <(nm -D --defined-only --format=posix "$C_SO" | awk '{print $1}' | sort -u) \
            <(nm -D --defined-only --format=posix "$R_SO" | awk '{print $1}' | sort -u))
        if [ -n "$missing" ]; then
            echo "SYMBOL DIFF NOT EMPTY for $R_SO ($label):"; echo "$missing"; FAIL=1
        else
            echo "symbol diff empty: $R_SO exports every C symbol"
        fi
    done

    # shellcheck disable=SC2086
    if timeout 600 cargo test $combo 2>&1 | tail -n 40; then
        echo "tests OK ($label)"
    else
        echo "TESTS FAILED ($label)"; FAIL=1
    fi
done

step "Result"
if [ "$FAIL" -eq 0 ]; then echo "ALL CHECKS PASSED"; else echo "SOME CHECKS FAILED"; fi
exit "$FAIL"
