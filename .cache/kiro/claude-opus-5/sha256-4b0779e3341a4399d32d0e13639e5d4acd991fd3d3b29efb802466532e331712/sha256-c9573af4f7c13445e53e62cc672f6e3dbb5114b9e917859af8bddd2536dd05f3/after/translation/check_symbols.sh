#!/usr/bin/env bash
# Phase D — symbol parity between the C .so and the Rust cdylib.
#
# Every dynamic symbol the C library exports must also be exported by the Rust
# library under the exact same name. Exits non-zero on any difference.
set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
c_dir="$here/../c_src/build"
rust_so="$here/target/release/libreverse_collide_lib.so"

c_so="$(ls -1 "$c_dir"/*.so 2>/dev/null | head -n1 || true)"
if [ -z "${c_so}" ]; then
    echo "FAIL: no C .so in $c_dir -- build it with:" >&2
    echo "  cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ." >&2
    exit 1
fi
if [ ! -f "$rust_so" ]; then
    echo "FAIL: $rust_so missing -- run 'cargo build --release'" >&2
    exit 1
fi

syms() { nm -D --defined-only "$1" | awk '$2=="T"||$2=="W"||$2=="B"||$2=="D"{print $3}' | sort -u; }

c_list="$(mktemp)"; r_list="$(mktemp)"
trap 'rm -f "$c_list" "$r_list"' EXIT
syms "$c_so"    > "$c_list"
syms "$rust_so" > "$r_list"

echo "C    .so: $c_so  ($(wc -l < "$c_list") exported symbols)"
echo "Rust .so: $rust_so  ($(wc -l < "$r_list") exported symbols)"

missing="$(comm -23 "$c_list" "$r_list")"
extra="$(comm -13 "$c_list" "$r_list")"

rc=0
if [ -n "$missing" ]; then
    echo "FAIL: exported by C but MISSING from Rust:"; echo "$missing" | sed 's/^/  - /'
    rc=1
else
    echo "OK: 0 symbols missing from the Rust .so"
fi
if [ -n "$extra" ]; then
    echo "NOTE: exported by Rust but not by C:"; echo "$extra" | sed 's/^/  + /'
else
    echo "OK: 0 extra symbols in the Rust .so"
fi

# Undefined symbols in the Rust .so must all be libc / runtime imports.
undef="$(nm -D --undefined-only "$rust_so" \
    | awk '{print $NF}' \
    | sed 's/@.*//' \
    | grep -vE '^(_ITM_|__cxa_|__gmon_start__|_Unwind_|__tls_get_addr|__errno_location)' \
    | grep -vE '^(abort|bcmp|calloc|close|dl_iterate_phdr|free|fstat64|getcwd|getenv|gettid|lseek64|malloc|memcpy|memmove|memset|mmap64|munmap|open64|posix_memalign|pthread_key_create|pthread_key_delete|pthread_setspecific|read|readlink|realloc|realpath|stat64|statx|strlen|syscall|write|writev)$' \
    || true)"
if [ -n "$undef" ]; then
    echo "FAIL: Rust .so has non-libc undefined symbols:"; echo "$undef" | sed 's/^/  ? /'
    rc=1
else
    echo "OK: all undefined symbols in the Rust .so are libc/runtime imports"
fi

exit $rc
