#!/usr/bin/env bash
# Phase A / Phase D — symbol parity between the C and the Rust shared library.
#
# Fails if the C `.so` exports a symbol the Rust `.so` does not, or if the Rust
# `.so` has undefined symbols outside libc / the language runtime.
set -uo pipefail
cd "$(dirname "$0")"

C_SO="${C_SO:-c_src/build/libtranslated_rust.so}"
RUST_SO="${RUST_SO:-target/release/libstr_dups_lib.so}"

if [[ ! -f "$C_SO" ]]; then
  echo "C library missing ($C_SO); build it with:"
  echo "  cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
  exit 1
fi
if [[ ! -f "$RUST_SO" ]]; then
  echo "Rust library missing ($RUST_SO); build it with: cargo build --release"
  exit 1
fi

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

nm -D --defined-only "$C_SO"    | awk '{print $3}' | sort -u > "$tmp/c.txt"
nm -D --defined-only "$RUST_SO" | awk '{print $3}' | sort -u > "$tmp/r.txt"

missing="$(comm -23 "$tmp/c.txt" "$tmp/r.txt")"
extra="$(comm -13 "$tmp/c.txt" "$tmp/r.txt")"

echo "C   exports: $(wc -l < "$tmp/c.txt")"
echo "Rust exports: $(wc -l < "$tmp/r.txt")"

rc=0
if [[ -n "$missing" ]]; then
  echo "MISSING FROM RUST:"; echo "$missing" | sed 's/^/  /'
  rc=1
else
  echo "missing from Rust: none"
fi
if [[ -n "$extra" ]]; then
  echo "EXTRA IN RUST (informational):"; echo "$extra" | sed 's/^/  /'
fi

# Undefined symbols in the Rust .so must all be libc / language-runtime imports.
nm -D --undefined-only "$RUST_SO" | awk '{print $NF}' | sed 's/@.*//' | sort -u > "$tmp/u.txt"
allow='^(_ITM_(de)?registerTMCloneTable|_Unwind_[A-Za-z]+|__assert_fail|__cxa_finalize|__cxa_thread_atexit_impl|__errno_location|__gmon_start__|__tls_get_addr|abort|bcmp|calloc|close|dl_iterate_phdr|free|fstat(64)?|getcwd|getenv|gettid|lseek(64)?|malloc|memcmp|memcpy|memmove|memset|mmap(64)?|munmap|open(64)?|posix_memalign|printf|pthread_[a-z_]+|read|readlink|realloc|realpath|sprintf|stat(64)?|statx|strcmp|strlen|syscall|write|writev)$'
bad="$(grep -Ev "$allow" "$tmp/u.txt" || true)"
if [[ -n "$bad" ]]; then
  echo "UNEXPECTED UNDEFINED SYMBOLS IN RUST .so:"; echo "$bad" | sed 's/^/  /'
  rc=1
else
  echo "undefined non-libc symbols: none"
fi

[[ $rc == 0 ]] && echo "SYMBOL PARITY OK"
exit $rc
