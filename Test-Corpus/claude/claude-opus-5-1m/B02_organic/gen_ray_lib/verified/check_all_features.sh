#!/usr/bin/env bash
# Phase A/D: enumerate EVERY valid feature combination from Cargo.toml and
# `cargo check` each one, then run the full differential suite for each
# combination in both the dev and the release profile.
set -uo pipefail
cd "$(dirname "$0")"

# --- Enumerate the [features] table --------------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ {inside=1; next}
    /^\[/           {inside=0}
    inside && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
        sub(/[[:space:]]*=.*/, "", $0); print $0
    }
  ' Cargo.toml
)

echo "=== features declared in Cargo.toml: ${#FEATURES[@]} ==="
for f in "${FEATURES[@]:-}"; do echo "  - $f"; done

# Build the power set of feature names (empty set == --no-default-features).
COMBOS=("")
n=${#FEATURES[@]}
if [ "$n" -gt 0 ]; then
    for ((mask=1; mask<(1<<n); mask++)); do
        combo=""
        for ((i=0; i<n; i++)); do
            if (( mask & (1<<i) )); then
                combo="${combo:+$combo,}${FEATURES[$i]}"
            fi
        done
        COMBOS+=("$combo")
    done
fi

echo "=== feature combinations to verify: ${#COMBOS[@]} ==="

rc=0
for combo in "${COMBOS[@]}"; do
    if [ -z "$combo" ]; then
        label="<none>"
        FEATFLAGS="--no-default-features"
    else
        label="$combo"
        FEATFLAGS="--no-default-features --features $combo"
    fi
    for profile in "" "--release"; do
        pname=${profile:-dev}
        echo
        echo "############ combo=$label profile=$pname ############"
        echo "--- cargo check ---"
        if ! cargo check --offline $profile $FEATFLAGS 2>&1 | tail -3; then rc=1; fi
        echo "--- cargo build (cdylib for the differential tests) ---"
        if ! cargo build --offline $profile $FEATFLAGS 2>&1 | tail -3; then rc=1; fi
        echo "--- nm -D symbol parity ---"
        soname=$( [ -n "$profile" ] && echo target/release/libgen_ray_lib.so || echo target/debug/libgen_ray_lib.so )
        TD="${TMPDIR:-/tmp}"
        nm -D --defined-only c_src/build/libtranslated_rust.so | awk '{print $3}' | sort > "$TD/c_syms.$$"
        nm -D --defined-only "$soname"                         | awk '{print $3}' | sort > "$TD/r_syms.$$"
        if diff "$TD/c_syms.$$" "$TD/r_syms.$$" > "$TD/symdiff.$$"; then
            echo "symbols: IDENTICAL ($(wc -l < "$TD/c_syms.$$") exported)"
        else
            echo "SYMBOL DIFF (C vs RUST):"; cat "$TD/symdiff.$$"; rc=1
        fi
        rm -f "$TD/c_syms.$$" "$TD/r_syms.$$" "$TD/symdiff.$$"
        echo "--- cargo test (all differential suites) ---"
        if ! cargo test --offline $profile $FEATFLAGS -- --test-threads=4 2>&1 | grep -E "^(test result|running|error|failures:|---- )" ; then rc=1; fi
    done
done

echo
if [ $rc -eq 0 ]; then
    echo "ALL FEATURE COMBINATIONS x PROFILES: PASS"
else
    echo "FAILURES DETECTED"
fi
exit $rc
