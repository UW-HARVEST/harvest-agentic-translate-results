#!/usr/bin/env bash
# Phase D — symbol parity between the C .so and the Rust .so.
# Exits non-zero if the diff is not empty.
set -uo pipefail

cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"

C_SO="$(ls "$ROOT"/c_src/build/lib*.so 2>/dev/null | head -1)"
R_SO="$ROOT/translation/target/release/libarr_del_lib.so"

if [ -z "$C_SO" ] || [ ! -f "$C_SO" ]; then
    echo "FAIL: C .so not found; build it with:"
    echo "  cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
    exit 1
fi
if [ ! -f "$R_SO" ]; then
    echo "FAIL: Rust .so not found; run 'cargo build --release' in translation/"
    exit 1
fi

echo "C   : $C_SO"
echo "Rust: $R_SO"
echo

nm -D --defined-only "$C_SO" | awk '{print $3}' | sort -u > /tmp/sym_c.txt
nm -D --defined-only "$R_SO" | awk '{print $3}' | sort -u > /tmp/sym_r.txt

echo "C   exports: $(wc -l < /tmp/sym_c.txt)"
echo "Rust exports: $(wc -l < /tmp/sym_r.txt)"
echo

MISSING="$(comm -23 /tmp/sym_c.txt /tmp/sym_r.txt)"
EXTRA="$(comm -13 /tmp/sym_c.txt /tmp/sym_r.txt)"

rc=0
if [ -n "$MISSING" ]; then
    echo "FAIL: exported by C but MISSING from Rust:"
    echo "$MISSING" | sed 's/^/  /'
    rc=1
fi
if [ -n "$EXTRA" ]; then
    echo "NOTE: exported by Rust but not by C:"
    echo "$EXTRA" | sed 's/^/  /'
    rc=1
fi

# Undefined symbols in the Rust .so must all be libc / libgcc-unwind.
UNDEF="$(nm -D --undefined-only "$R_SO" | awk '{print $NF}' | sed 's/@.*//' | sort -u)"
BAD="$(echo "$UNDEF" | grep -v -E '^(_ITM_|_Unwind_|__cxa_|__gmon_start__|__tls_get_addr|__errno_location|_?_?statx|gettid)' \
       | grep -v -E '^(abort|bcmp|calloc|close|dl_iterate_phdr|free|fstat64|getcwd|getenv|lseek64|malloc|memcpy|memmove|memset|mmap64|munmap|open64|posix_memalign|pthread_key_create|pthread_key_delete|pthread_setspecific|read|readlink|realloc|realpath|sprintf|stat64|strcmp|strlen|syscall|write|writev)$' || true)"
if [ -n "$BAD" ]; then
    echo "FAIL: non-libc undefined symbols in the Rust .so:"
    echo "$BAD" | sed 's/^/  /'
    rc=1
fi

if [ $rc -eq 0 ]; then
    echo "PASS: symbol diff is EMPTY ($(wc -l < /tmp/sym_c.txt)/$(wc -l < /tmp/sym_c.txt) match), 0 non-libc undefined symbols."
fi
exit $rc
