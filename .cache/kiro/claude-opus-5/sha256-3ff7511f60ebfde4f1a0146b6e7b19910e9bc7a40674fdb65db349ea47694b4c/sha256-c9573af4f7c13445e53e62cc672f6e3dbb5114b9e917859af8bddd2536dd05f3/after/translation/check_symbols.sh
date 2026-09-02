#!/usr/bin/env bash
# Phase D — symbol parity check, run standalone (also enforced by
# tests/phase_d_symbols.rs).
set -uo pipefail
cd "$(dirname "$0")"

C_SO=$(ls ../c_src/build/*.so 2>/dev/null | head -1)
RS_SO=${1:-target/release/libpow43_lib.so}

if [[ -z "$C_SO" || ! -f "$C_SO" ]]; then
    echo "FAIL: no C .so found in ../c_src/build/" >&2
    exit 1
fi
if [[ ! -f "$RS_SO" ]]; then
    echo "FAIL: Rust .so not found at $RS_SO" >&2
    exit 1
fi

echo "C  : $C_SO"
echo "RS : $RS_SO"

defined() { nm -D "$1" | awk '$2 ~ /^[TtDBRWVi]$/ {sub(/@.*/, "", $3); print $3}' | sort -u; }

C_DEF=$(defined "$C_SO")
RS_DEF=$(defined "$RS_SO")

echo "--- C defined symbols ---";  echo "$C_DEF"
echo "--- Rust defined symbols ---"; echo "$RS_DEF"

MISSING=$(comm -23 <(echo "$C_DEF") <(echo "$RS_DEF"))
if [[ -n "$MISSING" ]]; then
    echo "FAIL: symbols in C .so missing from Rust .so:" >&2
    echo "$MISSING" >&2
    exit 1
fi
echo "OK: 0 missing symbols."

# Undefined symbols in the Rust .so that are not libc / language-runtime imports.
UNEXPECTED=$(nm -D "$RS_SO" \
  | awk '$1 == "U" {sub(/@.*/, "", $2); print $2}' \
  | grep -vE '^(_Unwind_|__|pthread_|_ITM_)' \
  | grep -vxE 'abort|bcmp|calloc|close|dl_iterate_phdr|free|fstat|fstat64|getcwd|getenv|gettid|lseek|lseek64|malloc|memcmp|memcpy|memmove|memset|mmap|mmap64|munmap|open|open64|posix_memalign|read|readlink|realloc|realpath|stat|stat64|statx|strlen|syscall|write|writev|sysconf|getauxval|dlsym|dladdr|qsort' \
  || true)
if [[ -n "$UNEXPECTED" ]]; then
    echo "FAIL: Rust .so has undefined non-libc symbols:" >&2
    echo "$UNEXPECTED" >&2
    exit 1
fi
echo "OK: 0 undefined non-libc symbols."
