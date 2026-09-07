#!/bin/bash
# Phase D: symbol parity between the C .so and the Rust .so for every config.
# Prints, per config, the symbols the C .so exports that the Rust .so does not.
set -uo pipefail
ROOT="$(cd "$(dirname "$0")" && pwd)"
syms() { nm -D --defined-only "$1" | awk '{print $3}' | sort -u; }
rc=0
for op in add sub mul; do
  for rep in 0 1 2 3 4 5 6 7; do
    c="$ROOT/cbuild/libcdriver_${op}_${rep}.so"
    r="$ROOT/rustbuild/libdriver_${op}_${rep}.so"
    missing=$(comm -23 <(syms "$c") <(syms "$r") | tr '\n' ' ')
    undef=$(nm -D -u "$r" | awk '{print $2}' | sed 's/@.*//' | grep -vE '^(__|_ITM|_Unwind|abort$|calloc$|free$|malloc$|realloc$|memcpy$|memmove$|memset$|memcmp$|bcmp$|strlen$|write$|writev$|read$|close$|open$|open64$|fstat|lseek$|lseek64$|poll$|mmap$|mmap64$|stat64$|lseek64$|open64$|fstat64$|munmap$|mprotect$|madvise$|sigaltstack$|sigaction$|sigaddset$|sigemptyset$|pthread_[a-z_]+$|dl_iterate_phdr$|dladdr$|getenv$|getcwd$|getpid$|gettid$|syscall$|sysconf$|nanosleep$|clock_gettime$|exit$|_exit$|environ$|memrchr$|printf$|puts$|fwrite$|fflush$|stdout$|stderr$|readlink$|realpath$|statx$|copy_file_range$|pipe2$|posix_memalign$|prctl$|sched_getaffinity$|sched_yield$|strerror_r$|unlink$|isatty$|signal$|raise$|getrandom$)' | tr '\n' ' ')
    if [ -n "$missing" ]; then echo "MISSING [$op/$rep]: $missing"; rc=1; fi
    if [ -n "$undef" ]; then echo "UNDEF   [$op/$rep]: $undef"; rc=1; fi
  done
done
[ $rc -eq 0 ] && echo "symbol parity OK for all 24 configs (0 missing, 0 unexpected undefined)"
exit $rc
