#!/usr/bin/env bash
# Phase D — symbol-parity check.
#
# Every symbol exported by the C .so must also be exported by the Rust .so,
# with the exact same name. Exits non-zero if the diff is non-empty.
set -uo pipefail

cd "$(dirname "$0")" || exit 1
ROOT=$PWD
PROFILE=${1:-debug}

C_SO=$(find "$ROOT/c_src/build" -maxdepth 1 -name '*.so' 2>/dev/null | head -1)
R_SO="$ROOT/target/$PROFILE/libbitwriter_add_lib.so"

if [ -z "$C_SO" ] || [ ! -f "$C_SO" ]; then
    echo "FAIL: C .so not found. Build it with:"
    echo "  cd c_src && mkdir -p build && cd build &&" \
         "cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
    exit 1
fi
if [ ! -f "$R_SO" ]; then
    echo "FAIL: Rust .so not found at $R_SO. Build it with:"
    echo "  cargo build --no-default-features${PROFILE:+ --profile $PROFILE}"
    exit 1
fi

# Weak ELF/CRT housekeeping symbols the linker adds to every shared object;
# they are not part of the library's API surface.
IGNORE='^(_ITM_(de)?registerTMCloneTable|__cxa_finalize|__cxa_thread_atexit_impl|__gmon_start__|_edata|_end|__bss_start|_fini|_init)$'

exported() {
    # Global/weak *defined* dynamic symbols only.
    nm -D --defined-only "$1" 2>/dev/null \
        | awk '{ print $NF }' \
        | grep -Ev "$IGNORE" \
        | LC_ALL=C sort -u
}

C_LIST=$(exported "$C_SO")
R_LIST=$(exported "$R_SO")

echo "C    .so : $C_SO"
echo "Rust .so : $R_SO"
echo
echo "--- C exports ($(echo "$C_LIST" | grep -c .)) ---"
echo "$C_LIST"
echo "--- Rust exports ($(echo "$R_LIST" | grep -c .)) ---"
echo "$R_LIST"
echo

MISSING=$(LC_ALL=C comm -23 <(echo "$C_LIST") <(echo "$R_LIST"))
if [ -n "$MISSING" ]; then
    echo "FAIL: symbols exported by C but MISSING from Rust:"
    echo "$MISSING" | sed 's/^/  - /'
    exit 1
fi
echo "PASS: symbol diff is empty — every C export is present in the Rust .so."

# Report (but do not fail on) undefined non-libc symbols in the Rust .so.
UNDEF=$(nm -D --undefined-only "$R_SO" 2>/dev/null | awk '{ print $NF }' \
        | grep -Ev '@GLIBC|@GCC|@CXXABI' | grep -Ev "$IGNORE" | LC_ALL=C sort -u)
if [ -n "$UNDEF" ]; then
    echo
    echo "NOTE: undefined non-versioned symbols in the Rust .so:"
    echo "$UNDEF" | sed 's/^/  ? /'
else
    echo "PASS: 0 undefined non-libc symbols in the Rust .so."
fi
