#!/usr/bin/env bash
# Phase D symbol-parity gate: every symbol the C .so exports must also be
# exported by the Rust .so, and the Rust .so must have no undefined non-libc
# symbols.
set -uo pipefail
cd "$(dirname "$0")"

C_SO=c_src/build/libtranslated_rust.so
R_SO=${1:-target/release/libflip_horizontal_lib.so}

for f in "$C_SO" "$R_SO"; do
  [ -f "$f" ] || { echo "missing $f"; exit 1; }
done

# Weak symbols the toolchain injects into every shared object; not part of the
# library's own surface.
TOOLCHAIN='^(_ITM_deregisterTMCloneTable|_ITM_registerTMCloneTable|__cxa_finalize|__gmon_start__|__cxa_thread_atexit_impl|gettid|statx)$'

defined() {
  nm -D --defined-only "$1" | awk '{print $NF}' | sed 's/@.*//' \
    | grep -Ev "$TOOLCHAIN" | sort -u
}

defined "$C_SO" > "${TMPDIR:-/tmp}/c.syms"
defined "$R_SO" > "${TMPDIR:-/tmp}/r.syms"

echo "C   exported symbols: $(wc -l < "${TMPDIR:-/tmp}/c.syms")"
cat "${TMPDIR:-/tmp}/c.syms" | sed 's/^/    /'
echo "Rust exported symbols: $(wc -l < "${TMPDIR:-/tmp}/r.syms")"
cat "${TMPDIR:-/tmp}/r.syms" | sed 's/^/    /'

missing=$(comm -23 "${TMPDIR:-/tmp}/c.syms" "${TMPDIR:-/tmp}/r.syms")
echo
if [ -n "$missing" ]; then
  echo "MISSING from Rust .so:"
  echo "$missing" | sed 's/^/    /'
  exit 1
fi
echo "OK: 0 symbols missing from the Rust .so"

undef=$(nm -D --undefined-only "$R_SO" | awk '{print $NF}' \
  | grep -Ev 'GLIBC|GCC_|__tls_get_addr|dl_iterate_phdr' | grep -Ev "$TOOLCHAIN" | sort -u)
if [ -n "$undef" ]; then
  echo "UNDEFINED non-libc symbols in Rust .so:"
  echo "$undef" | sed 's/^/    /'
  exit 1
fi
echo "OK: 0 undefined non-libc symbols in the Rust .so"
