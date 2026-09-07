#!/usr/bin/env bash
# Phase D driver: rebuild both libraries, enumerate every feature combination
# declared in Cargo.toml, and run the full differential suite under each one and
# under both cargo profiles. Also re-checks the nm -D symbol diff.
set -uo pipefail

cd "$(dirname "$0")"
ROOT=$(cd .. && pwd)
FAIL=0
: "${TMPDIR:=/tmp}"
TMP=$(mktemp -d "$TMPDIR/difftest.XXXXXX")
trap 'rm -rf "$TMP"' EXIT

echo "=== 1. Building the C shared library (ground truth, cmake default = -O0) ==="
( cd "$ROOT/c_src" && mkdir -p build && cd build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null ) || { echo "C BUILD FAILED"; exit 1; }
C_SO=$(ls "$ROOT"/c_src/build/lib*.so | head -1)
echo "    $C_SO"

echo
echo "=== 2. Enumerating feature combinations from Cargo.toml ==="
# Every feature name declared under [features], excluding the implicit "default".
FEATURES=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/ {split($0,a,"="); gsub(/[ \t]/,"",a[1]); if(a[1]!="default") print a[1]}' Cargo.toml)
if [ -z "$FEATURES" ]; then
    echo "    Cargo.toml declares no [features] table."
    echo "    => the only combinations that exist are: <default> and --no-default-features"
    COMBOS=("" "--no-default-features")
else
    # Full power set of the declared features, with and without defaults.
    mapfile -t NAMES <<< "$FEATURES"
    N=${#NAMES[@]}
    COMBOS=("")
    for ((m=0; m<(1<<N); m++)); do
        sel=""
        for ((k=0; k<N; k++)); do
            (( m & (1<<k) )) && sel="$sel,${NAMES[k]}"
        done
        COMBOS+=("--no-default-features --features ${sel#,}")
    done
    printf '    features: %s\n' "$FEATURES"
fi

echo
echo "=== 3. Running the differential suite for every combination x profile ==="
for combo in "${COMBOS[@]}"; do
    for profile in "--release" ""; do
        label="cargo test ${profile:-<debug>} ${combo:-<default features>}"
        echo "--- $label"
        # cargo test does not build cdylib artifacts, so build the .so first.
        # shellcheck disable=SC2086
        timeout 600 cargo build --offline --lib $profile $combo >/dev/null 2>&1
        # shellcheck disable=SC2086
        timeout 600 cargo test --offline $profile $combo > "$TMP/rt" 2>&1
        grep -E 'test result|^error' "$TMP/rt"
        if grep -qE 'FAILED|error\[|^error' "$TMP/rt"; then
            echo "    *** FAILED: $label"
            sed -n '/failures:/,/^$/p' "$TMP/rt" | head -20
            FAIL=1
        fi
    done
done

echo
echo "=== 4. nm -D symbol diff (C .so vs Rust .so) ==="
for profile in release debug; do
    RS_SO="target/$profile/libcollided_lib.so"
    [ -f "$RS_SO" ] || continue
    diff <(nm -D --defined-only "$C_SO"        | awk '{print $3}' | sort) \
         <(nm -D --defined-only "$RS_SO" | grep ' T ' | awk '{print $3}' | sort) \
         > "$TMP/symdiff" 2>&1
    if [ -s "$TMP/symdiff" ]; then
        echo "    *** SYMBOL DIFF NOT EMPTY for $profile:"
        cat "$TMP/symdiff"
        FAIL=1
    else
        echo "    $profile: symbol sets identical (10 symbols)"
    fi
    rm -f "$TMP/symdiff"
done

echo
echo "=== 5. Binary executables ==="
if grep -q 'add_executable' "$ROOT/c_src/CMakeLists.txt" 2>/dev/null; then
    echo "    *** c_src declares an executable — stdout comparison required!"
    FAIL=1
elif grep -q '\[\[bin\]\]' Cargo.toml || [ -f src/main.rs ]; then
    echo "    *** the Rust crate declares a binary the C does not have!"
    FAIL=1
else
    echo "    neither project builds a driver binary (no add_executable, no [[bin]],"
    echo "    no src/main.rs) => stdout comparison is not applicable."
fi

echo
if [ "$FAIL" -eq 0 ]; then
    echo "=== ALL PHASES PASSED ==="
else
    echo "=== FAILURES DETECTED ==="
fi
exit "$FAIL"
