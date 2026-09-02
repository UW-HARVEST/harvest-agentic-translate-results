#!/usr/bin/env bash
# Phase D: run cargo check + the whole differential test suite for EVERY feature
# combination declared in Cargo.toml.
#
# The feature list is extracted from Cargo.toml rather than hard-coded, so adding
# a feature automatically widens the matrix.
set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$here"

# --- extract [features] keys ------------------------------------------------
mapfile -t FEATURES < <(
    awk '
        /^\[/ { inf = ($0 == "[features]"); next }
        inf && /=/ && $0 !~ /^[[:space:]]*#/ {
            split($0, a, "=");
            gsub(/[[:space:]]/, "", a[1]);
            if (a[1] != "" && a[1] != "default") print a[1]
        }
    ' Cargo.toml
)

echo "features declared in Cargo.toml: ${#FEATURES[@]} ${FEATURES[*]-}"

# --- build the combination list --------------------------------------------
COMBOS=()
if [[ ${#FEATURES[@]} -eq 0 ]]; then
    # No features -> the default build IS the only configuration, and
    # --no-default-features is identical to it. Both are still run, to prove it.
    COMBOS+=("default")
    COMBOS+=("--no-default-features")
else
    n=${#FEATURES[@]}
    total=$((1 << n))
    for ((mask = 0; mask < total; mask++)); do
        sel=()
        for ((i = 0; i < n; i++)); do
            (((mask >> i) & 1)) && sel+=("${FEATURES[$i]}")
        done
        if [[ ${#sel[@]} -eq 0 ]]; then
            COMBOS+=("--no-default-features")
        else
            COMBOS+=("--no-default-features --features $(IFS=,; echo "${sel[*]}")")
        fi
    done
    COMBOS+=("default")
fi

echo "combinations to verify: ${#COMBOS[@]}"
echo

fail=0
for combo in "${COMBOS[@]}"; do
    if [[ "$combo" == "default" ]]; then
        flags=()
    else
        read -r -a flags <<< "$combo"
    fi
    echo "======================================================================"
    echo "combination: $combo"
    echo "======================================================================"

    if ! timeout 600 cargo check --release "${flags[@]}" 2>&1 | tail -n 5; then
        echo "FAIL: cargo check failed for [$combo]"
        fail=1
        continue
    fi

    if ! timeout 600 cargo build --release "${flags[@]}" 2>&1 | tail -n 3; then
        echo "FAIL: cargo build failed for [$combo]"
        fail=1
        continue
    fi

    if ! ./check_symbols.sh; then
        echo "FAIL: symbol parity failed for [$combo]"
        fail=1
        continue
    fi

    if ! timeout 600 cargo test --release "${flags[@]}" 2>&1 | tail -n 40; then
        echo "FAIL: cargo test failed for [$combo]"
        fail=1
        continue
    fi
    echo "OK: [$combo]"
    echo
done

if [[ $fail -ne 0 ]]; then
    echo "RESULT: at least one feature combination FAILED"
    exit 1
fi
echo "RESULT: all ${#COMBOS[@]} feature combination(s) passed"
