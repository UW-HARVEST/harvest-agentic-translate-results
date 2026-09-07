#!/usr/bin/env bash
# Phase D — run the full differential suite under EVERY feature combination.
#
# Feature names are extracted mechanically from Cargo.toml rather than
# hard-coded, so this stays correct if a [features] table is ever added.
set -uo pipefail

cd "$(dirname "$0")"

TIMEOUT=${TIMEOUT:-600}

# ---- 1. make sure the C ground truth exists -------------------------------
C_BUILD=../c_src/build
if ! ls "$C_BUILD"/*.so >/dev/null 2>&1; then
    echo "== building C shared library =="
    (cd ../c_src && mkdir -p build && cd build \
        && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
        && cmake --build . >/dev/null) || { echo "C build FAILED"; exit 1; }
fi
C_SO=$(ls "$C_BUILD"/*.so | head -1)
echo "C .so   : $C_SO"

# ---- 2. enumerate feature combinations -----------------------------------
# Everything between `[features]` and the next `[section]`, minus `default`.
FEATURES=$(awk '
    /^\[features\]/ { inf=1; next }
    /^\[/           { inf=0 }
    inf && /=/      { split($0, a, "="); gsub(/[ \t"]/, "", a[1]);
                      if (a[1] != "default" && a[1] != "") print a[1] }
' Cargo.toml | sort -u)

COMBOS=()
if [ -z "$FEATURES" ]; then
    echo "Cargo.toml declares no [features] -> exactly one configuration."
    COMBOS+=("DEFAULT")
    COMBOS+=("NO_DEFAULT")
    COMBOS+=("ALL")
else
    # Full power set of the declared features, with and without defaults.
    feats=($FEATURES)
    n=${#feats[@]}
    for ((mask=0; mask < (1<<n); mask++)); do
        combo=""
        for ((b=0; b<n; b++)); do
            if (( mask & (1<<b) )); then combo="$combo,${feats[$b]}"; fi
        done
        combo="${combo#,}"
        COMBOS+=("NO_DEFAULT:$combo")
        COMBOS+=("DEFAULT:$combo")
    done
    COMBOS+=("ALL")
fi

# ---- 3. run cargo check + full test suite for each combination ------------
fail=0
for c in "${COMBOS[@]}"; do
    case "$c" in
        DEFAULT)          args=() ;;
        NO_DEFAULT)       args=(--no-default-features) ;;
        ALL)              args=(--all-features) ;;
        NO_DEFAULT:*)     f="${c#NO_DEFAULT:}"
                          args=(--no-default-features)
                          [ -n "$f" ] && args+=(--features "$f") ;;
        DEFAULT:*)        f="${c#DEFAULT:}"
                          args=()
                          [ -n "$f" ] && args+=(--features "$f") ;;
    esac

    echo
    echo "=================================================================="
    echo "== combination: $c   (cargo ${args[*]:-<none>})"
    echo "=================================================================="

    if ! timeout "$TIMEOUT" cargo check "${args[@]}" >/tmp/fc_check.log 2>&1; then
        echo "  cargo check FAILED"; tail -30 /tmp/fc_check.log; fail=1; continue
    fi
    echo "  cargo check ok"

    if ! timeout "$TIMEOUT" cargo build --release --lib "${args[@]}" >/tmp/fc_build.log 2>&1; then
        echo "  cargo build --release FAILED"; tail -30 /tmp/fc_build.log; fail=1; continue
    fi
    echo "  cargo build --release ok"

    if ! timeout "$TIMEOUT" cargo test "${args[@]}" >/tmp/fc_test.log 2>&1; then
        echo "  cargo test FAILED"
        grep -E 'DIVERGENCE|panicked|test result|^test .* FAILED' /tmp/fc_test.log | head -40
        fail=1; continue
    fi
    grep -E 'test result' /tmp/fc_test.log | sed 's/^/  /'
done

echo
if [ "$fail" -eq 0 ]; then
    echo "ALL FEATURE COMBINATIONS PASSED (${#COMBOS[@]} configurations)"
else
    echo "SOME FEATURE COMBINATIONS FAILED"
fi
exit "$fail"
