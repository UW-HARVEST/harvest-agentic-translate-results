#!/usr/bin/env bash
# Full verification run: enumerate every build configuration, type-check each
# one, (re)build the C and Rust shared objects, compare exported symbols and run
# the FFI differential test suite.
#
# `cargo test` does NOT rebuild a `cdylib`-only lib target, so `cargo build`
# must run first for every configuration — otherwise the tests would load a
# stale .so. The harness also asserts this (see tests/common/mod.rs).
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(dirname "$here")"
cd "$here"

TIMEOUT="${TIMEOUT:-600}"

# --------------------------------------------------------------------------
# 1. Enumerate feature combinations from Cargo.toml
# --------------------------------------------------------------------------
mapfile -t FEATURES < <(python3 - "$here/Cargo.toml" <<'PY'
import sys, re
text = open(sys.argv[1]).read()
# crude but sufficient: grab the [features] table
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', text, re.M | re.S)
names = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#', 1)[0].strip()
        if not line or '=' not in line:
            continue
        names.append(line.split('=', 1)[0].strip().strip('"'))
for n in names:
    if n != 'default':
        print(n)
PY
)

n=${#FEATURES[@]}
echo "== features declared in Cargo.toml: ${n} ${FEATURES[*]:-(none)}"

COMBOS=()
if [[ $n -eq 0 ]]; then
    COMBOS=("")
else
    total=$((1 << n))
    for ((mask = 0; mask < total; mask++)); do
        combo=()
        for ((b = 0; b < n; b++)); do
            if (((mask >> b) & 1)); then combo+=("${FEATURES[b]}"); fi
        done
        COMBOS+=("$(
            IFS=,
            echo "${combo[*]}"
        )")
    done
fi
echo "== ${#COMBOS[@]} feature combination(s) to verify"

# --------------------------------------------------------------------------
# 2. Build the C shared library
# --------------------------------------------------------------------------
echo "== building the C shared library"
(
    cd "$root/c_src"
    mkdir -p build
    cd build
    cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/tmp/harvest_cmake.log 2>&1
    cmake --build . >>/tmp/harvest_cmake.log 2>&1
) || {
    tail -30 /tmp/harvest_cmake.log
    exit 1
}
C_SO="$(find "$root/c_src/build" -maxdepth 1 -name 'lib*.so' | head -1)"
echo "   C .so: $C_SO"

# --------------------------------------------------------------------------
# 3. cargo check / build / test for every combination
# --------------------------------------------------------------------------
status=0
for combo in "${COMBOS[@]}"; do
    if [[ -z "$combo" ]]; then
        label="<no features>"
        args=(--no-default-features)
    else
        label="$combo"
        args=(--no-default-features --features "$combo")
    fi

    echo
    echo "======================================================================"
    echo "== configuration: $label"
    echo "======================================================================"

    echo "-- cargo check"
    if ! timeout "$TIMEOUT" cargo check "${args[@]}" >/tmp/harvest_check.log 2>&1; then
        tail -40 /tmp/harvest_check.log
        status=1
        continue
    fi

    echo "-- cargo build (required: cargo test does not rebuild a cdylib)"
    if ! timeout "$TIMEOUT" cargo build "${args[@]}" >/tmp/harvest_build.log 2>&1; then
        tail -40 /tmp/harvest_build.log
        status=1
        continue
    fi

    R_SO="$here/target/debug/libsh_geti_lib.so"
    echo "-- nm -D symbol parity"
    missing="$(comm -23 \
        <(nm -D --defined-only "$C_SO" | awk 'NF>=2{print $NF}' | sort -u) \
        <(nm -D --defined-only "$R_SO" | awk 'NF>=2{print $NF}' | sort -u))"
    if [[ -n "$missing" ]]; then
        echo "   MISSING from the Rust .so:"
        echo "$missing" | sed 's/^/     /'
        status=1
    else
        echo "   ok — every C symbol is exported by the Rust .so"
    fi

    echo "-- cargo test"
    if ! timeout "$TIMEOUT" cargo test "${args[@]}" -- --test-threads=1 \
        >/tmp/harvest_test.log 2>&1; then
        grep -E '^(test |error|thread|assertion|failures)' /tmp/harvest_test.log | head -60
        status=1
    else
        grep -E '^test result:' /tmp/harvest_test.log | sed 's/^/   /'
    fi

    echo "-- cargo build --release (panic=abort profile) + symbol parity"
    if ! timeout "$TIMEOUT" cargo build --release "${args[@]}" \
        >/tmp/harvest_release.log 2>&1; then
        tail -40 /tmp/harvest_release.log
        status=1
    else
        missing="$(comm -23 \
            <(nm -D --defined-only "$C_SO" | awk 'NF>=2{print $NF}' | sort -u) \
            <(nm -D --defined-only "$here/target/release/libsh_geti_lib.so" |
                awk 'NF>=2{print $NF}' | sort -u))"
        if [[ -n "$missing" ]]; then
            echo "   MISSING from the release Rust .so:"
            echo "$missing" | sed 's/^/     /'
            status=1
        else
            echo "   ok"
        fi
    fi
done

echo
if [[ $status -eq 0 ]]; then
    echo "ALL CONFIGURATIONS PASS"
else
    echo "FAILURES — see output above"
fi
exit $status
