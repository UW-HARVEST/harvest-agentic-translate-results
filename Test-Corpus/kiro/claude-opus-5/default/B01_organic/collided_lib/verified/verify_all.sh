#!/usr/bin/env bash
# Verify the Rust translation against the C reference for EVERY build-time
# configuration: all cargo feature combinations x both profiles.
#
# Usage: ./verify_all.sh
set -uo pipefail

cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"
LOGDIR=/tmp/verify-collided
mkdir -p "$LOGDIR"

# ---------------------------------------------------------------------------
# 1. Build the C reference shared library (the ground truth).
# ---------------------------------------------------------------------------
echo "== building C reference =="
( cd "$ROOT/c_src" && mkdir -p build && cd build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
    && cmake --build . ) > "$LOGDIR/c_build.log" 2>&1
if [ $? -ne 0 ]; then
    echo "FAIL: C build" ; tail -20 "$LOGDIR/c_build.log" ; exit 1
fi
C_SO=$(find "$ROOT/c_src/build" -name '*.so' | head -1)
echo "   C .so: $C_SO"

# ---------------------------------------------------------------------------
# 2. Enumerate every valid feature combination from Cargo.toml [features].
#    (Features named "default" are expanded by cargo itself and skipped here.)
# ---------------------------------------------------------------------------
mapfile -t FEATURES < <(
    awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/ {sub(/ *=.*/,""); gsub(/ /,""); if ($0 != "default" && $0 != "") print}' Cargo.toml
)
echo "== features declared: ${#FEATURES[@]} (${FEATURES[*]:-none}) =="

COMBOS=("")   # the empty combination: --no-default-features
n=${#FEATURES[@]}
if [ "$n" -gt 0 ]; then
    for ((mask = 1; mask < (1 << n); mask++)); do
        combo=""
        for ((i = 0; i < n; i++)); do
            if (( mask & (1 << i) )); then
                combo="${combo:+$combo,}${FEATURES[$i]}"
            fi
        done
        COMBOS+=("$combo")
    done
fi
# Also cover the crate's own default feature set as shipped.
COMBOS+=("__DEFAULT__")
echo "== combinations to verify: ${#COMBOS[@]} =="

fail=0
run() { # run <label> <logfile> <cmd...>
    local label=$1 log=$2; shift 2
    if timeout 600 "$@" > "$log" 2>&1; then
        echo "   PASS  $label"
    else
        echo "   FAIL  $label   (log: $log)"
        tail -25 "$log" | sed 's/^/         /'
        fail=1
    fi
}

for combo in "${COMBOS[@]}"; do
    if [ "$combo" = "__DEFAULT__" ]; then
        FLAGS=()
        label="default-features"
        slug="default"
        export HARNESS_CARGO_FEATURES=  # let the harness use plain `cargo build`
        unset HARNESS_CARGO_FEATURES
    else
        FLAGS=(--no-default-features)
        [ -n "$combo" ] && FLAGS+=(--features "$combo")
        label="no-default-features${combo:+ +[$combo]}"
        slug="nodefault${combo:+-${combo//,/_}}"
        export HARNESS_CARGO_FEATURES="$combo"
    fi

    echo "== configuration: $label =="
    run "cargo check       [$label]" "$LOGDIR/check-$slug.log"       cargo check "${FLAGS[@]}"
    run "cargo check tests [$label]" "$LOGDIR/checkt-$slug.log"      cargo check --all-targets "${FLAGS[@]}"
    run "cargo build dbg   [$label]" "$LOGDIR/build-$slug.log"       cargo build "${FLAGS[@]}"
    run "cargo build rel   [$label]" "$LOGDIR/buildrel-$slug.log"    cargo build --release "${FLAGS[@]}"

    # Symbol parity: every symbol the C .so exports must exist in the Rust .so.
    for profile in debug release; do
        R_SO="target/$profile/libcollided_lib.so"
        if [ ! -f "$R_SO" ]; then
            echo "   FAIL  missing $R_SO"; fail=1; continue
        fi
        missing=$(comm -23 \
            <(nm -D --defined-only "$C_SO"  | awk '$2 ~ /^[TtDdBbRrWi]$/ {print $3}' | grep -v '^_' | sort -u) \
            <(nm -D --defined-only "$R_SO" | awk '$2 ~ /^[TtDdBbRrWi]$/ {print $3}' | grep -v '^_' | sort -u))
        if [ -z "$missing" ]; then
            echo "   PASS  symbol parity ($profile) [$label]"
        else
            echo "   FAIL  symbol parity ($profile) [$label]: missing:"
            echo "$missing" | sed 's/^/         /'
            fail=1
        fi
    done

    run "cargo test dbg    [$label]" "$LOGDIR/test-$slug.log"        cargo test "${FLAGS[@]}"
    run "cargo test rel    [$label]" "$LOGDIR/testrel-$slug.log"     cargo test --release "${FLAGS[@]}"
done

echo
if [ "$fail" -eq 0 ]; then
    echo "ALL CONFIGURATIONS VERIFIED AGAINST C"
else
    echo "FAILURES PRESENT - see logs in $LOGDIR"
fi
exit "$fail"
