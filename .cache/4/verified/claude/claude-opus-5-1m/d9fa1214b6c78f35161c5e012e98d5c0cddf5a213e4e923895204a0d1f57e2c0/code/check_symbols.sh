#!/usr/bin/env bash
# Phase D — symbol parity between the C shared object and the Rust cdylib.
#
# Builds both shared objects and diffs their exported (dynamic, defined) symbol
# sets.  Exits non-zero if the C `.so` exports anything the Rust `.so` does not.
set -uo pipefail
cd "$(dirname "$0")"

PROFILE_DIR=${PROFILE_DIR:-target/debug}
OUT=$PROFILE_DIR/c_ref
mkdir -p "$OUT"

CC=${CC:-cc}
"$CC" -shared -fPIC -o "$OUT/libdriver_c.so" c_src/src/main.c || exit 1
cargo build --offline ${CARGO_EXTRA:-} >/dev/null || exit 1

C_SO=$OUT/libdriver_c.so
R_SO=$PROFILE_DIR/libdriver.so

syms() { nm -D --defined-only "$1" | awk '{print $NF}' | sort -u; }

syms "$C_SO" > "$OUT/c.syms"
syms "$R_SO" > "$OUT/rust.syms"

echo "=== C .so ($C_SO) exported symbols ==="
cat "$OUT/c.syms"
echo
echo "=== Rust .so ($R_SO) exported symbols (crate-specific ones) ==="
grep -vE '^(_|rust_eh_personality$)' "$OUT/rust.syms" || true
echo
echo "=== C symbols MISSING from the Rust .so ==="
MISSING=$(comm -23 "$OUT/c.syms" "$OUT/rust.syms")
if [ -n "$MISSING" ]; then
  echo "$MISSING"
  echo "FAIL: the Rust .so does not export every C symbol."
  exit 1
fi
echo "(none)"
echo
echo "=== Undefined non-libc symbols in the Rust .so ==="
UND=$(nm -D --undefined-only "$R_SO" | awk '{print $NF}' | sed 's/@.*//' | sort -u |
      grep -vE '^(__|_ITM_|_Unwind_|abort$|calloc$|free$|malloc$|realloc$|memcpy$|memmove$|memset$|memcmp$|bcmp$|strlen$|write$|writev$|read$|close$|open$|open64$|fstat$|fstat64$|realpath$|lseek$|lseek64$|poll$|pipe2?$|dup2?$|dup3$|fcntl$|ioctl$|mmap$|mmap64$|munmap$|mprotect$|sigaction$|sigaltstack$|sigemptyset$|sigaddset$|signal$|getenv$|environ$|exit$|_exit$|posix_memalign$|pthread_[a-z_]*$|sysconf$|gettid$|getpid$|syscall$|nanosleep$|clock_gettime$|sched_yield$|prctl$|readlink$|stat64?$|statx$|getcwd$|chdir$|access$|unlink$|rename$|mkdir$|rmdir$|opendir$|readdir64?$|closedir$|isatty$|printf$|fflush$|fwrite$|fputs$|puts$|snprintf$|vsnprintf$|strerror_r$|dl_iterate_phdr$|dlsym$|memrchr$|getrandom$|copy_file_range$|sendfile64?$|utimensat$|futimens$|linkat$|symlinkat$|renameat$|fchmod$|fchown$|truncate64?$|ftruncate64?$|posix_spawn.*$|execvp$|waitpid$|kill$|raise$|madvise$|eventfd.*$|epoll.*$|socket.*$|connect$|bind$|listen$|accept.*$|recv.*$|send.*$|shutdown$|getsockopt$|setsockopt$|getaddrinfo$|freeaddrinfo$|gai_strerror$|gethostname$|if_.*$|fork$|pread64$|pwrite64$|preadv.*$|pwritev.*$|readv$|fdopendir$|openat$|unlinkat$|mkdirat$|fstatat64?$|faccessat$|readlinkat$|sigprocmask$|pthread$)' || true)
if [ -n "$UND" ]; then
  echo "$UND"
else
  echo "(none)"
fi
echo
echo "PASS: every C symbol is exported by the Rust .so."
