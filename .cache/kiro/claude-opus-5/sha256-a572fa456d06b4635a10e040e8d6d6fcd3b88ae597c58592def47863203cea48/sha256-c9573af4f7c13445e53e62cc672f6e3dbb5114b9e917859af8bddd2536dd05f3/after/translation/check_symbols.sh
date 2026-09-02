#!/usr/bin/env bash
# Phase D symbol-parity gate: every symbol exported by the C .so must also be
# exported by the Rust .so, and the Rust .so must have no undefined non-libc
# symbols. Exits non-zero if the diff is not empty.
set -uo pipefail
cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"

C_SO="$(find "$ROOT/c_src/build" -maxdepth 1 -name '*.so' | sort | head -1)"
R_SO=""
for p in target/debug/libima_parse_lib.so target/release/libima_parse_lib.so; do
  [ -f "$p" ] && R_SO="$p"
done

if [ -z "${C_SO:-}" ] || [ ! -f "$C_SO" ]; then
  echo "C .so not built (cd ../c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .)" >&2
  exit 1
fi
if [ -z "$R_SO" ]; then
  echo "Rust .so not built (cargo build)" >&2
  exit 1
fi

# Drop toolchain/CRT internals (leading underscore) from the comparison.
nm -D --defined-only --format=posix "$C_SO" | awk '{print $1}' | grep -v '^_' | sort -u > /tmp/sym_c.$$
nm -D --defined-only --format=posix "$R_SO" | awk '{print $1}' | grep -v '^_' | sort -u > /tmp/sym_r.$$

echo "C   ($C_SO):"; sed 's/^/    /' /tmp/sym_c.$$
echo "RUST ($R_SO):"; sed 's/^/    /' /tmp/sym_r.$$

MISSING="$(comm -23 /tmp/sym_c.$$ /tmp/sym_r.$$)"
EXTRA="$(comm -13 /tmp/sym_c.$$ /tmp/sym_r.$$)"

# Undefined symbols in the Rust .so that are not libc / libgcc-unwind.
UNDEF="$(nm -D --undefined-only --format=posix "$R_SO" | awk '{print $1}' \
  | sed 's/@.*//' | grep -v '^_' | sort -u \
  | grep -vxE 'abort|bcmp|calloc|close|dl_iterate_phdr|free|fstat64|getcwd|getenv|gettid|lseek64|malloc|memcmp|memcpy|memmove|memset|mmap64|munmap|open64|posix_memalign|pthread_.*|read|readlink|realloc|realpath|stat64|statx|strlen|syscall|write|writev|sysconf|getpid|environ|poll|mprotect|sigaction|sigaltstack|sigaddset|sigemptyset|signal')"

rm -f /tmp/sym_c.$$ /tmp/sym_r.$$

rc=0
if [ -n "$MISSING" ]; then echo "MISSING from Rust .so:"; echo "$MISSING" | sed 's/^/    /'; rc=1; fi
if [ -n "$EXTRA" ];   then echo "EXTRA in Rust .so:";     echo "$EXTRA"   | sed 's/^/    /'; rc=1; fi
if [ -n "$UNDEF" ];   then echo "UNDEFINED non-libc in Rust .so:"; echo "$UNDEF" | sed 's/^/    /'; rc=1; fi
[ "$rc" -eq 0 ] && echo "symbol diff: EMPTY (0 missing, 0 extra, 0 undefined non-libc)"
exit "$rc"
