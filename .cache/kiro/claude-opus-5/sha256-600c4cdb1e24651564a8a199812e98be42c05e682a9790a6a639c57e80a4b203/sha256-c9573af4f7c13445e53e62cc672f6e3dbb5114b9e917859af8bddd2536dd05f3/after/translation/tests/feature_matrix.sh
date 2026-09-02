#!/usr/bin/env bash
# Phase D — run the whole differential suite under EVERY feature combination.
#
# Feature names are extracted from Cargo.toml rather than hard-coded, so this
# keeps working if features are added later. For each combination it rebuilds
# the cdylib (so the .so under test matches the feature set), re-checks symbol
# parity against the C .so, and runs every test.
#
# Usage: tests/feature_matrix.sh [--release]

set -euo pipefail
cd "$(dirname "$0")/.."

PROFILE_ARGS=()
PROFILE_DIR=debug
if [[ "${1:-}" == "--release" ]]; then
    PROFILE_ARGS=(--release)
    PROFILE_DIR=release
fi

C_SO=$(find ../c_src/build -maxdepth 1 -name 'lib*.so' | sort | tail -1)
if [[ -z "$C_SO" ]]; then
    echo "ERROR: C .so not built. Run:" >&2
    echo "  cd ../c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ." >&2
    exit 1
fi
echo "C library: $C_SO"

# --- Extract the [features] table from Cargo.toml -------------------------
mapfile -t FEATURES < <(
    awk '
        /^\[features\]/ { inside = 1; next }
        /^\[/           { inside = 0 }
        inside && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
            sub(/[[:space:]]*=.*/, "");
            if ($0 != "default") print
        }
    ' Cargo.toml
)

echo "Declared features: ${#FEATURES[@]} ${FEATURES[*]:-(none)}"

# --- Build the list of combinations to test -------------------------------
COMBOS=()
if [[ ${#FEATURES[@]} -eq 0 ]]; then
    # No features declared: the default build is the only configuration.
    COMBOS=("<default>")
else
    COMBOS+=("<default>")
    COMBOS+=("<no-default>")
    N=${#FEATURES[@]}
    for ((mask = 1; mask < (1 << N); mask++)); do
        set=()
        for ((i = 0; i < N; i++)); do
            (((mask >> i) & 1)) && set+=("${FEATURES[i]}")
        done
        COMBOS+=("$(
            IFS=,
            echo "${set[*]}"
        )")
    done
    COMBOS+=("<all-features>")
fi

echo "Combinations to verify: ${#COMBOS[@]}"

FAILED=()
for combo in "${COMBOS[@]}"; do
    case "$combo" in
    "<default>") ARGS=() ;;
    "<no-default>") ARGS=(--no-default-features) ;;
    "<all-features>") ARGS=(--all-features) ;;
    *) ARGS=(--no-default-features --features "$combo") ;;
    esac

    echo
    echo "=============================================================="
    echo "FEATURE COMBINATION: $combo   (cargo ${ARGS[*]:-})"
    echo "=============================================================="

    if ! timeout 600 cargo build "${PROFILE_ARGS[@]}" "${ARGS[@]}" 2>&1 | tail -3; then
        FAILED+=("$combo (build)")
        continue
    fi

    RUST_SO="target/$PROFILE_DIR/libcapsule_lib.so"
    # Symbol parity for THIS feature set.
    missing=$(comm -23 \
        <(nm -D --defined-only "$C_SO" | awk '$2=="T"||$2=="D"||$2=="B"{print $3}' | sort) \
        <(nm -D --defined-only "$RUST_SO" | awk '$2=="T"||$2=="D"||$2=="B"{print $3}' | sort))
    extra=$(comm -13 \
        <(nm -D --defined-only "$C_SO" | awk '$2=="T"||$2=="D"||$2=="B"{print $3}' | sort) \
        <(nm -D --defined-only "$RUST_SO" | awk '$2=="T"||$2=="D"||$2=="B"{print $3}' | sort))
    if [[ -n "$missing" || -n "$extra" ]]; then
        echo "SYMBOL PARITY FAILURE for '$combo'"
        [[ -n "$missing" ]] && echo "  missing from Rust: $missing"
        [[ -n "$extra" ]] && echo "  extra in Rust:     $extra"
        FAILED+=("$combo (symbols)")
        continue
    fi
    echo "symbol parity: OK ($(nm -D --defined-only "$C_SO" | awk '$2=="T"{print}' | wc -l) exported functions)"

    if timeout 600 cargo test "${PROFILE_ARGS[@]}" "${ARGS[@]}" 2>&1 | tee /tmp/ft.log | grep -E '^test result'; then
        if grep -qE 'FAILED|test result: FAILED' /tmp/ft.log; then
            FAILED+=("$combo (tests)")
        fi
    else
        FAILED+=("$combo (tests)")
    fi
done

echo
echo "=============================================================="
if [[ ${#FAILED[@]} -eq 0 ]]; then
    echo "ALL ${#COMBOS[@]} FEATURE COMBINATION(S) PASSED"
    exit 0
else
    echo "FAILURES: ${FAILED[*]}"
    exit 1
fi
