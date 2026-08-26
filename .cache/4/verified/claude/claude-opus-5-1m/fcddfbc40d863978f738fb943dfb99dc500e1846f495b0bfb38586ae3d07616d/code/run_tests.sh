#!/usr/bin/env bash
# Differential test runner.
#
# `cargo test` does NOT rebuild a `crate-type = ["cdylib"]` target, so the
# shared object under test must be built explicitly first. This script also
# loops over every valid feature combination (Cargo.toml declares no
# [features], so the only combination is the empty one).
set -uo pipefail

cd "$(dirname "$0")" || exit 1

PROFILE_ARGS=()
PROFILE_DIR=debug
if [[ "${1:-}" == "--release" ]]; then
    PROFILE_ARGS=(--release)
    PROFILE_DIR=release
    shift
fi

# --- 1. C reference shared library -----------------------------------------
if [[ ! -f c_src/build/libtranslated_rust.so ]]; then
    echo "=== building C reference .so ==="
    mkdir -p c_src/build
    (cd c_src/build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null &&
        cmake --build . >/dev/null) || exit 1
fi

# --- 2. feature combinations ------------------------------------------------
# Enumerated mechanically from Cargo.toml's [features] table (there is none),
# so the single valid combination is "no features".
COMBOS=("")

rc=0
for combo in "${COMBOS[@]}"; do
    label="${combo:-<none>}"
    echo "############################################################"
    echo "### features: ${label}   profile: ${PROFILE_DIR}"
    echo "############################################################"

    FEATFLAGS=(--no-default-features)
    [[ -n "$combo" ]] && FEATFLAGS+=(--features "$combo")

    echo "--- cargo check ---"
    cargo check --offline "${FEATFLAGS[@]}" "${PROFILE_ARGS[@]}" --tests 2>&1 | tail -5 || rc=1

    echo "--- cargo build (cdylib must be fresh for the tests) ---"
    cargo build --offline "${FEATFLAGS[@]}" "${PROFILE_ARGS[@]}" 2>&1 | tail -5 || rc=1
    # The crash-parity tests in tests/phase_c_errors.rs always load the RELEASE
    # cdylib (a debug build turns NULL/misaligned raw dereferences into a Rust
    # panic + abort instead of a fault), so it must always be up to date.
    cargo build --offline "${FEATFLAGS[@]}" --release 2>&1 | tail -3 || rc=1

    echo "--- symbol parity ---"
    nm -D --defined-only c_src/build/libtranslated_rust.so | awk '{print $3}' | sort -u \
        >"${TMPDIR:-/tmp}/c_names.txt"
    nm -D --defined-only "target/${PROFILE_DIR}/libstr_put_lib.so" | awk '{print $3}' | sort -u \
        >"${TMPDIR:-/tmp}/r_names.txt"
    missing=$(comm -23 "${TMPDIR:-/tmp}/c_names.txt" "${TMPDIR:-/tmp}/r_names.txt")
    extra=$(comm -13 "${TMPDIR:-/tmp}/c_names.txt" "${TMPDIR:-/tmp}/r_names.txt")
    if [[ -n "$missing" ]]; then
        echo "MISSING FROM RUST .so:"
        echo "$missing"
        rc=1
    fi
    if [[ -n "$extra" ]]; then
        echo "EXTRA IN RUST .so:"
        echo "$extra"
    fi
    [[ -z "$missing" && -z "$extra" ]] && echo "symbol diff empty ($(wc -l <"${TMPDIR:-/tmp}/c_names.txt") symbols)"

    echo "--- cargo test ---"
    log="${TMPDIR:-/tmp}/difftest-${PROFILE_DIR}.log"
    if cargo test --offline "${FEATFLAGS[@]}" "${PROFILE_ARGS[@]}" \
            -- --test-threads=1 "$@" >"$log" 2>&1; then
        :
    else
        rc=1
    fi
    # Full output is in $log; print every binary, its result line, and any
    # failures (never truncate the part that says whether a suite even ran).
    grep -E '^ *Running |^test result:|^test .* FAILED|^---- |^error' "$log" || true
    total=$(grep -cE '^test [a-z].* \.\.\. ok$' "$log" || true)
    fails=$(grep -cE 'FAILED' "$log" || true)
    echo "=> ${total} passing test functions, ${fails} failures  (full log: $log)"
    [[ "$fails" != "0" ]] && rc=1
done

echo
if [[ $rc -eq 0 ]]; then echo "ALL GREEN"; else echo "FAILURES (rc=$rc)"; fi
exit $rc
