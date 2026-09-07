#!/usr/bin/env bash
# Phase D — run the full differential suite under EVERY feature combination.
#
# The feature list is extracted mechanically from Cargo.toml rather than
# hard-coded, so a feature added later is picked up automatically.
set -uo pipefail
cd "$(dirname "$0")"

CARGO="cargo --offline"

# --- enumerate features declared in Cargo.toml -----------------------------
FEATURES=$(python3 - <<'PY'
import re, sys
txt = open('Cargo.toml').read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', txt, re.M | re.S)
names = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if '=' in line:
            n = line.split('=')[0].strip().strip('"')
            if n and n != 'default':
                names.append(n)
print(' '.join(names))
PY
)

echo "declared features: [${FEATURES:-<none>}]"

# --- build the combination list (powerset, capped) -------------------------
COMBOS=()
if [[ -z "${FEATURES// }" ]]; then
    # No [features] table: the only configurations are the default and the
    # explicitly-empty one. They compile identical code, but we run both so the
    # "every feature combination" gate is genuinely exercised rather than assumed.
    COMBOS=("__default__" "__none__" "__all__")
else
    COMBOS=("__default__" "__none__" "__all__")
    read -r -a FARR <<< "$FEATURES"
    n=${#FARR[@]}
    for ((mask = 1; mask < (1 << n); mask++)); do
        combo=""
        for ((i = 0; i < n; i++)); do
            if (( mask & (1 << i) )); then combo="$combo,${FARR[$i]}"; fi
        done
        COMBOS+=("${combo#,}")
    done
fi

# --- make sure the C reference library exists ------------------------------
C_SO="../c_src/build/libSieve.so"
if [[ ! -f "$C_SO" ]]; then
    echo "building the C reference library..."
    ( cd ../c_src && mkdir -p build && cd build \
        && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
        && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
fi

fail=0
for combo in "${COMBOS[@]}"; do
    case "$combo" in
        __default__) flags=();                                   label="(default features)";;
        __none__)    flags=(--no-default-features);              label="--no-default-features";;
        __all__)     flags=(--all-features);                     label="--all-features";;
        *)           flags=(--no-default-features --features "$combo"); label="--features $combo";;
    esac

    echo
    echo "=================================================================="
    echo "FEATURE COMBO: $label"
    echo "=================================================================="

    if ! timeout 300 $CARGO build "${flags[@]}" 2>&1 | tail -3; then
        echo "BUILD FAILED for $label"; fail=1; continue
    fi

    # symbol parity for this combination
    diff <(nm -D --defined-only "$C_SO"                | awk '{print $NF}' | sort) \
         <(nm -D --defined-only target/debug/libSieve.so | awk '{print $NF}' | sort)
    if [[ $? -ne 0 ]]; then
        echo "SYMBOL PARITY FAILED for $label"; fail=1
    else
        echo "symbol parity: OK (diff empty)"
    fi

    if ! timeout 600 $CARGO test "${flags[@]}" --tests -- --test-threads=1 2>&1 \
            | grep -E '^(test result|error|warning: unused)' ; then
        echo "TEST RUN PRODUCED NO RESULT LINE for $label"; fail=1
    fi
    # propagate the real test exit status
    timeout 600 $CARGO test "${flags[@]}" --tests -- --test-threads=1 >/dev/null 2>&1 \
        || { echo "TESTS FAILED for $label"; fail=1; }
done

echo
if [[ $fail -eq 0 ]]; then
    echo "ALL FEATURE COMBINATIONS PASSED"
else
    echo "SOME FEATURE COMBINATIONS FAILED"
fi
exit $fail
