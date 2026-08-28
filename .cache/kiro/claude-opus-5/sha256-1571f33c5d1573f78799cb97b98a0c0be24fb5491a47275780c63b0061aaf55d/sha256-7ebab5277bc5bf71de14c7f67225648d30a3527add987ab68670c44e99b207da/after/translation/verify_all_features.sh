#!/usr/bin/env bash
# Enumerate every valid feature combination from translation/Cargo.toml and run
# `cargo check` + `cargo test` (debug and release) for each.
set -uo pipefail

cd "$(dirname "$0")" || exit 1

# --- enumerate features -----------------------------------------------------
mapfile -t FEATURES < <(python3 - <<'PY'
import re, sys
txt = open('Cargo.toml').read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', txt, re.M | re.S)
if not m:
    sys.exit(0)
for line in m.group(1).splitlines():
    line = line.split('#')[0].strip()
    if not line or '=' not in line:
        continue
    name = line.split('=', 1)[0].strip().strip('"')
    if name and name != 'default':
        print(name)
PY
)

echo "declared non-default features: ${#FEATURES[@]} ${FEATURES[*]-}"

# Build the list of combinations to test: the empty set plus every subset.
COMBOS=("")
n=${#FEATURES[@]}
if (( n > 0 )); then
    for (( mask=1; mask < (1<<n); mask++ )); do
        combo=""
        for (( i=0; i<n; i++ )); do
            if (( mask & (1<<i) )); then
                combo+="${combo:+,}${FEATURES[i]}"
            fi
        done
        COMBOS+=("$combo")
    done
fi
# Also cover the crate's own default feature set.
COMBOS+=("__DEFAULT__")

fail=0
for combo in "${COMBOS[@]}"; do
    if [[ "$combo" == "__DEFAULT__" ]]; then
        args=()
        label="(default features)"
        unset CRC16_TEST_NO_DEFAULT_FEATURES
        unset CRC16_TEST_FEATURES
    elif [[ -z "$combo" ]]; then
        args=(--no-default-features)
        label="(no-default-features, no features)"
        export CRC16_TEST_NO_DEFAULT_FEATURES=1
        unset CRC16_TEST_FEATURES
    else
        args=(--no-default-features --features "$combo")
        label="(no-default-features, features=$combo)"
        export CRC16_TEST_NO_DEFAULT_FEATURES=1
        export CRC16_TEST_FEATURES="$combo"
    fi

    for prof in "" --release; do
        pl=${prof:-debug}
        echo "=== cargo check ${args[*]-} $pl $label ==="
        if ! timeout 600 cargo check "${args[@]}" $prof --all-targets \
              > "/tmp/check_${pl//-/}_$$.log" 2>&1; then
            echo "CHECK FAILED $label $pl"; tail -30 "/tmp/check_${pl//-/}_$$.log"; fail=1; continue
        fi
        # Emit the cdylib for this exact configuration so the tests load a
        # matching .so through libloading.
        if ! timeout 600 cargo build --lib "${args[@]}" $prof \
              > "/tmp/build_${pl//-/}_$$.log" 2>&1; then
            echo "BUILD FAILED $label $pl"; tail -30 "/tmp/build_${pl//-/}_$$.log"; fail=1; continue
        fi
        echo "=== cargo test  ${args[*]-} $pl $label ==="
        if ! timeout 600 cargo test "${args[@]}" $prof > "/tmp/test_${pl//-/}_$$.log" 2>&1; then
            echo "TEST FAILED $label $pl"; tail -40 "/tmp/test_${pl//-/}_$$.log"; fail=1; continue
        fi
        grep -E '^test result:' "/tmp/test_${pl//-/}_$$.log"
    done
done

if (( fail )); then
    echo "RESULT: FAILURES"
    exit 1
fi
echo "RESULT: all feature combinations pass"
