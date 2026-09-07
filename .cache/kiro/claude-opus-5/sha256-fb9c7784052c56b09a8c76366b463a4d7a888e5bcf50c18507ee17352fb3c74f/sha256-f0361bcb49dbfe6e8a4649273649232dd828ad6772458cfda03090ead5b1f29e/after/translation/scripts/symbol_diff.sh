#!/usr/bin/env bash
# Phase D: the authoritative `nm -D` symbol diff. Exits non-zero if the Rust
# .so fails to export any symbol the C .so exports, or if the Rust .so has
# undefined symbols that are not libc/toolchain imports.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
crate="$(cd "$here/.." && pwd)"
root="$(cd "$crate/.." && pwd)"

c_so="${C_DRIVER_SO:-$root/c_src/build/libdriver.so}"
rust_so="${RUST_DRIVER_SO:-$crate/target/release/libdriver.so}"

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

# Defined dynamic symbols, excluding the toolchain-generated ones that are not
# part of the library's API surface.
strip_noise() {
    grep -vE '^(_init|_fini|__bss_start|_edata|_end|_ITM_.*|__cxa_.*|__gmon_start__)$' || true
}

nm -D --defined-only "$c_so"    | awk '{print $NF}' | sort -u | strip_noise > "$tmp/c.txt"
nm -D --defined-only "$rust_so" | awk '{print $NF}' | sort -u | strip_noise > "$tmp/rust.txt"

echo "C .so defined symbols   ($(wc -l < "$tmp/c.txt")):"
sed 's/^/  /' "$tmp/c.txt"
echo "Rust .so defined symbols ($(wc -l < "$tmp/rust.txt")):"
sed 's/^/  /' "$tmp/rust.txt"

missing="$(comm -23 "$tmp/c.txt" "$tmp/rust.txt")"
if [[ -n "$missing" ]]; then
    echo "MISSING from the Rust .so:" >&2
    echo "$missing" | sed 's/^/  /' >&2
    exit 1
fi
echo "symbol diff: EMPTY (Rust exports every C symbol)"

# Undefined symbols in the Rust .so must all be libc / language-runtime.
nm -D --undefined-only "$rust_so" | awk '{print $NF}' | sed 's/@.*//' | sort -u > "$tmp/undef.txt"
non_libc="$(grep -vE '^(_ITM_|__cxa_|__gmon_start__|_Unwind_|__tls_get_addr|__errno_location|abort|bcmp|calloc|close|dl_iterate_phdr|free|fstat64|getcwd|getenv|gettid|lseek64|malloc|memcpy|memmove|memset|mmap64|munmap|open64|posix_memalign|printf|pthread_|read|readlink|realloc|realpath|setlocale|stat64|statx|strlen|syscall|write|writev|tolower|toupper|__ctype_)' "$tmp/undef.txt" || true)"
if [[ -n "$non_libc" ]]; then
    echo "Unexpected non-libc undefined symbols in the Rust .so:" >&2
    echo "$non_libc" | sed 's/^/  /' >&2
    exit 1
fi
echo "undefined symbols: all libc/runtime ($(wc -l < "$tmp/undef.txt") entries)"
