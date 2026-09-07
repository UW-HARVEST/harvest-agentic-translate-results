#!/usr/bin/env bash
# Phase D driver: enumerate every feature combination declared in Cargo.toml,
# build the cdylib for each, and run the full differential suite against BOTH
# the debug and the release .so.
#
# Usage:  ./check_features.sh
set -uo pipefail
cd "$(dirname "$0")"

C_BUILD=../c_src/build
if ! ls "$C_BUILD"/*.so >/dev/null 2>&1; then
    echo "building the C shared library first..."
    (cd ../c_src && mkdir -p build && cd build \
        && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
        && cmake --build . >/dev/null)
fi
C_SO=$(ls "$C_BUILD"/*.so | head -1)
export C_SO
echo "C .so: $C_SO"

# --- enumerate features declared by the crate -----------------------------
FEATURES=$(cargo metadata --no-deps --format-version 1 2>/dev/null \
    | tr ',' '\n' | grep -o '"features":{[^}]*}' | head -1 \
    | grep -o '"[a-zA-Z0-9_-]*":\[' | tr -d '":[' | sort -u)

if [ -z "$FEATURES" ]; then
    echo "no [features] declared -> the only configuration is the default one"
    COMBOS=("default")
else
    echo "declared features: $FEATURES"
    # power set of the declared features, plus the default build
    COMBOS=("default" "none")
    feats=($FEATURES)
    n=${#feats[@]}
    for ((mask = 1; mask < (1 << n); mask++)); do
        combo=""
        for ((b = 0; b < n; b++)); do
            if (((mask >> b) & 1)); then combo="$combo,${feats[$b]}"; fi
        done
        COMBOS+=("${combo#,}")
    done
fi

fail=0
for combo in "${COMBOS[@]}"; do
    case "$combo" in
        default) FLAGS=() ;;
        none) FLAGS=(--no-default-features) ;;
        *) FLAGS=(--no-default-features --features "$combo") ;;
    esac
    echo
    echo "================ feature combo: $combo ${FLAGS[*]} ================"
    timeout 600 cargo check "${FLAGS[@]}" >/dev/null 2>&1 \
        || { echo "  cargo check FAILED"; fail=1; continue; }

    for profile in debug release; do
        if [ "$profile" = release ]; then
            timeout 600 cargo build --release "${FLAGS[@]}" >/dev/null 2>&1
        else
            timeout 600 cargo build "${FLAGS[@]}" >/dev/null 2>&1
        fi
        so="target/$profile/libnormalize_lib.so"
        [ -f "$so" ] || { echo "  MISSING $so"; fail=1; continue; }
        echo "  --- testing against $so ---"
        RUST_SO="$so" timeout 600 cargo test "${FLAGS[@]}" 2>&1 \
            | grep -E '^test result|^error|FAILED' | sed 's/^/    /'
        rc=${PIPESTATUS[0]}
        [ "$rc" -eq 0 ] || { echo "    SUITE FAILED (rc=$rc)"; fail=1; }
    done
done

echo
if [ "$fail" -eq 0 ]; then
    echo "ALL FEATURE COMBINATIONS PASSED"
else
    echo "SOME COMBINATIONS FAILED"
fi
exit "$fail"
