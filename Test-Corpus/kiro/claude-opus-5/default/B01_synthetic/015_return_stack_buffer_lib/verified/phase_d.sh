#!/usr/bin/env bash
# Phase D: enumerate every feature combination from Cargo.toml and run the
# full differential suite under each. Also re-verifies nm -D symbol parity.
set -uo pipefail

cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"
C_SO="$ROOT/c_src/build/libdriver.so"
R_SO="$ROOT/translation/target/release/libdriver.so"

# --- enumerate features -----------------------------------------------------
FEATURES=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /^[a-zA-Z0-9_-]+ *=/{print $1}' Cargo.toml)
if [ -z "$FEATURES" ]; then
    echo "== no [features] table in Cargo.toml: combinations = {default} =="
    COMBOS=("")
else
    # powerset of declared features
    COMBOS=("")
    for f in $FEATURES; do
        new=()
        for c in "${COMBOS[@]}"; do
            new+=("$c" "${c:+$c,}$f")
        done
        COMBOS=("${new[@]}")
    done
fi

FAIL=0
run_case() {
    local label="$1"; shift
    echo "----- $label -----"
    if ! timeout 600 cargo build --release "$@" >/tmp/pd_build.log 2>&1; then
        echo "BUILD FAILED ($label)"; tail -20 /tmp/pd_build.log; FAIL=1; return
    fi
    # symbol parity for this configuration
    local diff
    diff=$(comm -23 <(nm -D --defined-only "$C_SO" | awk '{print $3}' | sort) \
                    <(nm -D --defined-only "$R_SO" | awk '{print $3}' | sort))
    if [ -n "$diff" ]; then
        echo "SYMBOL PARITY FAILED ($label): missing from Rust .so:"; echo "$diff"; FAIL=1
    else
        echo "symbol parity: OK (0 missing)"
    fi
    if ! timeout 600 cargo test --release "$@" -- --test-threads=1 >/tmp/pd_test.log 2>&1; then
        echo "TESTS FAILED ($label)"; grep -E 'FAILED|panicked|DIVERGENCE|test result' /tmp/pd_test.log | head -30; FAIL=1
    else
        grep 'test result' /tmp/pd_test.log
    fi
}

run_case "default features" 
for c in "${COMBOS[@]}"; do
    if [ -z "$c" ]; then
        run_case "--no-default-features" --no-default-features
    else
        run_case "--no-default-features --features $c" --no-default-features --features "$c"
    fi
done
run_case "--all-features" --all-features

echo
if [ "$FAIL" -eq 0 ]; then echo "PHASE D: ALL CONFIGURATIONS PASS"; else echo "PHASE D: FAILURES PRESENT"; exit 1; fi
