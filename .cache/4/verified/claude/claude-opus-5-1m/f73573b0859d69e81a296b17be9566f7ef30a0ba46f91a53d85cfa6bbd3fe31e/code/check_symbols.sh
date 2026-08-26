#!/usr/bin/env bash
# Phase A / Phase D: every dynamic symbol the C .so exports must also be
# exported by the Rust .so, with the exact same name.
#
# Usage: ./check_symbols.sh [cargo feature list]
set -u

cd "$(dirname "$0")" || exit 1

C_SO=c_src/build/libtranslated_rust.so
RS_SO=target/debug/libmemchra2_lib.so

if [ ! -f "$C_SO" ]; then
    echo "building C library ..."
    (mkdir -p c_src/build && cd c_src/build &&
        cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null &&
        cmake --build . >/dev/null) || exit 1
fi

FEATURES="${1:-}"
if [ -n "$FEATURES" ]; then
    cargo build --offline --no-default-features --features "$FEATURES" >/dev/null 2>&1 ||
        { echo "cargo build failed"; exit 1; }
else
    cargo build --offline --no-default-features >/dev/null 2>&1 ||
        { echo "cargo build failed"; exit 1; }
fi

WORK="target/symcheck"
mkdir -p "$WORK"
SYM_C="$WORK/sym_c.txt"
SYM_R="$WORK/sym_r.txt"

nm -D --defined-only "$C_SO" | awk '{print $3}' | sort -u > "$SYM_C"
nm -D --defined-only "$RS_SO" | awk '{print $3}' | sort -u > "$SYM_R"

echo "== C  .so exports (${C_SO}) =="
cat "$SYM_C"
echo "== Rust .so exports (${RS_SO}, features='${FEATURES}') =="
cat "$SYM_R"

echo "== missing from Rust (C exports not exported by Rust) =="
MISSING=$(comm -23 "$SYM_C" "$SYM_R")
if [ -z "$MISSING" ]; then
    echo "(empty)"
else
    echo "$MISSING"
fi

echo "== unresolved non-libc symbols in the Rust .so =="
UNDEF=$(nm -D --undefined-only "$RS_SO" | awk '{print $2}' |
    grep -vE '^(GLIBC|GCC)' | sed 's/@.*//' |
    grep -vE '^(_ITM_|__cxa_|__gmon_|__errno_location|__tls_get_addr|_Unwind_)' |
    grep -vxE 'abort|bcmp|calloc|close|dl_iterate_phdr|free|fstat|fstat64|getcwd|getenv|gettid|lseek|lseek64|malloc|memcmp|memcpy|memmove|memset|mmap|mmap64|munmap|open|open64|posix_memalign|pthread_key_create|pthread_key_delete|pthread_getspecific|pthread_setspecific|pthread_mutex_lock|pthread_mutex_unlock|pthread_mutex_trylock|pthread_rwlock_rdlock|pthread_rwlock_unlock|read|readlink|realloc|realpath|snprintf|stat|stat64|statx|strlen|strncmp|syscall|sysconf|write|writev|memrchr|__libc_start_main|environ|_edata|_end|__bss_start|_fini|_init')
if [ -z "$UNDEF" ]; then
    echo "(none)"
else
    echo "$UNDEF"
fi

[ -z "$MISSING" ] || exit 1
[ -z "$UNDEF" ] || exit 1
echo "SYMBOL PARITY OK"
