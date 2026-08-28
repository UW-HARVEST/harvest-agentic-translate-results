#!/bin/bash
# Phase D — symbol parity: every dynamic symbol the C .so exports must also be
# exported by the Rust .so, with the exact same name. Exits non-zero if the diff
# is not empty.
set -u
cd "$(dirname "$0")/.." || exit 1
ROOT=..

C_SO=$(find "$ROOT/c_src/build" -maxdepth 1 -name 'lib*.so' -type f 2>/dev/null | sort | head -1)
if [ -z "$C_SO" ]; then
  echo "FAIL: no C .so; build it with:"
  echo "  cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
  exit 1
fi

cargo build --release >/dev/null 2>&1 || { echo "FAIL: cargo build --release"; exit 1; }
R_SO=$(find target/release -maxdepth 1 -name 'lib*.so' -type f | sort | head -1)
if [ -z "$R_SO" ]; then echo "FAIL: no Rust .so in target/release"; exit 1; fi

echo "C   : $C_SO"
echo "Rust: $R_SO"
echo

c_syms=$(nm -D --defined-only "$C_SO" | awk '{print $NF}' | sort -u)
r_syms=$(nm -D --defined-only "$R_SO" | awk '{print $NF}' | sort -u)

echo "== C exported symbols =="; printf '%s\n' "$c_syms" | sed 's/^/  /'
echo
echo "== missing from Rust .so =="
missing=$(comm -23 <(printf '%s\n' "$c_syms") <(printf '%s\n' "$r_syms"))
if [ -n "$missing" ]; then printf '%s\n' "$missing" | sed 's/^/  /'; else echo "  (none)"; fi
echo
echo "== extra in Rust .so (allowed: Rust runtime/CRT) =="
comm -13 <(printf '%s\n' "$c_syms") <(printf '%s\n' "$r_syms") | sed 's/^/  /' | head -20

echo
echo "== undefined (imported) non-libc symbols in Rust .so =="
# Everything Rust's libstd pulls in from glibc / the unwinder is expected.
unresolved=$(nm -D -u "$R_SO" | awk '{print $NF}' \
  | grep -vE '@GLIBC|@GCC|^_ITM_|^__gmon_start__$|^__cxa_finalize$|^_Unwind_|^__tls_get_addr$|^statx$|^gettid$|^__cxa_thread_atexit_impl$')
if [ -n "$unresolved" ]; then printf '%s\n' "$unresolved" | sed 's/^/  /'; else echo "  (none)"; fi

echo
# Loadability is the real proof that every import resolves.
if ! ldd -r "$R_SO" 2>&1 | grep -q 'undefined symbol'; then
  echo "ldd -r: all imports resolve"
else
  echo "FAIL: ldd -r reports undefined symbols"; ldd -r "$R_SO" | grep 'undefined symbol'; exit 1
fi

if [ -n "$missing" ] || [ -n "$unresolved" ]; then
  echo "RESULT: FAIL"
  exit 1
fi
echo "RESULT: PASS — symbol diff is empty"
