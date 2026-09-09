#!/bin/bash
# Full verification driver: symbol parity + every feature combination.
#
# Usage: ./verify.sh
#
# Phase D of the verification plan. Prints a summary and exits non-zero if any
# check fails.
set -uo pipefail
cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"
LOGS="$ROOT/_logs"
mkdir -p "$LOGS"
fail=0

say() { printf '\n=== %s ===\n' "$1"; }

# --------------------------------------------------------------------------
say "Building C reference library"
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >"$LOGS/cmake.log" 2>&1 \
  && cmake --build . -j8 >"$LOGS/cbuild.log" 2>&1 ) \
  || { echo "C build FAILED (see $LOGS/cbuild.log)"; exit 1; }
CSO="$ROOT/c_src/build/libsodium.so"
ls -l "$CSO"

# --------------------------------------------------------------------------
# Enumerate feature combinations declared in Cargo.toml. The crate declares no
# [features] section, so the only configuration is the default one; the loop is
# written generically so it keeps working if features are ever added.
say "Feature combinations"
FEATURES=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/ {split($0,a,"="); gsub(/[ \t]/,"",a[1]); if (a[1] != "default") print a[1]}' Cargo.toml)
if [ -z "$FEATURES" ]; then
    echo "no [features] declared -> single configuration (default)"
    COMBOS=("default")
else
    echo "features: $FEATURES"
    COMBOS=("default" "none")
    for f in $FEATURES; do COMBOS+=("$f"); done
    COMBOS+=("$(echo $FEATURES | tr ' ' ',')")
fi

run_combo() {
    local combo="$1" flags=()
    case "$combo" in
        default) flags=() ;;
        none)    flags=(--no-default-features) ;;
        *)       flags=(--no-default-features --features "$combo") ;;
    esac
    say "combo: $combo"
    cargo build --offline --release "${flags[@]}" >"$LOGS/build_$combo.log" 2>&1 \
        || { echo "  BUILD FAILED (see $LOGS/build_$combo.log)"; fail=1; return; }
    RSO="target/release/liblibsodium.so"

    # ---- symbol parity -------------------------------------------------
    nm -D --defined-only "$CSO" | awk '{print $3}' | sort -u >"$LOGS/c_syms.txt"
    nm -D --defined-only "$RSO" | awk '{print $3}' | sort -u >"$LOGS/r_syms_$combo.txt"
    comm -23 "$LOGS/c_syms.txt" "$LOGS/r_syms_$combo.txt" >"$LOGS/missing_$combo.txt"
    comm -13 "$LOGS/c_syms.txt" "$LOGS/r_syms_$combo.txt" >"$LOGS/extra_$combo.txt"
    local nmiss nextra
    nmiss=$(wc -l <"$LOGS/missing_$combo.txt")
    nextra=$(wc -l <"$LOGS/extra_$combo.txt")
    echo "  C symbols:    $(wc -l <"$LOGS/c_syms.txt")"
    echo "  Rust symbols: $(wc -l <"$LOGS/r_syms_$combo.txt")"
    echo "  missing from Rust: $nmiss"
    echo "  extra in Rust:     $nextra"
    if [ "$nmiss" -ne 0 ]; then
        echo "  MISSING SYMBOLS:"; sed 's/^/    /' "$LOGS/missing_$combo.txt"; fail=1
    fi

    # undefined symbols must all be libc / compiler runtime
    nm -D --undefined-only "$RSO" | awk '{print $2}' | sed 's/@.*//' | sort -u \
        | grep -vE '^(_ITM_|__cxa_|__errno_location|__gmon_start__|__tls_get_addr|_Unwind_)' \
        | grep -E '^(crypto_|sodium_|randombytes|argon2|escrypt|blake2b)' >"$LOGS/undef_$combo.txt"
    if [ -s "$LOGS/undef_$combo.txt" ]; then
        echo "  UNDEFINED libsodium symbols in Rust .so:"; sed 's/^/    /' "$LOGS/undef_$combo.txt"; fail=1
    else
        echo "  undefined non-libc libsodium symbols: 0"
    fi

    # ---- differential tests --------------------------------------------
    # Run each integration-test binary separately so no single command can run
    # long enough to hit a CI timeout, and so a failure names its suite.
    : >"$LOGS/test_$combo.log"
    for f in tests/t*.rs; do
        local t; t="$(basename "$f" .rs)"
        timeout 600 cargo test --offline --release "${flags[@]}" --test "$t" \
            -- --test-threads=1 >"$LOGS/test_${combo}_$t.log" 2>&1
        local rc=$?
        local line; line="$(grep -E '^test result:' "$LOGS/test_${combo}_$t.log" | tail -1)"
        printf '  %-24s %s\n' "$t" "${line:-<no result line>}"
        cat "$LOGS/test_${combo}_$t.log" >>"$LOGS/test_$combo.log"
        if [ $rc -ne 0 ]; then
            echo "    SUITE FAILED (see $LOGS/test_${combo}_$t.log)"
            grep -E '^test .* FAILED|panicked at' "$LOGS/test_${combo}_$t.log" \
                | head -20 | sed 's/^/      /'
            fail=1
        fi
    done
}

for c in "${COMBOS[@]}"; do run_combo "$c"; done

say "RESULT"
if [ "$fail" -eq 0 ]; then echo "ALL CHECKS PASSED"; else echo "FAILURES PRESENT"; fi
exit $fail
