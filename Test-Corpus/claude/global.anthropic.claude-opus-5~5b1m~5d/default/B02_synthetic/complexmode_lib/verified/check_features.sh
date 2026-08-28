#!/usr/bin/env bash
# Run the full differential suite under EVERY Cargo feature combination.
#
# The feature list is extracted from Cargo.toml rather than hard-coded, so this
# stays correct if features are ever added. With no [features] table there is
# exactly one configuration (the default), which is still exercised explicitly
# via --no-default-features and --all-features.
set -uo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
cd "$here" || exit 1
root="$(dirname "$here")"

# ---- build the C reference library ----------------------------------------
if ! ls "$root"/c_src/build/*.so >/dev/null 2>&1; then
    echo "== building C reference library =="
    ( cd "$root/c_src" && mkdir -p build && cd build \
        && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
        && cmake --build . ) || exit 1
fi

# ---- enumerate features ----------------------------------------------------
# Names on the left-hand side of `=` inside the [features] section.
features=$(awk '
    /^\[features\]/ { inf=1; next }
    /^\[/           { inf=0 }
    inf && /=/      { gsub(/[ \t]/, "", $0); split($0, a, "="); if (a[1] != "") print a[1] }
' Cargo.toml | grep -v '^default$' | sort -u)

echo "== features declared in Cargo.toml: ${features:-<none>} =="

# Build the list of flag-sets to test: powerset of the optional features, plus
# the default build and the all-features build.
combos=()
combos+=("")                                # default features
combos+=("--no-default-features")
combos+=("--all-features")
if [ -n "$features" ]; then
    mapfile -t farr <<< "$features"
    n=${#farr[@]}
    total=$((1 << n))
    for ((mask = 0; mask < total; mask++)); do
        sel=""
        for ((i = 0; i < n; i++)); do
            if (( mask & (1 << i) )); then sel="$sel,${farr[$i]}"; fi
        done
        sel="${sel#,}"
        combos+=("--no-default-features --features $sel")
    done
fi

# ---- run ------------------------------------------------------------------
fail=0
for profile in "" "--release"; do
    for combo in "${combos[@]}"; do
        label="cargo test ${profile:-<debug>} ${combo:-<default features>}"
        echo
        echo "=================================================================="
        echo "== $label"
        echo "=================================================================="
        # The tests dlopen the cdylib, so it must exist for this configuration.
        # shellcheck disable=SC2086
        if ! timeout 600 cargo build $profile $combo >/dev/null 2>&1; then
            echo "BUILD FAILED: $label"
            fail=1
            continue
        fi
        # shellcheck disable=SC2086
        if timeout 600 cargo test $profile $combo -- --test-threads=1; then
            echo "PASS: $label"
        else
            echo "FAIL: $label"
            fail=1
        fi
    done
done

echo
echo "== symbol parity =="
if ./check_symbols.sh; then :; else fail=1; fi

echo
if [ "$fail" -eq 0 ]; then
    echo "ALL FEATURE COMBINATIONS PASSED"
else
    echo "SOME CONFIGURATIONS FAILED"
fi
exit "$fail"
