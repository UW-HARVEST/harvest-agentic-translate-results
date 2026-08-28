#!/usr/bin/env bash
# Phase A / Phase D: every dynamic symbol the C .so exports must also be
# exported by the Rust .so, with the exact same name.
set -uo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
root="$(dirname "$here")"
c_so="$root/c_src/build/libdriver.so"
rs_so="${1:-$here/target/release/libdriver.so}"

for f in "$c_so" "$rs_so"; do
    if [ ! -f "$f" ]; then
        echo "MISSING SHARED OBJECT: $f" >&2
        exit 2
    fi
done

defined() { nm -D --defined-only "$1" | awk '{print $NF}' | sort -u; }
undefined() { nm -D -u "$1" | awk '{print $NF}' | sed 's/@.*//' | sort -u; }

c_def="$(mktemp)"; rs_def="$(mktemp)"
trap 'rm -f "$c_def" "$rs_def"' EXIT
defined "$c_so"  > "$c_def"
defined "$rs_so" > "$rs_def"

echo "== C   defined ($(wc -l < "$c_def")): $(tr '\n' ' ' < "$c_def")"
echo "== RS  defined ($(wc -l < "$rs_def")): $(tr '\n' ' ' < "$rs_def")"

missing="$(comm -23 "$c_def" "$rs_def")"
extra="$(comm -13 "$c_def" "$rs_def")"

rc=0
if [ -n "$missing" ]; then
    echo "MISSING FROM RUST .so:"; echo "$missing"; rc=1
fi
if [ -n "$extra" ]; then
    echo "EXTRA IN RUST .so (informational):"; echo "$extra"
fi

# undefined non-libc / non-unwinder symbols in the Rust .so
allowed='^(_ITM_|__cxa_|__gmon_start__|_Unwind_|__tls_get_addr|__errno_location|__assert_fail|gettid|statx|gnu_get_libc_version)'
libc_funcs='^(abort|bcmp|calloc|close|dl_iterate_phdr|free|fstat64|getcwd|getenv|lseek64|malloc|memcpy|memmove|memset|mmap64|munmap|open64|posix_memalign|pthread_[a-z_]+|read|readlink|realloc|realpath|stat64|strdup|strlen|syscall|write|writev|sigaltstack|sigaction|sysconf|getrandom|__libc_start_main|memrchr|strerror_r|qsort|environ|__xpg_strerror_r)$'
bad="$(undefined "$rs_so" | grep -Ev "$allowed" | grep -Ev "$libc_funcs" || true)"
if [ -n "$bad" ]; then
    echo "UNDEFINED NON-LIBC SYMBOLS IN RUST .so:"; echo "$bad"; rc=1
fi

if [ "$rc" -eq 0 ]; then
    echo "SYMBOL PARITY: OK"
else
    echo "SYMBOL PARITY: FAILED"
fi
exit "$rc"
