#!/usr/bin/env bash
# Phase D driver: enumerate every feature combination from Cargo.toml and run the
# full differential suite for each, in both the dev and release profiles, and
# assert exported-symbol parity with the C .so for each resulting cdylib.
set -uo pipefail
cd "$(dirname "$0")/.." || exit 1

C_SO=../c_src/build/libdriver.so
OFFLINE=--offline
fail=0

if [ ! -f "$C_SO" ]; then
    echo "building the C shared library first"
    ( cd ../c_src && mkdir -p build && cd build \
        && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
        && cmake --build . >/dev/null ) || exit 1
fi

# ---- enumerate feature combinations (powerset of [features]) ----------------
mapfile -t FEATURES < <(
    awk '
        /^\[features\]/ { in_f = 1; next }
        /^\[/           { in_f = 0 }
        in_f && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
            split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
            if (a[1] != "default") print a[1]
        }
    ' Cargo.toml
)

COMBOS=()
if [ "${#FEATURES[@]}" -eq 0 ]; then
    echo "Cargo.toml declares no [features]: the only configuration is the default."
    COMBOS+=("<default>" "<no-default-features>")
else
    n=${#FEATURES[@]}
    total=$((1 << n))
    COMBOS+=("<default>")
    for ((mask = 0; mask < total; mask++)); do
        combo=""
        for ((i = 0; i < n; i++)); do
            if (( mask & (1 << i) )); then
                combo="${combo:+$combo,}${FEATURES[i]}"
            fi
        done
        COMBOS+=("${combo:-<no-default-features>}")
    done
fi

echo "feature combinations to verify: ${COMBOS[*]}"
echo

for profile in dev release; do
    prof_flag=""
    prof_dir=debug
    if [ "$profile" = release ]; then prof_flag=--release; prof_dir=release; fi

    for combo in "${COMBOS[@]}"; do
        case "$combo" in
            "<default>")             feat_flags=() ;;
            "<no-default-features>") feat_flags=(--no-default-features) ;;
            *)                       feat_flags=(--no-default-features --features "$combo") ;;
        esac

        echo "=============================================================="
        echo "profile=$profile features=$combo"
        echo "=============================================================="

        if ! cargo build $OFFLINE $prof_flag "${feat_flags[@]}"; then
            echo "FAIL: build (profile=$profile features=$combo)"; fail=1; continue
        fi

        R_SO="target/$prof_dir/libdriver.so"

        # ---- symbol parity for this exact cdylib -------------------------
        symdiff="${TMPDIR:-/tmp}/symdiff.$$"
        diff <(nm -D --defined-only "$C_SO" | awk '{print $2, $3}' | sort) \
             <(nm -D --defined-only "$R_SO" | awk '{print $2, $3}' | sort) \
             > "$symdiff"
        if [ -s "$symdiff" ]; then
            echo "FAIL: exported-symbol mismatch (profile=$profile features=$combo):"
            cat "$symdiff"
            fail=1
        else
            echo "OK: exported symbols identical to the C .so"
        fi
        rm -f "$symdiff"

        # ---- full differential suite -------------------------------------
        if ! cargo test $OFFLINE $prof_flag "${feat_flags[@]}" -- --test-threads=4; then
            echo "FAIL: tests (profile=$profile features=$combo)"; fail=1
        fi
        echo
    done
done

echo "=============================================================="
if [ "$fail" -eq 0 ]; then
    echo "ALL FEATURE COMBINATIONS AND PROFILES PASSED"
else
    echo "FAILURES PRESENT"
fi
exit "$fail"
