#!/bin/sh
# Phase D symbol parity check: every symbol exported by the C shared object must
# also be exported by the Rust cdylib, with the exact same name.
#
# Usage: ./build_c.sh && cargo build && ./check_symbols.sh
set -e
cd "$(dirname "$0")"

C_SO=cbuild/libcdriver.so
CW_SO=cbuild/libcwrap.so
R_SO=target/debug/libdriver.so

for f in "$C_SO" "$CW_SO" "$R_SO"; do
    [ -f "$f" ] || { echo "missing $f -- run ./build_c.sh && cargo build"; exit 1; }
done

tmp=$(mktemp -d "${TMPDIR:-/tmp}/symcheck.XXXXXX")
trap 'rm -rf "$tmp"' EXIT

nm -D --defined-only "$C_SO"  | awk '{print $3}' | sort -u > "$tmp/c"
nm -D --defined-only "$CW_SO" | awk '$3 ~ /^w_/ {print $3}' | sort -u > "$tmp/cw"
nm -D --defined-only "$R_SO"  | awk '{print $3}' | sort -u > "$tmp/r"

missing=$(comm -23 "$tmp/c" "$tmp/r")
missing_w=$(comm -23 "$tmp/cw" "$tmp/r")
extra=$(comm -13 "$(cat "$tmp/c" "$tmp/cw" | sort -u > "$tmp/all"; echo "$tmp/all")" "$tmp/r")

printf 'C   .so: %s symbols\n' "$(wc -l < "$tmp/c")"
printf 'C   w_*  : %s symbols\n' "$(wc -l < "$tmp/cw")"
printf 'Rust .so: %s symbols\n' "$(wc -l < "$tmp/r")"

if [ -n "$missing" ] || [ -n "$missing_w" ]; then
    echo "FAIL: symbols exported by C but not by Rust:"
    echo "$missing"
    echo "$missing_w"
    exit 1
fi

echo "OK: 0 missing symbols"
[ -n "$extra" ] && { echo "note: extra symbols only in the Rust .so:"; echo "$extra"; }

echo
echo "undefined (imported) symbols in the Rust .so that are not libc/libgcc:"
nm -D --undefined-only "$R_SO" | awk '{print $NF}' | sed 's/@.*//' |
    grep -v -E '^(_ITM_|__cxa_|__gmon_start__|__tls_get_addr|__errno_location|_Unwind_)' |
    grep -v -E '^(abort|atan2|atof|bcmp|calloc|close|cos|dl_iterate_phdr|exit|fprintf|free|fstat64|getauxval|getcwd|getenv|gettid|lseek64|malloc|memcpy|memmove|memset|mmap64|munmap|open64|pause|posix_memalign|printf|pthread_key_create|pthread_key_delete|pthread_setspecific|rand|read|readlink|realloc|realpath|sigaltstack|sin|sqrt|srand|stat64|statx|stderr|strlen|syscall|write|writev)$' ||
    echo "  (none)"
