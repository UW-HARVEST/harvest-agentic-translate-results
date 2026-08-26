#!/usr/bin/env bash
# Phase D: every dynamic symbol the C shared object defines must also be defined
# by the Rust shared object, under the exact same name.
#
# Usage: ./symbol_parity.sh [debug|release]
set -u
cd "$(dirname "$0")"
PROFILE="${1:-debug}"

C_SO=build_c/libcdriver.so
R_SO="target/${PROFILE}/libdriver.so"

if [ ! -f "$C_SO" ]; then
  mkdir -p build_c
  gcc -shared -fPIC -fno-strict-aliasing -O0 c_src/src/main.c -o "$C_SO" || exit 1
fi
if [ ! -f "$R_SO" ]; then
  echo "missing $R_SO (run cargo build${PROFILE:+ --$PROFILE})" >&2
  exit 1
fi

syms() { nm -D --defined-only "$1" | awk '{print $NF}' | sort -u; }

echo "=== C   $C_SO ==="; syms "$C_SO"
echo "=== Rust $R_SO ==="; syms "$R_SO"

missing=$(comm -23 <(syms "$C_SO") <(syms "$R_SO"))
extra=$(comm -13 <(syms "$C_SO") <(syms "$R_SO"))

echo "=== missing from Rust .so ==="; echo "${missing:-<none>}"
echo "=== extra in Rust .so ===";    echo "${extra:-<none>}"

rc=0
[ -n "$missing" ] && { echo "FAIL: Rust .so is missing symbols the C .so exports"; rc=1; }
[ -n "$extra" ]   && { echo "WARN: Rust .so exports extra symbols"; rc=1; }

# no undefined non-libc symbols in the Rust .so
undef=$(nm -D --undefined-only "$R_SO" | awk '{print $NF}' \
        | grep -vE '@GLIBC|@GCC|^_ITM_|^_Unwind_|^__gmon_start__$' || true)
echo "=== undefined non-libc symbols in Rust .so ==="; echo "${undef:-<none>}"
[ -n "$undef" ] && { echo "FAIL: unresolved non-libc symbols"; rc=1; }

[ "$rc" -eq 0 ] && echo "SYMBOL PARITY: OK"
exit $rc
