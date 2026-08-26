#!/usr/bin/env bash
# Runs the full differential test suite under every build configuration, and
# checks C-vs-Rust dynamic symbol parity for each resulting Rust .so.
#
# Cargo.toml declares no [features], so the feature cross-product is the single
# empty set; it is exercised under all the spellings that could differ, plus
# both profiles (dev vs release, which differ because release sets
# `panic = "abort"`).
set -u

cd "$(dirname "$0")" || exit 1

C_SO=c_src/build/libtranslated_rust.so
TMP=${TMPDIR:-/tmp}
fail=0

if [ ! -f "$C_SO" ]; then
    echo "!! C shared object missing: $C_SO"
    echo "   build it: cd c_src && mkdir -p build && cd build &&"
    echo "   cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
    exit 1
fi

# Every symbol the C .so exports must also be exported by the Rust .so.
nm -D --defined-only "$C_SO" \
    | awk '$2=="T"||$2=="D"||$2=="B"||$2=="R"{print $3}' | sort -u > "$TMP/c.syms"

echo "=== C .so exports ($(wc -l < "$TMP/c.syms")) ==="
cat "$TMP/c.syms"

# name|extra cargo flags|profile dir
CONFIGS="
B1 default||debug
B2 no-default-features|--no-default-features|debug
B3 all-features|--all-features|debug
B4 release|--release|release
B5 release+no-default-features|--release --no-default-features|release
"

echo "$CONFIGS" | while IFS='|' read -r name flags profile; do
    [ -z "${name:-}" ] && continue

    echo
    echo "################################################################"
    echo "# $name   (cargo flags: ${flags:-<none>}, profile: $profile)"
    echo "################################################################"

    # shellcheck disable=SC2086
    if ! timeout 600 cargo build $flags > "$TMP/build.$profile.log" 2>&1; then
        echo "!! BUILD FAILED"
        tail -25 "$TMP/build.$profile.log"
        fail=1
        echo "$fail" > "$TMP/failflag"
        continue
    fi

    # ---- symbol parity for this configuration -------------------------------
    r_so="target/$profile/libima_parse_lib.so"
    if [ ! -f "$r_so" ]; then
        echo "!! Rust .so missing: $r_so"
        echo 1 > "$TMP/failflag"
        continue
    fi
    nm -D --defined-only "$r_so" \
        | awk '$2=="T"||$2=="D"||$2=="B"||$2=="R"{print $3}' | sort -u > "$TMP/r.syms"
    missing=$(comm -23 "$TMP/c.syms" "$TMP/r.syms")
    if [ -n "$missing" ]; then
        echo "!! SYMBOLS MISSING FROM $r_so:"
        echo "$missing"
        echo 1 > "$TMP/failflag"
    else
        echo "symbol parity: OK (0 missing)"
    fi

    # ---- differential tests -------------------------------------------------
    # shellcheck disable=SC2086
    if timeout 600 cargo test $flags > "$TMP/test.$profile.log" 2>&1; then
        grep -E "^test result:" "$TMP/test.$profile.log"
        echo "tests: OK"
    else
        echo "!! TESTS FAILED"
        grep -E "^(test result:|---- |thread )" "$TMP/test.$profile.log" | head -40
        tail -20 "$TMP/test.$profile.log"
        echo 1 > "$TMP/failflag"
    fi
done

echo
echo "################################################################"
if [ -f "$TMP/failflag" ]; then
    rm -f "$TMP/failflag"
    echo "RESULT: FAILURES PRESENT"
    exit 1
fi
echo "RESULT: ALL CONFIGURATIONS PASSED"
