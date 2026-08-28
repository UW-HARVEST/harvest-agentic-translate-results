#!/usr/bin/env bash
# Verify the Rust translation against the C ground truth for EVERY valid
# build-time configuration.
#
#   ./verify_all.sh
#
# Steps
#   1. enumerate every feature combination declared in Cargo.toml
#   2. `cargo check` each combination
#   3. build the C shared libraries (default CMake configuration)
#   4. `cargo test` each combination (the differential tests dlopen both the C
#      and the Rust .so and compare their observable behaviour)
#   5. compare exported symbols: every symbol the C .so exports must also be
#      exported by the Rust .so
set -u -o pipefail

cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"
LOG_DIR="$(mktemp -d)"
FAILED=0
TIMEOUT=600

note() { printf '\n=== %s ===\n' "$*"; }
fail() { printf 'FAIL: %s\n' "$*"; FAILED=1; }

# ---------------------------------------------------------------------------
# 1. enumerate feature combinations (powerset of the [features] table)
# ---------------------------------------------------------------------------
mapfile -t FEATURES < <(
    awk '
        /^\[features\]/ { in_f = 1; next }
        /^\[/           { in_f = 0 }
        in_f && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
            split($0, a, "=");
            gsub(/[[:space:]]/, "", a[1]);
            if (a[1] != "default") print a[1];
        }
    ' Cargo.toml
)

COMBOS=()
if [ "${#FEATURES[@]}" -eq 0 ]; then
    note "Cargo.toml declares no [features]: the crate has exactly one configuration"
    COMBOS=("")
else
    n=${#FEATURES[@]}
    for ((mask = 0; mask < (1 << n); mask++)); do
        combo=""
        for ((i = 0; i < n; i++)); do
            if (((mask >> i) & 1)); then
                combo="${combo:+$combo,}${FEATURES[i]}"
            fi
        done
        COMBOS+=("$combo")
    done
    note "feature combinations to verify: ${#COMBOS[@]}"
    printf '  - %s\n' "${COMBOS[@]/#/features=}"
fi

# ---------------------------------------------------------------------------
# 2. cargo check every combination
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
    label="${combo:-<default>}"
    note "cargo check --no-default-features --features '$combo'"
    if timeout "$TIMEOUT" cargo check --no-default-features --features "$combo" \
        >"$LOG_DIR/check.log" 2>&1; then
        echo "ok"
    else
        tail -30 "$LOG_DIR/check.log"
        fail "cargo check failed for $label"
    fi
done

# ---------------------------------------------------------------------------
# 3. build the C shared libraries
# ---------------------------------------------------------------------------
note "building the C shared libraries"
mkdir -p "$ROOT/c_src/build"
(
    cd "$ROOT/c_src/build" &&
        cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON &&
        cmake --build .
) >"$LOG_DIR/cmake.log" 2>&1 || {
    tail -30 "$LOG_DIR/cmake.log"
    fail "C build failed"
}
ls -1 "$ROOT/c_src/build/"*.so* 2>/dev/null || fail "no C .so produced"

# ---------------------------------------------------------------------------
# 4. differential tests for every combination
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
    label="${combo:-<default>}"
    note "cargo test --no-default-features --features '$combo'"
    if timeout "$TIMEOUT" cargo test --no-default-features --features "$combo" \
        >"$LOG_DIR/test.log" 2>&1; then
        grep -E '^test result:' "$LOG_DIR/test.log"
    else
        grep -E '^(test |error|assertion|thread)' "$LOG_DIR/test.log" | head -40
        fail "cargo test failed for $label"
    fi
done

# ---------------------------------------------------------------------------
# 5. exported symbol comparison
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
    label="${combo:-<default>}"
    note "exported symbols (features='$combo')"
    timeout "$TIMEOUT" cargo build --release --no-default-features --features "$combo" \
        >"$LOG_DIR/build.log" 2>&1 || {
        tail -30 "$LOG_DIR/build.log"
        fail "cargo build --release failed for $label"
        continue
    }

    RUST_SO="target/release/libcJSON_test.so"
    # The Rust cdylib contains the translation of BOTH cJSON.c and test.c, so
    # it must export the union of the two C libraries' symbols.
    {
        nm -D --defined-only "$ROOT/c_src/build/libcjson.so.1.7.19"
        nm -D --defined-only "$ROOT/c_src/build/libcJSON_test.so"
    } | awk '{print $3}' | grep -v '^$' | sort -u >"$LOG_DIR/c_syms.txt"
    nm -D --defined-only "$RUST_SO" | awk '{print $3}' | grep -v '^$' | sort -u \
        >"$LOG_DIR/rust_syms.txt"

    missing="$(comm -23 "$LOG_DIR/c_syms.txt" "$LOG_DIR/rust_syms.txt")"
    if [ -n "$missing" ]; then
        echo "symbols exported by the C .so but missing from the Rust .so:"
        echo "$missing" | sed 's/^/  /'
        fail "missing exports for $label"
    else
        echo "all $(wc -l <"$LOG_DIR/c_syms.txt") C symbols are exported by the Rust .so"
    fi
done

note "summary"
if [ "$FAILED" -eq 0 ]; then
    echo "ALL CHECKS PASSED"
else
    echo "THERE WERE FAILURES (logs in $LOG_DIR)"
fi
exit "$FAILED"
