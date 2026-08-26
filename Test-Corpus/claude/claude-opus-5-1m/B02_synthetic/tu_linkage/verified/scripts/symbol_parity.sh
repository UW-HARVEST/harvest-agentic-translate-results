#!/usr/bin/env bash
# Phase D: every symbol exported by the C .so must also be exported by the Rust
# .so, and the Rust .so must have no undefined non-libc symbols.
set -uo pipefail
HERE="$(cd "$(dirname "$0")/.." && pwd)"
cd "$HERE"
C_SO=${1:-c_src/build/libdriver_c_full.so}
R_SO=${2:-target/release/libdriver.so}
TMP="${TMPDIR:-/tmp}/symparity.$$"
mkdir -p "$TMP"

nm -D --defined-only "$C_SO" | awk '$2=="T"{print $3}' | sort -u > "$TMP/c.txt"
nm -D --defined-only "$R_SO" | awk '$2=="T"{print $3}' | sort -u > "$TMP/r.txt"

echo "C  ($C_SO): $(wc -l < "$TMP/c.txt") exported text symbols"
echo "Rust ($R_SO): $(wc -l < "$TMP/r.txt") exported text symbols"
echo "--- symbols in C but NOT in Rust ---"
comm -23 "$TMP/c.txt" "$TMP/r.txt" | tee "$TMP/missing.txt"
echo "--- extra symbols in Rust (allowed) ---"
comm -13 "$TMP/c.txt" "$TMP/r.txt" | head -20

echo "--- undefined symbols in the Rust .so that are not libc/libgcc ---"
nm -D --undefined-only "$R_SO" | awk '{print $2}' | sed 's/@.*//' | sort -u \
  | grep -v -E '^_' \
  | grep -v -w -E 'malloc|calloc|realloc|free|posix_memalign|memcpy|memmove|memset|bcmp|strlen|strcmp|strtol|fgets|fputc|fprintf|printf|fwrite|stdin|stdout|stderr|abort|close|open64|read|write|writev|lseek64|fstat64|stat64|statx|mmap64|munmap|getcwd|getenv|gettid|readlink|realpath|syscall|dl_iterate_phdr|pthread_key_create|pthread_key_delete|pthread_setspecific|pthread_getspecific' \
  | tee "$TMP/undef.txt"

rc=0
if [ -s "$TMP/missing.txt" ]; then echo "FAIL: missing symbols"; rc=1; fi
if [ -s "$TMP/undef.txt" ]; then echo "FAIL: unexpected undefined symbols"; rc=1; fi
[ "$rc" = 0 ] && echo "symbol parity OK (0 missing, 0 unexpected undefined)"
rm -rf "$TMP"
exit $rc
