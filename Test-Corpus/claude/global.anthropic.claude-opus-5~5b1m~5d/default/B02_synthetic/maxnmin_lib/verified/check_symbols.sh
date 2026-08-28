#!/usr/bin/env bash
# Phase D symbol parity: every symbol exported by the C .so must also be
# exported by the Rust .so, with the exact same name. Exits non-zero on any
# missing symbol or on any non-libc undefined symbol in the Rust .so.
set -uo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
c_so="${HARVEST_C_SO:-$(ls "$here"/../c_src/build/*.so 2>/dev/null | head -n1)}"
rust_so="${HARVEST_RUST_SO:-$here/target/release/libmaxnmin_lib.so}"

if [[ ! -f "$c_so" ]]; then
    echo "FAIL: no C .so; build it with:"
    echo "  cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
    exit 1
fi
if [[ ! -f "$rust_so" ]]; then
    echo "FAIL: no Rust .so at $rust_so; run: cargo build --offline --release"
    exit 1
fi

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

nm -D --defined-only "$c_so"    | awk '$2 ~ /^[TDBRW]$/ {print $3}' | sort -u > "$tmp/c.syms"
nm -D --defined-only "$rust_so" | awk '$2 ~ /^[TDBRW]$/ {print $3}' | sort -u > "$tmp/r.syms"

echo "C    .so: $c_so  ($(wc -l < "$tmp/c.syms") exported)"
echo "Rust .so: $rust_so  ($(wc -l < "$tmp/r.syms") exported)"

missing="$(comm -23 "$tmp/c.syms" "$tmp/r.syms")"
extra="$(comm -13 "$tmp/c.syms" "$tmp/r.syms")"

if [[ -n "$missing" ]]; then
    echo "FAIL: exported by C but MISSING from Rust:"
    echo "$missing" | sed 's/^/  - /'
    exit 1
fi
echo "OK: symbol diff (C -> Rust) is empty"
[[ -n "$extra" ]] && { echo "note: Rust-only exports (harmless):"; echo "$extra" | sed 's/^/  + /'; }

# non-libc undefined symbols in the Rust .so would mean untranslated code
nm -D --undefined-only "$rust_so" | awk '{print $NF}' | sed 's/@.*//' | sort -u > "$tmp/r.undef"
grep -vE '^(_Unwind_|__gxx_)' "$tmp/r.undef" \
  | grep -vxE '_ITM_deregisterTMCloneTable|_ITM_registerTMCloneTable|__cxa_finalize|__cxa_thread_atexit_impl|__gmon_start__|__errno_location|__libc_start_main|__tls_get_addr|abort|bcmp|calloc|close|dl_iterate_phdr|environ|free|fstat64|fstatat64|getcwd|getenv|getrandom|gettid|lseek64|malloc|memcmp|memcpy|memmove|memrchr|memset|mmap64|munmap|open64|openat64|poll|posix_memalign|pthread_[a-z_]*|read|readlink|realloc|realpath|sigaction|sigaltstack|signal|stat64|statx|strlen|strncpy|syscall|sysconf|sysinfo|write|writev' \
  > "$tmp/r.bad" || true
if [[ -s "$tmp/r.bad" ]]; then
    echo "FAIL: Rust .so has non-libc undefined symbols:"
    sed 's/^/  ? /' "$tmp/r.bad"
    exit 1
fi
echo "OK: Rust .so has 0 undefined non-libc symbols"
