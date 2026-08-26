#!/usr/bin/env bash
# Full verification driver: builds the C .so, then for EVERY cargo feature
# combination builds the Rust cdylib, diffs `nm -D` symbols, and runs the
# differential test suite.
set -uo pipefail
cd "$(dirname "$0")"

C_SO=c_src/build/libtranslated_rust.so
RUST_NAME=libdequantize_granule_lib.so

echo "=== building C shared library ==="
mkdir -p c_src/build
( cd c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
test -f "$C_SO" || { echo "missing $C_SO"; exit 1; }

# ---- enumerate every valid feature combination (powerset of [features]) ----
mapfile -t FEATURES < <(python3 - <<'PY'
import re,sys
txt=open("Cargo.toml").read()
m=re.search(r'^\[features\](.*?)(^\[|\Z)', txt, re.S|re.M)
feats=[]
if m:
    for line in m.group(1).splitlines():
        line=line.split('#')[0].strip()
        if '=' in line:
            name=line.split('=')[0].strip()
            if name and name!='default':
                feats.append(name)
print('\n'.join(feats))
PY
)

# mapfile can leave a single empty element for empty input -- drop blanks
TMPF=(); for f in "${FEATURES[@]:-}"; do [ -n "$f" ] && TMPF+=("$f"); done
FEATURES=("${TMPF[@]:-}"); [ -z "${FEATURES[0]:-}" ] && FEATURES=()

COMBOS=()
if [ "${#FEATURES[@]}" -eq 0 ]; then
    COMBOS=("")            # only the empty (default) configuration exists
else
    n=${#FEATURES[@]}
    for ((mask=0; mask<(1<<n); mask++)); do
        combo=""
        for ((i=0; i<n; i++)); do
            if (( mask & (1<<i) )); then combo="${combo:+$combo,}${FEATURES[$i]}"; fi
        done
        COMBOS+=("$combo")
    done
fi
echo "=== feature combinations: ${#COMBOS[@]} (features: ${FEATURES[*]:-<none>}) ==="

FAIL=0
for combo in "${COMBOS[@]}"; do
    label="${combo:-<default/empty>}"
    echo
    echo "############ FEATURE COMBO: $label ############"

    echo "--- cargo check ---"
    if ! timeout 600 cargo check --no-default-features ${combo:+--features "$combo"} 2>&1 | tail -5; then
        echo "CHECK FAILED for $label"; FAIL=1; continue
    fi

    echo "--- cargo build (emit cdylib) ---"
    if ! timeout 600 cargo build --no-default-features ${combo:+--features "$combo"} 2>&1 | tail -5; then
        echo "BUILD FAILED for $label"; FAIL=1; continue
    fi
    R_SO="target/debug/$RUST_NAME"
    test -f "$R_SO" || { echo "missing $R_SO"; FAIL=1; continue; }

    echo "--- nm -D symbol diff (C vs Rust) ---"
    diff <(nm -D --defined-only "$C_SO" | awk '{print $NF}' | sort -u) \
         <(nm -D --defined-only "$R_SO" | awk '{print $NF}' | sort -u) \
         && echo "symbol diff: EMPTY (parity OK)" \
         || { echo "SYMBOL PARITY FAILED for $label"; FAIL=1; }

    echo "--- differential tests (Phase B + Phase C) ---"
    if ! timeout 600 cargo test --no-default-features ${combo:+--features "$combo"} -- --test-threads=4 2>&1 | tail -45; then
        echo "TESTS FAILED for $label"; FAIL=1
    fi
done

echo
if [ "$FAIL" -eq 0 ]; then echo "=== ALL FEATURE COMBINATIONS PASSED ==="; else echo "=== FAILURES PRESENT ==="; fi
exit $FAIL
