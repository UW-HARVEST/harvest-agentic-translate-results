#!/usr/bin/env bash
# Phase D: `nm -D` symbol parity between the C .so and the Rust .so.
# Exits non-zero if the C library exports anything the Rust library does not.
set -uo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
root="$(dirname "$here")"

c_so="$(ls "$root"/../c_src/build/lib*.so 2>/dev/null | head -n1)"
if [ -z "${c_so:-}" ]; then
  echo "C .so not found; build it with:"
  echo "  cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
  exit 1
fi

boilerplate='^(_init|_fini|__bss_start|_edata|_end|_ITM_registerTMCloneTable|_ITM_deregisterTMCloneTable|__gmon_start__|rust_eh_personality)$'

syms() {
  nm -D --defined-only "$1" | awk '{print $3}' | grep -Ev "$boilerplate" | sort -u
}

status=0
for profile in debug release; do
  rust_so="$root/target/$profile/libdequantize_granule_lib.so"
  [ -f "$rust_so" ] || { echo "== $profile: not built, skipping"; continue; }

  echo "== profile: $profile"
  echo "   C   : $c_so"
  echo "   Rust: $rust_so"

  missing="$(comm -23 <(syms "$c_so") <(syms "$rust_so"))"
  extra="$(comm -13 <(syms "$c_so") <(syms "$rust_so"))"

  if [ -n "$missing" ]; then
    echo "   MISSING from Rust .so:"; echo "$missing" | sed 's/^/     /'
    status=1
  else
    echo "   missing from Rust .so: (none)"
  fi
  if [ -n "$extra" ]; then
    echo "   extra in Rust .so    :"; echo "$extra" | sed 's/^/     /'
  else
    echo "   extra in Rust .so    : (none)"
  fi

  # `get_bits` is `static` in C and must not be exported by Rust either.
  if syms "$rust_so" | grep -q get_bits; then
    echo "   ERROR: Rust .so exports get_bits (internal linkage in C)"
    status=1
  fi

  # No undefined non-libc symbols.
  und="$(nm -D -u "$rust_so" | awk '{print $2}' | awk -F'@' '{print $1}' \
        | grep -Ev '^(_Unwind_|stat64|fstat64|lstat64|realpath|__|_ITM_|memcpy|memset|memmove|memcmp|malloc|free|realloc|calloc|abort|exit|write|writev|open|close|read|dl_|pthread_|sysconf|getenv|strlen|bcmp|posix_memalign|mmap|munmap|mprotect|syscall|gettimeofday|clock_gettime|sigaction|sigaltstack|sigemptyset|signal|raise|fwrite|fputs|fflush|stderr|stdout|environ|getauxval|qsort|rand|srand|nanosleep|sched_yield|madvise|statx|readlink|poll|dup|fcntl|pipe2|prctl|unlink|access|getcwd|chdir|rename|mkdir|rmdir|opendir|readdir|closedir|lseek|ftruncate|fsync|utimensat|copy_file_range|sendfile|pread|pwrite|socket|connect|bind|listen|accept|send|recv|shutdown|setsockopt|getsockopt|getpid|gettid|kill|waitpid|fork|execvp|dup2|_exit|atexit|cxa_|Unwind_|rust_)' || true)"
  if [ -n "$und" ]; then
    echo "   undefined non-libc symbols:"; echo "$und" | sed 's/^/     /'
  else
    echo "   undefined non-libc symbols: (none)"
  fi
  echo
done

exit $status
