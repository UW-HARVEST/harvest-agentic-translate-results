#!/bin/bash
# Full verification run: builds the C shared library, enumerates every valid
# Cargo feature combination, and runs `cargo check` + the differential test
# suite for each one.
#
# Usage: ./run_tests.sh
set -u
cd "$(dirname "$0")" || exit 1

LOG_DIR="${TMPDIR:-/tmp}"
rc=0

echo "=== 1. Enumerating feature combinations from Cargo.toml ==="
# Every feature declared in the [features] table (excluding "default").
mapfile -t FEATURES < <(
    awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/ {split($0,a,"="); gsub(/[ \t]/,"",a[1]); if (a[1] != "default" && a[1] != "") print a[1]}' Cargo.toml
)
if [ "${#FEATURES[@]}" -eq 0 ]; then
    echo "   no [features] table -> exactly one combination (the empty set)"
    COMBOS=("")
else
    # Power set of the declared features.
    COMBOS=("")
    n=${#FEATURES[@]}
    total=$((1 << n))
    COMBOS=()
    for ((mask = 0; mask < total; mask++)); do
        combo=""
        for ((i = 0; i < n; i++)); do
            if (((mask >> i) & 1)); then
                [ -n "$combo" ] && combo="$combo,"
                combo="$combo${FEATURES[$i]}"
            fi
        done
        COMBOS+=("$combo")
    done
    printf '   features: %s\n' "${FEATURES[*]}"
    echo "   ${#COMBOS[@]} combination(s)"
fi

echo
echo "=== 2. Building the C shared library ==="
(
    mkdir -p c_src/build &&
        cd c_src/build &&
        cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON &&
        cmake --build .
) >"$LOG_DIR/c_build.log" 2>&1
if [ $? -ne 0 ]; then
    echo "   FAILED (see $LOG_DIR/c_build.log)"
    tail -20 "$LOG_DIR/c_build.log"
    exit 1
fi
ls -l c_src/build/*.so

echo
for combo in "${COMBOS[@]}"; do
    if [ -z "$combo" ]; then
        label="<no features>"
        feat_args=(--no-default-features)
    else
        label="$combo"
        feat_args=(--no-default-features --features "$combo")
    fi

    echo "=== 3. cargo check  [$label] ==="
    if timeout 600 cargo check "${feat_args[@]}" >"$LOG_DIR/check.log" 2>&1; then
        echo "   OK"
    else
        echo "   FAILED"
        tail -40 "$LOG_DIR/check.log"
        rc=1
        continue
    fi

    # `cargo test` does NOT rebuild a `crate-type = ["cdylib"]` library when no
    # test target links against it, so build the .so explicitly first. The test
    # harness also refuses to run against a stale .so.
    echo "=== 4. cargo build (cdylib)  [$label] ==="
    if timeout 600 cargo build "${feat_args[@]}" >"$LOG_DIR/build.log" 2>&1; then
        echo "   OK"
    else
        echo "   FAILED"
        tail -40 "$LOG_DIR/build.log"
        rc=1
        continue
    fi

    echo "=== 5. differential tests  [$label] ==="
    # --test-threads=1: the harness redirects fd 1 to capture the libraries'
    # stdout, which cannot be done concurrently.
    if timeout 600 cargo test "${feat_args[@]}" -- --test-threads=1 \
        >"$LOG_DIR/test.log" 2>&1; then
        grep -E '^test result:' "$LOG_DIR/test.log" | sed 's/^/   /'
    else
        echo "   FAILED"
        grep -E '^(test .* FAILED|test result:|---- )' "$LOG_DIR/test.log" | head -60
        rc=1
    fi

    # The optimiser can legitimately change UB-adjacent code (the float->int
    # cast, wrapping arithmetic), so verify the optimised cdylib as well.
    echo "=== 6. differential tests, optimised (--release)  [$label] ==="
    if timeout 600 cargo build --release "${feat_args[@]}" >"$LOG_DIR/build_rel.log" 2>&1 &&
        timeout 600 cargo test --release "${feat_args[@]}" -- --test-threads=1 \
            >"$LOG_DIR/test_rel.log" 2>&1; then
        grep -E '^test result:' "$LOG_DIR/test_rel.log" | sed 's/^/   /'
    else
        echo "   FAILED"
        grep -E '^(test .* FAILED|test result:|---- |error)' "$LOG_DIR/test_rel.log" | head -60
        rc=1
    fi
    echo
done

echo "==============================================="
if [ "$rc" -eq 0 ]; then
    echo "ALL FEATURE COMBINATIONS PASSED"
else
    echo "FAILURES DETECTED"
fi
exit "$rc"
