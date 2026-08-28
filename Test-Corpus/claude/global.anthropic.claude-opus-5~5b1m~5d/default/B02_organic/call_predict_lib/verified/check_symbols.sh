#!/usr/bin/env bash
# Phase A / Phase D — nm -D symbol parity between the C .so and the Rust .so.
# Exits non-zero if the Rust .so is missing any symbol the C .so exports, or if
# the Rust .so has undefined non-libc symbols.
set -uo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/.." && pwd)"

C_SO="${C_SO:-$(ls "$ROOT"/c_src/build/*.so 2>/dev/null | head -1)}"
RUST_SO="${RUST_SO:-$HERE/target/release/libcall_predict_lib.so}"

[ -f "$C_SO" ]    || { echo "missing C .so ($C_SO)"; exit 1; }
[ -f "$RUST_SO" ] || { echo "missing Rust .so ($RUST_SO)"; exit 1; }

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

nm -D --defined-only "$C_SO"    | awk '{print $NF}' | sort -u > "$tmp/c.txt"
nm -D --defined-only "$RUST_SO" | awk '{print $NF}' | sort -u > "$tmp/r.txt"

echo "C   exports: $(wc -l < "$tmp/c.txt")  -> $(paste -sd, "$tmp/c.txt")"
echo "Rust exports: $(wc -l < "$tmp/r.txt")  -> $(paste -sd, "$tmp/r.txt")"

missing=$(comm -23 "$tmp/c.txt" "$tmp/r.txt")
if [ -n "$missing" ]; then
  echo "MISSING FROM RUST .so:"
  echo "$missing"
  exit 1
fi
echo "symbol diff (C \\ Rust): EMPTY  ✅"

# Undefined symbols in the Rust .so that are not libc / libgcc-unwind.
undef=$(nm -D -u "$RUST_SO" | awk '{print $NF}' | sed 's/@.*//' | sort -u | grep -v -E \
  '^(_ITM_(de)?registerTMCloneTable|__cxa_finalize|__cxa_thread_atexit_impl|__gmon_start__|__errno_location|__tls_get_addr|_Unwind_[A-Za-z]+|abort|bcmp|calloc|close|dl_iterate_phdr|free|fstat64|getcwd|getenv|gettid|lseek64|malloc|memcpy|memmove|memset|mmap64|munmap|open64|posix_memalign|pthread_key_create|pthread_key_delete|pthread_setspecific|read|readlink|realloc|realpath|stat64|statx|strlen|syscall|write|writev)$' || true)
if [ -n "$undef" ]; then
  echo "UNRESOLVED NON-LIBC SYMBOLS IN RUST .so:"
  echo "$undef"
  exit 1
fi
echo "undefined non-libc symbols: NONE  ✅"
