#!/usr/bin/env bash
# Phase D — symbol parity. Compares `nm -D` on the C and Rust shared objects and
# fails if the Rust `.so` is missing anything the C `.so` exports.
set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(dirname "$here")"

c_so="$(ls "$root"/c_src/build/lib*.so 2>/dev/null | head -n1)"
if [[ -z "${c_so:-}" ]]; then
  echo "FAIL: no C .so under $root/c_src/build — build it first:" >&2
  echo "  cd c_src && mkdir -p build && cd build && \\" >&2
  echo "    cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ." >&2
  exit 1
fi

rust_so=""
for profile in release debug; do
  cand="$here/target/$profile/libunderhanded_c_nuke_lib.so"
  [[ -f "$cand" ]] && { rust_so="$cand"; break; }
done
if [[ -z "$rust_so" ]]; then
  echo "FAIL: Rust .so not built — run: (cd translation && cargo build --release)" >&2
  exit 1
fi

echo "C    : $c_so"
echo "Rust : $rust_so"
echo

# Defined, non-local dynamic symbols. Filter out data/toolchain-internal entries.
defined() {
  nm -D --defined-only "$1" | awk '$2 == "T" || $2 == "W" || $2 == "D" { print $3 }' | sort -u
}

c_syms="$(defined "$c_so")"
rust_syms="$(defined "$rust_so")"

echo "== C exported symbols =="
echo "$c_syms"
echo
echo "== Rust exported symbols =="
echo "$rust_syms"
echo

missing="$(comm -23 <(echo "$c_syms") <(echo "$rust_syms"))"
echo "== Missing from Rust (must be empty) =="
if [[ -n "$missing" ]]; then
  echo "$missing"
  echo "FAIL: Rust .so is missing $(echo "$missing" | wc -l) symbol(s)."
  exit 1
fi
echo "(none)"
echo

# Undefined symbols in the Rust .so must all be libc / libgcc_s.
echo "== Rust undefined symbols that are NOT libc/libgcc (must be empty) =="
bad="$(nm -D -u "$rust_so" | awk '{ print $NF }' | sed 's/@.*//' | sort -u \
  | grep -vE '^(_ITM_|_Unwind_|__cxa_|__gmon_start__|__errno_location|__tls_get_addr|__libc_)' \
  | grep -vxE 'abort|bcmp|calloc|close|dl_iterate_phdr|free|fstat64|getcwd|getenv|gettid|lseek64|malloc|memcpy|memmove|memset|mmap64|munmap|open64|posix_memalign|pthread_key_create|pthread_key_delete|pthread_setspecific|read|readlink|realloc|realpath|stat64|statx|strlen|syscall|write|writev' || true)"
if [[ -n "$bad" ]]; then
  echo "$bad"
  echo "FAIL: unexpected undefined symbols."
  exit 1
fi
echo "(none)"
echo
echo "PASS: symbol parity — 0 missing, 0 unexpected undefined."
