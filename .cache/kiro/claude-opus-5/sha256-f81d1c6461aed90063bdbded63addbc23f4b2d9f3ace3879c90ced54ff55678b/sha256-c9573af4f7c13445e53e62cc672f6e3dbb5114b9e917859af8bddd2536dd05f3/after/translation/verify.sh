#!/usr/bin/env bash
# Runs the full differential suite across every buildable configuration.
#
#   * every cargo feature combination declared in Cargo.toml (there are none,
#     so this reduces to default + --no-default-features), and
#   * both cargo profiles, because the debug profile enables overflow checks
#     that the release profile does not — the INT_MAX wrap must not panic in
#     either.
#
# Usage: ./verify.sh
set -uo pipefail
cd "$(dirname "$0")"

C_SO_PATH="$(cd .. && pwd)/c_src/build/libSieve.so"
if [[ ! -f "$C_SO_PATH" ]]; then
    echo "building the C library first"
    (cd ../c_src && mkdir -p build && cd build \
        && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
        && cmake --build . >/dev/null) || exit 1
fi

# --- enumerate feature combinations mechanically from Cargo.toml -------------
mapfile -t FEATURES < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/ {split($0,a,"="); gsub(/ /,"",a[1]); if (a[1]!="default") print a[1]}' Cargo.toml
)
COMBOS=("default")
if (( ${#FEATURES[@]} > 0 )); then
    COMBOS+=("--no-default-features")
    for f in "${FEATURES[@]}"; do
        COMBOS+=("--no-default-features --features $f")
    done
    # all features together
    COMBOS+=("--all-features")
else
    echo "Cargo.toml declares no [features]; the only configurations are the"
    echo "default one (identical to --no-default-features) and the two profiles."
    COMBOS+=("--no-default-features")
fi

FAIL=0
for profile in release debug; do
  for combo in "${COMBOS[@]}"; do
    flags=""
    [[ "$combo" != "default" ]] && flags="$combo"
    profile_flag=""
    [[ "$profile" == "release" ]] && profile_flag="--release"

    echo "=============================================================="
    echo "profile=$profile  features=$combo"
    echo "=============================================================="

    # shellcheck disable=SC2086
    timeout 300 cargo build $profile_flag $flags >/dev/null 2>&1 || {
        echo "BUILD FAILED"; FAIL=1; continue; }

    RUST_SO="$(pwd)/target/$profile/libSieve.so"
    [[ -f "$RUST_SO" ]] || { echo "missing $RUST_SO"; FAIL=1; continue; }

    # Symbol parity for this exact artifact.
    missing=$(comm -23 \
        <(nm -D --defined-only "$C_SO_PATH"  | awk '{print $NF}' | grep -v '^_ITM_\|^__gmon_start__$\|^__cxa_finalize' | sort -u) \
        <(nm -D --defined-only "$RUST_SO"    | awk '{print $NF}' | sed 's/@.*//' | sort -u))
    if [[ -n "$missing" ]]; then
        echo "SYMBOL DIFF NOT EMPTY: $missing"; FAIL=1
    else
        echo "symbol diff: empty"
    fi

    # The tests themselves always run optimized (they are the harness, not the
    # subject); RUST_SO selects which artifact is under test.
    # shellcheck disable=SC2086
    C_SO="$C_SO_PATH" RUST_SO="$RUST_SO" \
        timeout 600 cargo test --release $flags -- --test-threads=1 2>&1 \
        | grep -E '^(test |test result|error|running)' | tail -45
    # shellcheck disable=SC2181
    if [[ ${PIPESTATUS[0]} -ne 0 ]]; then FAIL=1; fi
  done
done

echo
if (( FAIL == 0 )); then
    echo "ALL CONFIGURATIONS PASSED"
else
    echo "SOME CONFIGURATIONS FAILED"
fi
exit $FAIL
