#!/usr/bin/env bash
# Phase D -- run the full verification under every feature combination.
#
# `translation/Cargo.toml` declares no [features] table, so the cross-product is
# just {default, --no-default-features}. The loop is written generically so it
# keeps working if features are added later: it parses the [features] section out
# of Cargo.toml and enumerates the power set.
set -uo pipefail

cd "$(dirname "$0")/.." || exit 1
ROOT="$(cd .. && pwd)"
TIMEOUT=${TIMEOUT:-600}

# --- build the C reference library ----------------------------------------
if ! ls "$ROOT"/c_src/build/lib*.so >/dev/null 2>&1; then
    echo "== building C reference library =="
    ( cd "$ROOT/c_src" && mkdir -p build && cd build \
      && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
      && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
fi

# --- enumerate feature combinations ---------------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/      { inf=1; next }
    /^\[/                { inf=0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
        split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
        if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)

COMBOS=()
if [ "${#FEATURES[@]}" -eq 0 ]; then
    COMBOS+=("<default>" "<none>")
else
    COMBOS+=("<default>" "<none>")
    n=${#FEATURES[@]}
    for (( mask=1; mask < (1<<n); mask++ )); do
        combo=""
        for (( i=0; i<n; i++ )); do
            if (( mask & (1<<i) )); then combo="${combo:+$combo,}${FEATURES[i]}"; fi
        done
        COMBOS+=("$combo")
    done
fi

echo "== ${#COMBOS[@]} feature combination(s): ${COMBOS[*]} =="

FAIL=0
for combo in "${COMBOS[@]}"; do
    case "$combo" in
        "<default>") FLAGS=() ;;
        "<none>")    FLAGS=(--no-default-features) ;;
        *)           FLAGS=(--no-default-features --features "$combo") ;;
    esac

    echo
    echo "===================== features: $combo ====================="

    for stage in check build test; do
        case $stage in
            check) CMD=(cargo check --release "${FLAGS[@]}" --all-targets) ;;
            build) CMD=(cargo build --release "${FLAGS[@]}") ;;
            test)  CMD=(cargo test  --release "${FLAGS[@]}") ;;
        esac
        echo "--- ${CMD[*]}"
        if ! timeout "$TIMEOUT" "${CMD[@]}" 2>&1 | tail -n 30; then
            echo "!! FAILED: features=$combo stage=$stage"
            FAIL=1
        fi
    done
done

echo
if [ "$FAIL" -eq 0 ]; then
    echo "ALL FEATURE COMBINATIONS PASSED (${#COMBOS[@]})"
else
    echo "SOME FEATURE COMBINATIONS FAILED"
fi
exit "$FAIL"
