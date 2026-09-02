#!/usr/bin/env bash
# Full differential verification driver.
#
# Two things `cargo test` alone gets wrong for this crate, both of which would
# silently make the suite pass without testing anything:
#
#   1. `cargo test` does NOT rebuild a `cdylib` artifact, so the tests would
#      dlopen a stale `target/*/libString_Slice.so`. We build explicitly.
#   2. The tests redirect the process-wide stdout fd, so they must run
#      single-threaded or libtest's own progress output pollutes the captures.
#
# Usage: ./verify.sh
set -euo pipefail
cd "$(dirname "$0")"

ROOT="$(cd .. && pwd)"
C_SO="$ROOT/c_src/build/libString_Slice.so"

echo "== building the C reference library"
if [ ! -f "$C_SO" ]; then
    ( cd "$ROOT/c_src" && mkdir -p build && cd build \
        && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
        && cmake --build . )
fi
test -f "$C_SO" || { echo "FAIL: $C_SO not built"; exit 1; }

# Enumerate every feature combination declared in Cargo.toml. This crate has no
# [features] section, so the list collapses to the default/empty configuration,
# but the loop is written generically so it keeps working if features are added.
mapfile -t FEATURES < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/{ gsub(/ /,""); split($0,a,"="); if (a[1] != "default") print a[1] }' Cargo.toml
)
echo "== declared cargo features: ${#FEATURES[@]} ${FEATURES[*]:-(none)}"

# Build the combination list: default, --no-default-features, then each feature
# individually and all of them together (when any exist).
COMBOS=("default" "none")
if [ "${#FEATURES[@]}" -gt 0 ]; then
    for f in "${FEATURES[@]}"; do COMBOS+=("$f"); done
    ALL=$(IFS=, ; echo "${FEATURES[*]}")
    COMBOS+=("$ALL")
fi

echo "== cargo check across ${#COMBOS[@]} combination(s)"
for combo in "${COMBOS[@]}"; do
    case "$combo" in
        default) args=() ;;
        none)    args=(--no-default-features) ;;
        *)       args=(--no-default-features --features "$combo") ;;
    esac
    printf '   check [%s] ... ' "$combo"
    timeout 600 cargo check "${args[@]}" >/dev/null 2>&1 && echo ok || { echo FAIL; exit 1; }
done

FAILED=0
for combo in "${COMBOS[@]}"; do
    case "$combo" in
        default) args=() ;;
        none)    args=(--no-default-features) ;;
        *)       args=(--no-default-features --features "$combo") ;;
    esac
    for profile in debug release; do
        rel=()
        [ "$profile" = release ] && rel=(--release)

        echo
        echo "=================================================================="
        echo "== features=[$combo] profile=$profile"
        echo "=================================================================="

        # 1. Build the cdylib for real (cargo test will not do this).
        timeout 600 cargo build "${args[@]}" "${rel[@]}" 2>&1 | tail -3
        SO="target/$profile/libString_Slice.so"
        test -f "$SO" || { echo "FAIL: $SO missing"; exit 1; }

        # 2. Symbol diff, independent of the in-test assertion.
        echo "-- nm -D symbol diff (C vs Rust), must be empty:"
        if diff <(nm -D --defined-only "$C_SO" | awk '{print $NF}' | sed 's/@.*//' | sort) \
                <(nm -D --defined-only "$SO"  | awk '{print $NF}' | sed 's/@.*//' | sort -u) \
                | grep '^<' ; then
            echo "FAIL: symbols exported by C are missing from Rust"
            FAILED=1
        else
            echo "   (no C symbol missing from the Rust .so)"
        fi

        # 3. Run the differential suite against exactly this .so, serialized.
        LOG="target/verify-${combo//,/_}-$profile.log"
        if SLICE_RUST_SO="$PWD/$SO" RUST_TEST_THREADS=1 \
             timeout 600 cargo test "${args[@]}" -- --test-threads=1 > "$LOG" 2>&1 ; then
            grep -E '^test result' "$LOG" | sed 's/^/   /'
        else
            echo "FAIL: differential suite failed for features=[$combo] profile=$profile"
            grep -E '^(test result|---- |thread .* panicked|assertion)' "$LOG" | head -40
            FAILED=1
        fi
    done
done

echo
if [ "$FAILED" -eq 0 ]; then
    echo "ALL COMBINATIONS PASSED"
else
    echo "THERE WERE FAILURES"
fi
exit "$FAILED"
