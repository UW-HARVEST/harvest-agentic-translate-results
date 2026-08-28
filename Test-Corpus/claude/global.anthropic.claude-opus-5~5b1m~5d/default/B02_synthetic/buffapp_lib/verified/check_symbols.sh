#!/usr/bin/env bash
# Phase A / Phase D: mechanical symbol-parity check.
# Every symbol the C .so DEFINES must also be DEFINED by the Rust .so.
set -u

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
C_SO="$(find "$ROOT/c_src/build" -maxdepth 1 -name 'lib*.so' | head -1)"
R_SO="$ROOT/translation/target/release/libbuffapp_lib.so"

[ -n "$C_SO" ] || { echo "C .so not found; build c_src first"; exit 1; }
[ -f "$R_SO" ] || { echo "Rust .so not found; cargo build --release first"; exit 1; }

echo "C   .so: $C_SO"
echo "Rust.so: $R_SO"
echo

# Defined symbols only (T/t/D/B/R/W), sorted.
defined() { nm -D --defined-only "$1" | awk '{print $NF}' | sed 's/@.*//' | sort -u; }

CS="${TMPDIR:-/tmp}/c_syms.$$"; RS="${TMPDIR:-/tmp}/r_syms.$$"
defined "$C_SO" > "$CS"
defined "$R_SO" > "$RS"

echo "=== C defined symbols ($(wc -l < "$CS")) ==="
cat "$CS"
echo
echo "=== Symbols DEFINED by C .so but MISSING from Rust .so ==="
MISSING="$(comm -23 "$CS" "$RS")"
if [ -z "$MISSING" ]; then
    echo "(none) -- symbol diff is EMPTY. PARITY OK."
    rc=0
else
    echo "$MISSING"
    rc=1
fi

echo
echo "=== Undefined non-libc symbols in Rust .so (informational) ==="
nm -D --undefined-only "$R_SO" | awk '{print $NF}' | sed 's/@.*//' | sort -u \
  | grep -v -E '^(malloc|realloc|calloc|free|posix_memalign|strlen|strcpy|strcmp|bcmp|memcpy|memmove|memset|sprintf|printf|abort|getenv|getcwd|readlink|realpath|open64|close|read|write|writev|lseek64|fstat64|stat64|statx|mmap64|munmap|syscall|gettid|dl_iterate_phdr|__errno_location|__tls_get_addr|__cxa_finalize|__cxa_thread_atexit_impl|__gmon_start__|_ITM_registerTMCloneTable|_ITM_deregisterTMCloneTable|pthread_key_create|pthread_key_delete|pthread_setspecific|_Unwind_.*)$' \
  || true

rm -f "$CS" "$RS"
exit $rc
