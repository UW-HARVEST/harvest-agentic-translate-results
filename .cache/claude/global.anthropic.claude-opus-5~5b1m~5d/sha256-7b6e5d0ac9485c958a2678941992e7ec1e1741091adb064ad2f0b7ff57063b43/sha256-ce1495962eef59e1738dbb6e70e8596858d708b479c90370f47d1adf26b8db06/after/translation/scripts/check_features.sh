#!/usr/bin/env bash
# Phase D: run the whole differential suite under EVERY feature combination,
# and under both cargo profiles (the release cdylib is `panic = "abort"`, the
# dev one unwinds, so they are genuinely different artifacts).
#
# Feature combinations are extracted from Cargo.toml rather than hard-coded, so
# this keeps working if features are ever added.
set -uo pipefail
cd "$(dirname "$0")/.." || exit 1

C_SO=../c_src/build/libdriver.so
if [ ! -f "$C_SO" ]; then
    echo "building the C reference shared library..."
    ( cd ../c_src && mkdir -p build && cd build \
        && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
        && cmake --build . >/dev/null ) || exit 1
fi

# --- enumerate feature combinations -----------------------------------------
mapfile -t FEATURES < <(
    python3 - <<'PY'
import re, sys
src = open('Cargo.toml').read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', src, re.M | re.S)
names = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if '=' in line:
            n = line.split('=')[0].strip().strip('"')
            if n != 'default':
                names.append(n)
for n in names:
    print(n)
PY
)

COMBOS=()
if [ "${#FEATURES[@]}" -eq 0 ]; then
    echo "Cargo.toml declares no [features]; the only build configuration is the default."
    COMBOS+=("--no-default-features")
    COMBOS+=("")            # default == same code
else
    n=${#FEATURES[@]}
    total=$((1 << n))
    for ((mask = 0; mask < total; mask++)); do
        combo=""
        for ((b = 0; b < n; b++)); do
            if (((mask >> b) & 1)); then combo="$combo,${FEATURES[b]}"; fi
        done
        COMBOS+=("--no-default-features --features ${combo#,}")
    done
    COMBOS+=("")            # plus the default feature set
fi

fail=0
for profile in release dev; do
    if [ "$profile" = release ]; then PFLAG=--release; else PFLAG=; fi
    for combo in "${COMBOS[@]}"; do
        label="profile=$profile features=[${combo:-<default>}]"
        echo "=============================================================="
        echo ">>> $label"
        # shellcheck disable=SC2086
        if ! cargo build --offline $PFLAG $combo >/dev/null 2>&1; then
            echo "    BUILD FAILED"
            fail=1
            continue
        fi
        # which Rust .so the tests will dlopen, so the label is verifiable
        if [ "$profile" = release ]; then SO=target/release/libdriver.so; else SO=target/debug/libdriver.so; fi
        if [ ! -f "$SO" ]; then
            echo "    MISSING $SO (tests would fall back to another profile)"
            fail=1
            continue
        fi
        echo "    Rust .so: $SO ($(nm -D --defined-only "$SO" | grep -c ' T ') exported T symbols)"
        # shellcheck disable=SC2086
        timeout 600 cargo test --offline $PFLAG $combo 2>&1 | grep -E "^test result|^error|panicked"
        if [ "${PIPESTATUS[0]}" -ne 0 ]; then
            echo "    TESTS FAILED for $label"
            fail=1
        fi
    done
done

echo "=============================================================="
if [ "$fail" -eq 0 ]; then
    echo "check_features: all profile x feature combinations PASSED"
else
    echo "check_features: FAILURES (see above)"
fi
exit "$fail"
