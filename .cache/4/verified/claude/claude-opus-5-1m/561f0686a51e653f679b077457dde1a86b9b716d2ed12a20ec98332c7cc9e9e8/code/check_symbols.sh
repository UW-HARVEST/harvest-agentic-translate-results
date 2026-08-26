#!/usr/bin/env bash
# Phase D — symbol parity between the C `.so` and the Rust `.so`.
#
# Regenerates the data behind SYMBOLS.md. Exits non-zero if any symbol exported
# by the C shared library is missing from the Rust shared library.
set -uo pipefail
cd "$(dirname "$0")"

C_SO=${DRIVER_C_SO:-c_src/build/libdriver.so}
RUST_SOS=${1:-"target/release/libdriver.so target/debug/libdriver.so"}

if [[ ! -f $C_SO ]]; then
    echo "error: $C_SO does not exist; build it with:" >&2
    echo "  (cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .)" >&2
    exit 1
fi

# Global *defined* symbols only, minus linker/runtime bookkeeping.
exported() {
    nm -D --defined-only "$1" 2>/dev/null \
        | awk '$2 ~ /^[TDBRWVGS]$/ { print $3 }' \
        | grep -Ev '^(_ITM_|__|_init$|_fini$|_edata$|_end$)' \
        | sort -u
}

rc=0
c_syms=$(exported "$C_SO")
echo "=== C .so ($C_SO) exports ==="
echo "$c_syms"

for so in $RUST_SOS; do
    if [[ ! -f $so ]]; then
        echo "--- skipping $so (not built) ---"
        continue
    fi
    r_syms=$(exported "$so")
    echo
    echo "=== Rust .so ($so) exports ==="
    echo "$r_syms"

    missing=$(comm -23 <(echo "$c_syms") <(echo "$r_syms"))
    extra=$(comm -13 <(echo "$c_syms") <(echo "$r_syms"))

    echo
    if [[ -n $missing ]]; then
        echo "FAIL: exported by C but MISSING from $so:"
        echo "$missing" | sed 's/^/  /'
        rc=1
    else
        echo "OK: every C-exported symbol is present in $so"
    fi
    if [[ -n $extra ]]; then
        echo "FAIL: exported by $so but not by the C .so:"
        echo "$extra" | sed 's/^/  /'
        rc=1
    else
        echo "OK: $so exports no extra API symbols"
    fi

    # `inner` is `static` in driver.c and must not be visible anywhere.
    if nm -D "$so" | grep -q 'inner'; then
        echo "FAIL: $so leaks an 'inner'-like symbol (it is static in the C)"
        rc=1
    fi

    # Both libc imports the C relies on must also be imported by the Rust .so.
    for want in printf memcpy; do
        if ! nm -D --undefined-only "$so" | grep -q "\\b$want@"; then
            echo "FAIL: $so does not import libc '$want'"
            rc=1
        fi
    done

    # All relocations must resolve (RTLD_NOW equivalent).
    if ! LD_BIND_NOW=1 ldd -r "$so" 2>&1 | grep -Eq 'undefined symbol'; then
        echo "OK: $so has no undefined non-libc symbols"
    else
        echo "FAIL: $so has undefined symbols:"
        ldd -r "$so" 2>&1 | grep 'undefined symbol' | sed 's/^/  /'
        rc=1
    fi
done

echo
if [[ $rc -eq 0 ]]; then
    echo "SYMBOL PARITY: PASS (diff is empty)"
else
    echo "SYMBOL PARITY: FAIL"
fi
exit $rc
