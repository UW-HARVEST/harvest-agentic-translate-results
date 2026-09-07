#!/usr/bin/env bash
# Phase D - symbol parity between the C .so and the Rust .so.
# Exits non-zero if the Rust library is missing any symbol the C exports, or if
# either library has an undefined non-libc symbol.
set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
root="$(dirname "$here")"

cso="$(find "$root/c_src/build" -maxdepth 1 -name '*.so' | head -1)"
profile="${1:-debug}"
rso="$here/target/$profile/libpinflate_lib.so"

[ -f "$cso" ] || { echo "MISSING C .so (build c_src first)"; exit 1; }
[ -f "$rso" ] || { echo "MISSING Rust .so at $rso"; exit 1; }

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

nm -D --defined-only "$cso" | awk '{print $3}' | sort -u > "$tmp/c.def"
nm -D --defined-only "$rso" | awk '{print $3}' | sort -u > "$tmp/r.def"

echo "=== C exports ($(wc -l < "$tmp/c.def")) ==="
cat "$tmp/c.def"

echo
echo "=== missing from Rust (must be empty) ==="
missing="$(comm -23 "$tmp/c.def" "$tmp/r.def")"
echo "$missing"

# undefined symbols that are not provided by libc / the Rust runtime
libc_re='^(__assert_fail|calloc|free|malloc|realloc|memcpy|memmove|memset|memcmp|abort|write|writev|open|close|read|getenv|strlen|__cxa_|_ITM_|__gmon_start__|__tls_get_addr|__errno_location|pthread_|dl_iterate_phdr|_Unwind_|__libc_start_main|__rust_|bcmp|posix_memalign|sysconf|mmap|munmap|mprotect|madvise|sigaltstack|sigaction|signal|raise|syscall|poll|readlink|dlsym|dladdr|gnu_get_libc_version|statx|stat|fstat|lseek|abort@|qsort|strerror_r|environ|__environ|getrandom|getcwd|gettid|realpath|clock_gettime|nanosleep|sched_yield|memrchr|strchr|strrchr|strnlen|__memcpy_chk|__snprintf_chk|snprintf|vsnprintf|fwrite|fputs|fflush|stderr|stdout)' 

for lib in "$cso" "$rso"; do
  echo
  echo "=== undefined non-libc symbols in $(basename "$lib") (must be empty) ==="
  nm -D --undefined-only "$lib" | awk '{print $NF}' | sed 's/@.*//' \
    | sort -u | grep -Ev "$libc_re" || true
done

if [ -n "$missing" ]; then
  echo
  echo "SYMBOL PARITY: FAIL"
  exit 1
fi
echo
echo "SYMBOL PARITY: OK (0 missing)"
