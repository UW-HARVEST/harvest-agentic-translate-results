#!/usr/bin/env bash
# Verify the Rust translation against the C ground truth across every
# build-time configuration.
#
#  * Feature combinations are read out of Cargo.toml. This crate declares no
#    [features], so the only valid combinations are the empty set, exercised
#    both with and without --no-default-features.
#  * Both cargo profiles (dev and release) are covered: release enables
#    optimisation and panic = "abort", which is a different code path.
#  * The C side is additionally rebuilt at -O0/-O2/-O3 because the original
#    relies on signed-overflow wraparound, which optimisers may treat
#    differently; the Rust translation must match all of them.
set -uo pipefail

cd "$(dirname "$0")" || exit 1
ROOT="$(cd .. && pwd)"
FAILED=0

echo "=== declared cargo features ==="
if awk '/^\[features\]/{f=1;next} /^\[/{f=0} f&&NF' Cargo.toml | grep -q .; then
    awk '/^\[features\]/{f=1;next} /^\[/{f=0} f&&NF' Cargo.toml
else
    echo "(none declared -> only the empty feature set is valid)"
fi

# Feature combinations: the empty set only. Kept as a list so that adding a
# feature to Cargo.toml only requires extending this array.
COMBOS=("" "--no-default-features")

echo
echo "=== cargo check, every feature combination x profile ==="
for combo in "${COMBOS[@]}"; do
    for prof in "" "--release"; do
        label="check [${combo:-default}] [${prof:-dev}]"
        if timeout 600 cargo check $combo $prof --all-targets >/tmp/check.log 2>&1; then
            echo "PASS  $label"
        else
            echo "FAIL  $label"; tail -20 /tmp/check.log; FAILED=1
        fi
    done
done

echo
echo "=== cargo test, every feature combination x profile ==="
for combo in "${COMBOS[@]}"; do
    for prof in "" "--release"; do
        label="test  [${combo:-default}] [${prof:-dev}]"
        export HARVEST_CARGO_ARGS="$combo"
        if timeout 600 cargo test $combo $prof >/tmp/test.log 2>&1; then
            echo "PASS  $label  ($(grep -c '^test .* ok$' /tmp/test.log) test cases)"
        else
            echo "FAIL  $label"
            grep -E "^test .* FAILED|differing line|panicked at|assertion" /tmp/test.log | head -20
            FAILED=1
        fi
        unset HARVEST_CARGO_ARGS
    done
done

echo
echo "=== C optimisation levels (signed-overflow / UB sensitivity) ==="
for opt in -O0 -O2 -O3; do
    bdir="/tmp/c_build$opt"
    rm -rf "$bdir"
    if ! cmake -S "$ROOT/c_src" -B "$bdir" \
            -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
            -DCMAKE_C_FLAGS="$opt" >/tmp/cmake_$opt.log 2>&1 \
        || ! cmake --build "$bdir" >>/tmp/cmake_$opt.log 2>&1; then
        echo "FAIL  building C at $opt"; tail -10 /tmp/cmake_$opt.log; FAILED=1; continue
    fi
    so="$(find "$bdir" -maxdepth 1 -name '*.so' | head -1)"
    for prof in "" "--release"; do
        label="test  [C $opt] [${prof:-dev}]"
        if HARVEST_C_SO="$so" timeout 600 cargo test $prof >/tmp/test_$opt.log 2>&1; then
            echo "PASS  $label"
        else
            echo "FAIL  $label"
            grep -E "^test .* FAILED|differing line|panicked at|assertion" /tmp/test_$opt.log | head -20
            FAILED=1
        fi
    done
done

echo
if [ "$FAILED" -eq 0 ]; then
    echo "ALL CONFIGURATIONS PASS"
else
    echo "SOME CONFIGURATIONS FAILED"
fi
exit "$FAILED"
