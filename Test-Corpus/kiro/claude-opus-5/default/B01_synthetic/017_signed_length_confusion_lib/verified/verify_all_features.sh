#!/usr/bin/env bash
# Differential verification of the Rust translation against the C ground truth,
# across every valid build-time feature combination.
#
# Feature names are read out of [features] in Cargo.toml, so this keeps working
# if features are added later. For each combination the script:
#   1. cargo check
#   2. builds the cdylib into its own target dir
#   3. compares the exported dynamic symbols against the C .so
#   4. runs the differential test suite against that exact cdylib
#
# Usage: ./verify_all_features.sh
set -uo pipefail

cd "$(dirname "$0")" || exit 1
HERE=$(pwd)
C_SO="$HERE/../c_src/build/libdriver.so"
TIMEOUT=${TIMEOUT:-600}
FAILURES=0

# --- Build the C ground-truth library ----------------------------------------
if [[ ! -f "$C_SO" ]]; then
    echo "==> building C shared library"
    ( mkdir -p "$HERE/../c_src/build" && cd "$HERE/../c_src/build" \
        && timeout "$TIMEOUT" cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
        && timeout "$TIMEOUT" cmake --build . >/dev/null ) || {
        echo "FAIL: could not build the C library"; exit 1; }
fi

# --- Enumerate feature combinations ------------------------------------------
# Every name declared under [features], minus the "default" alias itself.
mapfile -t FEATURES < <(
    awk '
        /^\[features\]/ { inside = 1; next }
        /^\[/           { inside = 0 }
        inside && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
            sub(/[[:space:]]*=.*/, "", $0); if ($0 != "default") print $0
        }
    ' Cargo.toml
)

COMBOS=()
n=${#FEATURES[@]}
if (( n == 0 )); then
    echo "==> Cargo.toml declares no [features]; the only configuration is the default"
else
    for (( mask = 0; mask < (1 << n); mask++ )); do
        combo=""
        for (( i = 0; i < n; i++ )); do
            (( mask & (1 << i) )) && combo+="${combo:+,}${FEATURES[i]}"
        done
        COMBOS+=("$combo")
    done
fi
# The powerset above already contains the empty set (== --no-default-features).
(( n == 0 )) && COMBOS+=("")
# Whatever `default` resolves to is a real shipping configuration too.
COMBOS+=("__DEFAULT__")

# --- Verify each combination --------------------------------------------------
for combo in "${COMBOS[@]}"; do
    if [[ "$combo" == "__DEFAULT__" ]]; then
        label="default features"
        flags=()
        tag="default"
    elif [[ -z "$combo" ]]; then
        label="--no-default-features (no features)"
        flags=(--no-default-features)
        tag="nofeat"
    else
        label="--no-default-features --features $combo"
        flags=(--no-default-features --features "$combo")
        tag="feat-${combo//,/_}"
    fi

    echo
    echo "======================================================================"
    echo "==> $label"
    echo "======================================================================"

    target="$HERE/target/verify/$tag"

    echo "--- cargo check"
    if ! timeout "$TIMEOUT" cargo check "${flags[@]}" --target-dir "$target" 2>&1 | tail -n 5; then
        echo "FAIL: cargo check failed for $label"; FAILURES=$((FAILURES + 1)); continue
    fi

    echo "--- cargo build (cdylib)"
    if ! timeout "$TIMEOUT" cargo build --lib "${flags[@]}" --target-dir "$target" 2>&1 | tail -n 5; then
        echo "FAIL: cargo build failed for $label"; FAILURES=$((FAILURES + 1)); continue
    fi
    rust_so="$target/debug/libdriver.so"
    if [[ ! -f "$rust_so" ]]; then
        echo "FAIL: no cdylib produced at $rust_so"; FAILURES=$((FAILURES + 1)); continue
    fi

    echo "--- exported symbol parity (nm -D)"
    syms() { nm -D --defined-only "$1" | awk '{print $3}' \
        | grep -Ev '^(_init|_fini|_edata|_end|_IO_stdin_used|__|_ITM_)' | sort -u; }
    missing=$(comm -23 <(syms "$C_SO") <(syms "$rust_so"))
    if [[ -n "$missing" ]]; then
        echo "FAIL: Rust .so is missing symbols exported by the C .so:"
        echo "$missing" | sed 's/^/      /'
        FAILURES=$((FAILURES + 1))
    else
        echo "    OK: $(syms "$C_SO" | tr '\n' ' ')"
    fi

    echo "--- cargo test (C vs Rust through the FFI boundary)"
    if ! DRIVER_RUST_SO="$rust_so" timeout "$TIMEOUT" \
            cargo test "${flags[@]}" --target-dir "$target" 2>&1 \
            | grep -E '^(test |error|failures:|test result:)' ; then
        echo "FAIL: cargo test failed for $label"; FAILURES=$((FAILURES + 1)); continue
    fi
    if ! DRIVER_RUST_SO="$rust_so" timeout "$TIMEOUT" \
            cargo test "${flags[@]}" --target-dir "$target" >/dev/null 2>&1; then
        echo "FAIL: cargo test reported failures for $label"; FAILURES=$((FAILURES + 1))
    fi
done

echo
if (( FAILURES == 0 )); then
    echo "ALL CONFIGURATIONS PASS (${#COMBOS[@]} checked)"
else
    echo "$FAILURES CONFIGURATION(S) FAILED"
fi
exit $(( FAILURES > 0 ))
