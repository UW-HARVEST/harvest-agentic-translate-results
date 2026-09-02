#!/usr/bin/env bash
# Differential verification driver.
#
# IMPORTANT: `cargo test` does NOT rebuild the `cdylib`, because the
# integration tests never link against it (they `dlopen` it). So the library
# MUST be built explicitly first, otherwise the tests silently run against a
# stale `.so`.
set -uo pipefail

cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"
C_BUILD="$ROOT/c_src/build"

TESTS_FILTER="${TESTS_FILTER:-}"
PROFILE_FLAG="--release"
TIMEOUT="${TIMEOUT:-600}"

# ---------------------------------------------------------------- C reference
if [ ! -d "$C_BUILD" ] || ! ls "$C_BUILD"/*.so >/dev/null 2>&1; then
    echo "== building C reference =="
    mkdir -p "$C_BUILD"
    ( cd "$C_BUILD" \
      && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
      && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
fi
C_SO="$(ls "$C_BUILD"/*.so | head -1)"
echo "C   .so: $C_SO"

# ------------------------------------------------- feature combinations
# Enumerate every feature combination declared in Cargo.toml. If there is no
# [features] section there is exactly one combination: the default.
mapfile -t FEATURES < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /^[A-Za-z0-9_-]+[[:space:]]*=/{
        sub(/[[:space:]]*=.*/,""); print }' Cargo.toml
)

COMBOS=()
if [ "${#FEATURES[@]}" -eq 0 ]; then
    COMBOS+=("default:")
    COMBOS+=("no-default:--no-default-features")
else
    COMBOS+=("default:")
    COMBOS+=("no-default:--no-default-features")
    n=${#FEATURES[@]}
    for (( mask=1; mask < (1<<n); mask++ )); do
        combo=""
        for (( i=0; i<n; i++ )); do
            if (( mask & (1<<i) )); then combo="$combo,${FEATURES[$i]}"; fi
        done
        combo="${combo#,}"
        COMBOS+=("$combo:--no-default-features --features $combo")
    done
fi

echo "feature combinations: ${#COMBOS[@]}"
for c in "${COMBOS[@]}"; do echo "  - ${c%%:*}"; done

FAIL=0
for entry in "${COMBOS[@]}"; do
    name="${entry%%:*}"
    flags="${entry#*:}"
    echo
    echo "================================================================"
    echo "== FEATURE COMBO: $name   (cargo flags: '${flags:-<none>}')"
    echo "================================================================"

    # shellcheck disable=SC2086
    if ! timeout "$TIMEOUT" cargo build $PROFILE_FLAG $flags 2>&1 | tail -3; then
        echo "!! cargo build FAILED for combo $name"; FAIL=1; continue
    fi
    RUST_SO="$PWD/target/release/libpoly_ray_lib.so"
    if [ ! -f "$RUST_SO" ]; then echo "!! no $RUST_SO"; FAIL=1; continue; fi
    echo "RUST .so: $RUST_SO"

    # -------- symbol parity (Phase D) for this combo
    nm -D --defined-only "$C_SO"   | awk '$2=="T"||$2=="W"{print $3}' | sort > /tmp/c_syms.txt
    nm -D --defined-only "$RUST_SO" | awk '$2=="T"||$2=="W"{print $3}' | sort > /tmp/r_syms.txt
    MISSING="$(comm -23 /tmp/c_syms.txt /tmp/r_syms.txt)"
    if [ -n "$MISSING" ]; then
        echo "!! SYMBOLS MISSING FROM RUST .so:"; echo "$MISSING" | sed 's/^/     /'
        FAIL=1
    else
        echo "symbol parity: OK ($(wc -l < /tmp/c_syms.txt) C symbols, 0 missing)"
    fi

    # -------- differential tests
    export C_SO_PATH="$C_SO"
    export RUST_SO_PATH="$RUST_SO"
    LOG="$(mktemp)"
    # shellcheck disable=SC2086
    timeout "$TIMEOUT" cargo test $PROFILE_FLAG $flags $TESTS_FILTER \
            -- --test-threads=4 >"$LOG" 2>&1
    rc=$?
    grep -E "^test result:|DIVERGENCE|panicked at|^error" "$LOG" | sed 's/^/    /'
    awk -F'[ ;]' '/^test result:/{p+=$4; f+=$6} END{
        printf "    TOTAL: %d passed, %d failed\n", p, f }' "$LOG"
    if [ "$rc" -ne 0 ]; then
        echo "!! TESTS FAILED for combo $name (see $LOG)"; FAIL=1
    else
        rm -f "$LOG"
    fi
done

echo
if [ "$FAIL" -eq 0 ]; then
    echo "########## ALL COMBOS PASSED ##########"
else
    echo "########## FAILURES PRESENT ##########"
fi
exit "$FAIL"
