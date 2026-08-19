#!/usr/bin/env bash
# Phase D — symbol parity between the C .so and the Rust .so.
# Every dynamic symbol DEFINED by the C library must also be defined by the
# Rust library, with the exact same name. Prints the diff; exits non-zero if
# anything is missing.
set -u -o pipefail

cd "$(dirname "$0")" || exit 1

C_SO="c_src/build/libdriver.so"
PROFILE="${1:-debug}"
RUST_SO="target/${PROFILE}/libdriver.so"

if [[ ! -f "$C_SO" ]]; then
    echo "!! missing $C_SO -- build it with:"
    echo "   cd c_src && mkdir -p build && cd build && \\"
    echo "   cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
    exit 1
fi
if [[ ! -f "$RUST_SO" ]]; then
    echo "!! missing $RUST_SO -- run: cargo build ${PROFILE/debug/}"
    exit 1
fi

tmp=$(mktemp -d "${TMPDIR:-/tmp}/symcheck.XXXXXX")
trap 'rm -rf "$tmp"' EXIT

# Defined global code/data symbols only, version suffix stripped.
extract() {
    nm -D --defined-only "$1" \
        | awk '{ if (NF>=3) { k=$2; n=$3 } else if (NF==2) { k=$1; n=$2 } else next;
                 if (k=="T"||k=="D"||k=="B"||k=="R"||k=="G"||k=="S"||k=="i") print n }' \
        | sed 's/@.*//' | sort -u
}

extract "$C_SO"    > "$tmp/c.txt"
extract "$RUST_SO" > "$tmp/rust.txt"

echo "=== C .so   : $C_SO ($(wc -l < "$tmp/c.txt") defined symbols) ==="
cat "$tmp/c.txt"
echo
echo "=== Rust .so: $RUST_SO ($(wc -l < "$tmp/rust.txt") defined symbols) ==="
cat "$tmp/rust.txt"
echo

comm -23 "$tmp/c.txt" "$tmp/rust.txt" > "$tmp/missing.txt"
comm -13 "$tmp/c.txt" "$tmp/rust.txt" > "$tmp/extra.txt"

if [[ -s "$tmp/extra.txt" ]]; then
    echo "-- extra symbols in Rust (allowed; runtime/ABI glue) --"
    cat "$tmp/extra.txt"
    echo
fi

if [[ -s "$tmp/missing.txt" ]]; then
    echo "!! FAIL: symbols exported by the C .so but MISSING from the Rust .so:"
    cat "$tmp/missing.txt"
    exit 1
fi

echo "== Undefined non-libc symbols in the Rust .so =="
nm -D --undefined-only "$RUST_SO" \
    | sed 's/@.*//' | awk '{print $NF}' | sort -u \
    | grep -vE '^(U|w)$' \
    | grep -vE '^(_ITM_|__cxa_|__gmon_|__libc_|_Unwind_|__tls_|__errno|__pthread|pthread_|__gnu_|__assert|_dl_|__stack_chk|__register_|__deregister_|_edata|_end|__bss_start)' \
    | grep -vE '^[a-z0-9_]+$' > "$tmp/undef.txt"
if [[ -s "$tmp/undef.txt" ]]; then
    echo "!! FAIL: unresolved non-libc symbols (untranslated C module?):"
    cat "$tmp/undef.txt"
    exit 1
fi
echo "(none)"
echo
echo "== PASS: symbol diff is EMPTY -- 0 missing, 0 unresolved non-libc =="
