#!/usr/bin/env bash
# Phase A / Phase D symbol parity check.
# Every symbol exported by the C .so must also be exported by the Rust .so.
set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(dirname "$here")"

c_so="$(find "$root/c_src/build" -maxdepth 1 -name '*.so' | head -1)"
r_so="$here/target/release/libread_side_info_lib.so"

if [[ -z "$c_so" || ! -f "$c_so" ]]; then
    echo "C .so not found; build it with:"
    echo "  cd $root/c_src && mkdir -p build && cd build &&"
    echo "  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
    exit 1
fi
[[ -f "$r_so" ]] || { echo "Rust .so not found; run: cd $here && cargo build --release"; exit 1; }

echo "C   .so: $c_so"
echo "Rust .so: $r_so"
echo

c_syms="$(nm -D --defined-only "$c_so" | awk '{print $3}' | sort -u)"
r_syms="$(nm -D --defined-only "$r_so" | awk '{print $3}' | sort -u)"

echo "=== C exports ($(echo "$c_syms" | grep -c .)) ==="
echo "$c_syms"
echo
echo "=== Rust exports ($(echo "$r_syms" | grep -c .)) ==="
echo "$r_syms"
echo

missing="$(comm -23 <(echo "$c_syms") <(echo "$r_syms"))"
if [[ -n "$missing" ]]; then
    echo "=== MISSING from Rust .so ==="
    echo "$missing"
    exit 1
fi
echo "=== SYMBOL DIFF EMPTY: every C export is present in the Rust .so ==="

# Undefined symbols that are neither libc nor the libgcc unwinder would mean an
# unresolved reference; list anything unexpected.
echo
echo "=== Rust undefined non-libc / non-unwind symbols ==="
nm -D --undefined-only "$r_so" \
  | awk '{print $2}' \
  | grep -v '^_Unwind_' \
  | grep -v '@GLIBC' \
  | grep -v '^__cxa_finalize$\|^__gmon_start__$\|^_ITM_\|^gettid$\|^statx$\|^__cxa_thread_atexit_impl$' \
  | grep . && { echo "^^^ unexpected undefined symbols"; exit 1; }
echo "(none)"
