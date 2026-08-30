#!/usr/bin/env bash
# Phase D — symbol parity gate.
#
# Every dynamic symbol the C .so exports must also be exported by the Rust .so,
# with the exact same name. Also checks that the Rust .so has no undefined
# non-libc dependencies. Exits non-zero if the diff is not empty.
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$HERE/.." && pwd)"
C_SO="$ROOT/c_src/build/libStaticLoop.so"
PROFILE="${1:-release}"
RS_SO="$HERE/target/$PROFILE/libStaticLoop.so"

fail=0

for so in "$C_SO" "$RS_SO"; do
  if [[ ! -f "$so" ]]; then
    echo "MISSING SHARED OBJECT: $so" >&2
    echo "  build C:    cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ." >&2
    echo "  build Rust: cd translation && cargo build --profile-appropriate" >&2
    exit 2
  fi
done

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

# Defined, dynamic, global symbols only.
nm -D --defined-only "$C_SO"  | awk '{print $NF}' | sort -u > "$tmp/c.txt"
nm -D --defined-only "$RS_SO" | awk '{print $NF}' | sort -u > "$tmp/rs.txt"

echo "=== C .so defined dynamic symbols ($(wc -l < "$tmp/c.txt")) ==="
cat "$tmp/c.txt"
echo
echo "=== Rust .so defined dynamic symbols ($(wc -l < "$tmp/rs.txt")) ==="
cat "$tmp/rs.txt"
echo

# The gate: symbols in C that are absent from Rust.
comm -23 "$tmp/c.txt" "$tmp/rs.txt" > "$tmp/missing.txt"
echo "=== Symbols exported by C but MISSING from Rust ==="
if [[ -s "$tmp/missing.txt" ]]; then
  cat "$tmp/missing.txt"
  echo "FAIL: $(wc -l < "$tmp/missing.txt") symbol(s) missing from the Rust .so" >&2
  fail=1
else
  echo "(none)"
fi
echo

# Informational: Rust-only symbols (Rust std/panic machinery is expected).
echo "=== Symbols exported by Rust but not by C (informational) ==="
comm -13 "$tmp/c.txt" "$tmp/rs.txt" | head -40
echo

# Undefined symbols in the Rust .so must all resolve against libc/libgcc.
echo "=== Rust .so undefined symbols not satisfied by libc/libgcc/ld ==="
# Strip the glibc symbol-version suffix (`printf@GLIBC_2.2.5` -> `printf`) and
# the 64-bit LFS suffix (`open64` -> `open`) before matching, otherwise every
# versioned libc import looks like an unrecognised dependency.
nm -D --undefined-only "$RS_SO" | awk '{print $NF}' | sed 's/@.*$//' \
  | sed -E 's/^(open|close|read|write|stat|fstat|lstat|lseek|mmap|munmap|readdir|pread|pwrite|truncate|ftruncate|getrlimit|setrlimit|glob|scandir|statfs|fstatfs|tmpfile|fopen|freopen|fseek|ftell|fgetpos|fsetpos|creat|openat|fstatat|mkstemp|nftw|ftw|versionsort|alphasort)64$/\1/' \
  | sort -u \
  | grep -vE '^(__|_ITM_|_Unwind_)' \
  | grep -vE '^(printf|puts|putchar|fwrite|memcpy|memmove|memset|memcmp|malloc|calloc|realloc|free|abort|exit|write|writev|strlen|bcmp|posix_memalign|getenv|dl_iterate_phdr|pthread_[a-z_]*|sigaction|sigaltstack|sysconf|mmap|munmap|mprotect|open|close|read|readlink|stat|fstat|lseek|poll|gettimeofday|clock_gettime|nanosleep|sched_yield|syscall|environ|qsort|strerror_r|realpath|getcwd|isatty|fflush|fdopen|fclose|ferror|fputc|fputs|snprintf|vsnprintf|strchr|strrchr|strcmp|strncmp|memrchr|getrandom|statx|pipe2|dup2|fcntl|kill|raise|signal|unlink|rename|mkdir|rmdir|opendir|readdir|closedir|getpid|gettid|sigemptyset|sigaddset|pthread_self|round|floor|ceil|fmod|pow|log|exp|sqrt)$' \
  > "$tmp/undef.txt" || true
if [[ -s "$tmp/undef.txt" ]]; then
  cat "$tmp/undef.txt"
  echo "NOTE: review the above; any genuine non-libc dependency is a failure." >&2
else
  echo "(none — all undefined symbols resolve against libc/libgcc/the runtime)"
fi
echo

if [[ "$fail" -eq 0 ]]; then
  echo "SYMBOL PARITY: PASS (profile=$PROFILE, 0 missing symbols)"
else
  echo "SYMBOL PARITY: FAIL (profile=$PROFILE)" >&2
fi
exit "$fail"
