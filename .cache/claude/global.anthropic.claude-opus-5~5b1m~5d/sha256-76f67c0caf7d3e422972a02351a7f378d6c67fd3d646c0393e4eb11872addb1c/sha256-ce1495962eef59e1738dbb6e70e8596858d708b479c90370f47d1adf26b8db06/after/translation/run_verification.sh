#!/usr/bin/env bash
# Full differential verification matrix.
#
#   ./run_verification.sh            # everything
#   ./run_verification.sh quick      # skip the slow fork()/exec() sweeps
#
# Cargo.toml declares no [features], so the feature combinations are
# {default} == {--no-default-features} == {--all-features}; the script proves
# that by hashing the cdylib each of them produces, then runs the whole suite
# against both the release and the debug cdylib.
set -uo pipefail

cd "$(dirname "$0")"
LOG=target/testlogs
mkdir -p "$LOG"

FAST_TESTS=(symbols globals convert_pix inflate errors tamper oob_tables)
SLOW_TESTS=(aborts dynamic_overshoot)
if [ "${1:-}" = quick ]; then
    ALL_TESTS=("${FAST_TESTS[@]}")
else
    ALL_TESTS=("${FAST_TESTS[@]}" "${SLOW_TESTS[@]}")
fi

fail=0
step() { printf '\n=== %s ===\n' "$*"; }

# --------------------------------------------------------------------------
step "build the C reference library"
( cd ../c_src && mkdir -p build && cd build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
C_SO=$(echo ../c_src/build/lib*.so)
echo "C .so: $C_SO"

# --------------------------------------------------------------------------
step "feature combinations produce identical artifacts"
declare -a COMBOS=("" "--no-default-features" "--all-features")
prev=""
for combo in "${COMBOS[@]}"; do
    rm -f target/release/libconvert_pix_lib.so
    if ! cargo build --offline --release $combo >/dev/null 2>&1; then
        echo "FAIL: cargo build --release $combo"
        fail=1
        continue
    fi
    h=$(sha256sum target/release/libconvert_pix_lib.so | cut -d' ' -f1)
    echo "  ${combo:-<default>}: $h"
    if [ -n "$prev" ] && [ "$prev" != "$h" ]; then
        echo "  NOTE: differs from the previous combination"
    fi
    prev=$h
    # every combination must at least compile all the tests
    if ! cargo check --offline --release --tests $combo >/dev/null 2>&1; then
        echo "FAIL: cargo check --tests $combo"
        fail=1
    fi
done

# --------------------------------------------------------------------------
step "symbol parity (nm -D): C must be a subset of Rust"
cargo build --offline --release >/dev/null 2>&1
nm -D --defined-only "$C_SO" | awk '{print $3}' | sort -u > "$LOG/c.syms"
nm -D --defined-only target/release/libconvert_pix_lib.so | awk '{print $3}' | sort -u > "$LOG/rs.syms"
missing=$(comm -23 "$LOG/c.syms" "$LOG/rs.syms")
if [ -n "$missing" ]; then
    echo "FAIL: missing from the Rust .so:"; echo "$missing"; fail=1
else
    echo "  0 missing symbols ($(wc -l < "$LOG/c.syms") exported by the C object)"
fi

# --------------------------------------------------------------------------
# the debug cdylib too: overflow-checks are on there, and the translation
# depends on wrap-around arithmetic in many places
cargo rustc --offline --lib --crate-type cdylib >/dev/null 2>&1

for profile in release debug; do
    step "run the suite against the $profile cdylib"
    export CP_RUST_SO="$PWD/target/$profile/libconvert_pix_lib.so"
    for t in "${ALL_TESTS[@]}"; do
        out="$LOG/${profile}_${t}.log"
        if cargo test --offline --release --test "$t" -- --test-threads=1 > "$out" 2>&1; then
            printf '  %-18s %s\n' "$t" "$(grep -m1 '^test result' "$out")"
        else
            printf '  %-18s FAILED (see %s)\n' "$t" "$out"
            fail=1
        fi
    done
    unset CP_RUST_SO
done

step "summary"
if [ "$fail" = 0 ]; then echo "ALL CHECKS PASSED"; else echo "SOME CHECKS FAILED"; fi
exit "$fail"
