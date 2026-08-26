#!/bin/sh
# Phase A / Phase D symbol-parity gate.
#
# Every symbol exported by the C .so must also be exported, under the exact same
# name, by the Rust cdylib.  Exits non-zero if the diff is not empty.
set -eu
cd "$(dirname "$0")"

C_SO=c_src/build/libtranslated_rust.so
R_SO=target/debug/libaabb_lib.so

if [ ! -f "$C_SO" ]; then
    echo "building C reference .so"
    (mkdir -p c_src/build && cd c_src/build \
        && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
        && cmake --build . >/dev/null)
fi
cargo build --offline >/dev/null 2>&1 || cargo build >/dev/null

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
nm -D --defined-only "$C_SO" | awk '{print $3}' | sort >"$tmp/c.syms"
nm -D --defined-only "$R_SO" | awk '{print $3}' | sort >"$tmp/r.syms"

echo "C exports   : $(wc -l <"$tmp/c.syms")"
echo "Rust exports: $(wc -l <"$tmp/r.syms")"

missing=$(comm -23 "$tmp/c.syms" "$tmp/r.syms")
extra=$(comm -13 "$tmp/c.syms" "$tmp/r.syms")

if [ -n "$missing" ]; then
    echo "MISSING from Rust .so:"
    echo "$missing"
fi
if [ -n "$extra" ]; then
    echo "EXTRA in Rust .so (informational):"
    echo "$extra"
fi

echo
echo "undefined non-libc symbols in Rust .so:"
nm -D --undefined-only "$R_SO" | awk '{print $2}' \
    | grep -vE '^(_ITM_|_Unwind_|__cxa_|__gmon_|__tls_get_addr|__errno_location)' \
    | grep -vE '@GLIBC|@GCC' || true

[ -z "$missing" ] || { echo "SYMBOL PARITY: FAIL"; exit 1; }
echo "SYMBOL PARITY: OK (0 missing)"
