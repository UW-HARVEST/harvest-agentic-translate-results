#!/usr/bin/env bash
# Phase A/D: compare the exported dynamic symbols of the C .so and the Rust .so.
# Exits non-zero if the Rust library is missing any symbol the C library exports.
set -euo pipefail
cd "$(dirname "$0")"

C_SO=c_src/build/libdriver.so
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

nm -D --defined-only "$C_SO" | awk '{print $3}' | sort -u > "$tmp/c.txt"

rc=0
for RUST_SO in target/debug/libdriver.so target/release/libdriver.so; do
  [ -f "$RUST_SO" ] || { echo "SKIP $RUST_SO (not built)"; continue; }
  nm -D --defined-only "$RUST_SO" | awk '{print $3}' | sort -u > "$tmp/r.txt"

  echo "=== $C_SO  vs  $RUST_SO ==="
  echo "C exports        : $(wc -l < "$tmp/c.txt")"
  echo "Rust exports     : $(wc -l < "$tmp/r.txt")"
  missing=$(comm -23 "$tmp/c.txt" "$tmp/r.txt" || true)
  if [ -n "$missing" ]; then
    echo "MISSING from Rust:"; echo "$missing"; rc=1
  else
    echo "MISSING from Rust: (none)"
  fi

  # Undefined symbols in Rust that are not libc / libgcc-unwind imports.
  nm -D --undefined-only "$RUST_SO" | awk '{print $2}' | sed 's/@.*//' \
    | grep -vE '^(_ITM_|__cxa_|__gmon_start__|_Unwind_|__tls_get_addr|__errno_location)' \
    | grep -vE '^(abort|bcmp|calloc|close|dl_iterate_phdr|free|fstat64|getcwd|getenv|gettid|lseek64|malloc|memcpy|memmove|memset|mmap64|munmap|open64|posix_memalign|pthread_[a-z_]+|read|readlink|realloc|realpath|stat64|statx|strlen|strrchr|syscall|write|writev)$' \
    | sort -u > "$tmp/undef.txt" || true
  if [ -s "$tmp/undef.txt" ]; then
    echo "UNDEFINED non-libc symbols in Rust (untranslated code!):"; cat "$tmp/undef.txt"; rc=1
  else
    echo "UNDEFINED non-libc symbols in Rust: (none)"
  fi
  echo
done
exit $rc
