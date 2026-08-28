#!/usr/bin/env bash
# Phase D — symbol parity. Every symbol the C .so exports must also be exported
# by the Rust .so under the exact same name. Exits non-zero if the diff is not
# empty. Also reports non-libc undefined symbols in the Rust .so.
set -uo pipefail
cd "$(dirname "$0")"

c_so=$(ls ../c_src/build/lib*.so 2>/dev/null | head -1)
if [ -z "${c_so}" ]; then
  echo "FAIL: no C .so found in ../c_src/build (build it first)"
  exit 1
fi

tmp=$(mktemp -d "${TMPDIR:-/tmp}/symdiff.XXXXXX")
trap 'rm -rf "${tmp}"' EXIT

status=0
for profile in debug release; do
  rust_so="target/${profile}/libcollided_lib.so"
  [ -f "${rust_so}" ] || { echo "SKIP ${profile}: ${rust_so} not built"; continue; }

  nm -D --defined-only "${c_so}"      | awk '{print $3}' | grep -v '^$' | sort -u > "${tmp}/sym_c"
  nm -D --defined-only "${rust_so}"   | awk '{print $3}' | grep -v '^$' | sort -u > "${tmp}/sym_r"

  missing=$(comm -23 "${tmp}/sym_c" "${tmp}/sym_r")
  echo "--- ${profile}: C exports $(wc -l < "${tmp}/sym_c"), Rust exports $(wc -l < "${tmp}/sym_r") (incl. Rust runtime symbols)"
  if [ -n "${missing}" ]; then
    echo "FAIL: symbols exported by C but MISSING from Rust (${profile}):"
    echo "${missing}" | sed 's/^/    /'
    status=1
  else
    echo "OK: 0 missing symbols (${profile})"
  fi

  # Undefined symbols that are not satisfied by the platform runtime.
  undef=$(nm -D --undefined-only "${rust_so}" | awk '{print $2}' | grep -v '^$' \
    | grep -vE '^(_|__|GCC_|GLIBC_)' \
    | grep -vE '^(abort|memcpy|memmove|memset|memcmp|bcmp|strlen|malloc|calloc|realloc|free|posix_memalign|write|writev|open|close|read|readlink|getenv|getcwd|sysconf|dl_iterate_phdr|dlsym|pthread_[a-z_]+|sigaction|sigaltstack|signal|raise|mmap|munmap|mprotect|madvise|environ|stat|fstat|lstat|poll|nanosleep|sched_yield|gettid|getpid|syscall|exit|atexit|qsort|realpath|dirfd|opendir|readdir64|closedir|statx|pipe2|fcntl|dup3|execvp|waitpid|kill|clock_gettime|uname|prctl|copy_file_range|sendfile64|epoll_[a-z]+|eventfd|socket|connect|accept4|bind|listen|recv|send|shutdown|getsockopt|setsockopt|getpeername|getsockname|freeaddrinfo|getaddrinfo|gai_strerror|isatty|ioctl|lseek64|pread64|pwrite64|ftruncate64|fsync|fdatasync|rename|renameat|unlink|unlinkat|rmdir|mkdir|mkdirat|openat|fchmod|fchown|link|linkat|symlink|symlinkat|utimensat|readdir|sigemptyset|sigaddset|sigprocmask|pthread|munlock|mlock)$' \
    || true)
  if [ -n "${undef}" ]; then
    echo "NOTE: undefined non-obvious symbols in ${profile} Rust .so (verify these are libc):"
    echo "${undef}" | sed 's/^/    /'
  fi

  # The ten C API symbols must be present by name.
  for s in c2V c2Maxv c2Minv c2Clampv c2Sub c2Dot c2CircletoCircle c2CircletoAABB c2AABBtoAABB collided; do
    grep -qx "${s}" "${tmp}/sym_r" || { echo "FAIL: ${profile} Rust .so is missing ${s}"; status=1; }
  done
done
exit $status
