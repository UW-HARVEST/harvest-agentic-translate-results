#!/usr/bin/env bash
# Phase D symbol-parity gate: every symbol exported by the C .so must also be
# exported by the Rust .so, and the Rust .so must have no undefined non-libc
# symbols. Exits non-zero if either diff is non-empty.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
crate="$(dirname "$here")"
root="$(dirname "$crate")"

c_so="$(find "$root/c_src/build" -maxdepth 1 -name '*.so' | sort | head -n1)"
rs_so="${HEX2BIN_RUST_SO:-$crate/target/release/libhex2bin_lib.so}"
test -f "$c_so"  || { echo "missing C .so; run cmake first"; exit 2; }
test -f "$rs_so" || { echo "missing Rust .so; run cargo build --release"; exit 2; }

echo "C  : $c_so"
echo "Rust: $rs_so"

# Linker/loader housekeeping that is not part of the C API surface.
noise='^(_init|_fini|_edata|_end|__bss_start|__.*_impl|.*@.*)$'

nm -D --defined-only "$c_so"  | awk '{print $NF}' | grep -Ev "$noise" | sort -u > /tmp/c_syms.txt
nm -D --defined-only "$rs_so" | awk '{print $NF}' | grep -Ev "$noise" | sort -u > /tmp/rs_syms.txt

echo
echo "--- C exports ($(wc -l < /tmp/c_syms.txt)) ---"
cat /tmp/c_syms.txt
echo "--- Rust exports ($(wc -l < /tmp/rs_syms.txt)) ---"
cat /tmp/rs_syms.txt

missing="$(comm -23 /tmp/c_syms.txt /tmp/rs_syms.txt)"
echo
if [ -n "$missing" ]; then
  echo "MISSING FROM RUST .so:"
  echo "$missing"
  exit 1
fi
echo "missing-from-Rust: NONE"

# Undefined non-libc symbols in the Rust .so.
undef="$(nm -D --undefined-only "$rs_so" | awk '{print $NF}' | sed 's/@@.*//;s/@.*//' | sort -u)"
allow='^(__cxa_finalize|__gmon_start__|_ITM_deregisterTMCloneTable|_ITM_registerTMCloneTable|__tls_get_addr|abort|memcpy|memmove|memset|memcmp|bcmp|strlen|malloc|free|realloc|calloc|__errno_location|dl_iterate_phdr|_Unwind_.*|pthread_.*|write|writev|getenv|sysconf|mmap|munmap|mprotect|open|close|read|poll|__libc_start_main|__stack_chk_fail|signal|sigaction|sigaltstack|syscall|gettid|getpid|nanosleep|sched_yield|memrchr|strchr|__assert_fail|posix_memalign|_exit|exit|raise|__register_atfork|statx|readlink|getcwd|qsort|bsearch|__cxa_thread_atexit_impl|fstat64|lseek64|mmap64|open64|realpath|stat64|fstat|lstat|stat|lseek|mmap|getrandom|clock_gettime|pipe2|dup2|fcntl|ioctl|unlink|mkdir|rmdir|rename|opendir|readdir64|closedir|strerror_r|snprintf|vsnprintf|memchr)$'
bad="$(echo "$undef" | grep -Ev "$allow" || true)"
echo
if [ -n "$bad" ]; then
  echo "UNDEFINED non-libc symbols in Rust .so:"
  echo "$bad"
  exit 1
fi
echo "undefined non-libc symbols in Rust .so: NONE"
echo "SYMBOL PARITY: PASS"
