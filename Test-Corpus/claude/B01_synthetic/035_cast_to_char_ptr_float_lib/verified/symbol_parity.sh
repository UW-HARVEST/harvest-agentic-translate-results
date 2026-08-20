#!/usr/bin/env bash
# Phase D: compare `nm -D` on the C .so and the Rust .so.
# Every symbol the C .so EXPORTS, the Rust .so must also export, exact name.
set -uo pipefail
cd "$(dirname "$0")" || exit 1

C_SO="c_src/build/libdriver.so"
R_SO="target/debug/libdriver.so"

[ -f "$C_SO" ] || { echo "missing $C_SO (build the C library first)"; exit 1; }
[ -f "$R_SO" ] || { echo "missing $R_SO (cargo build first)"; exit 1; }

# Toolchain/crt-injected glue present in every shared object; not library API.
GLUE='^(_ITM_registerTMCloneTable|_ITM_deregisterTMCloneTable|__cxa_finalize|__gmon_start__|_edata|_end|__bss_start|__cxa_thread_atexit_impl|_fini|_init)$'

exports() { nm -D --defined-only "$1" | awk '{print $NF}' | sed 's/@.*//' | sort -u | grep -Ev "$GLUE"; }

exports "$C_SO" > "${TMPDIR:-/tmp}/c_exports.txt"
exports "$R_SO" > "${TMPDIR:-/tmp}/r_exports.txt"

echo "=== C .so exported symbols ($(wc -l < "${TMPDIR:-/tmp}/c_exports.txt")) ==="
cat "${TMPDIR:-/tmp}/c_exports.txt"
echo "=== Rust .so exported symbols ($(wc -l < "${TMPDIR:-/tmp}/r_exports.txt")) ==="
cat "${TMPDIR:-/tmp}/r_exports.txt"

echo "=== MISSING from Rust .so (must be empty) ==="
MISSING=$(comm -23 "${TMPDIR:-/tmp}/c_exports.txt" "${TMPDIR:-/tmp}/r_exports.txt")
echo "${MISSING:-<none>}"

# Undefined symbols in the Rust .so that do NOT resolve against the system
# libraries (libc / libm / libgcc_s / ld.so). `ldd -r` performs the authoritative
# relocation check and prints "undefined symbol: ..." for anything unresolved.
echo "=== Rust .so undefined non-libc/non-runtime symbols (must be empty) ==="
NONLIBC=$(ldd -r "$R_SO" 2>&1 | grep -E 'undefined symbol|not found' || true)
echo "${NONLIBC:-<none>}"

echo "=== (reference) C .so undefined symbols that do not resolve ==="
ldd -r "$C_SO" 2>&1 | grep -E 'undefined symbol|not found' || echo "<none>"

echo "=== (reference) full undefined-import list of the Rust .so ==="
nm -D --undefined-only "$R_SO" | awk '{print $NF}' | sed 's/@.*//' | sort -u | tr '\n' ' '
echo

if [ -n "$MISSING" ]; then
  echo "SYMBOL PARITY: FAIL (missing exports)"
  exit 1
fi
if [ -n "$NONLIBC" ]; then
  echo "SYMBOL PARITY: FAIL (unresolved non-libc imports:$NONLIBC)"
  exit 1
fi
echo "SYMBOL PARITY: PASS (0 missing exports, 0 non-libc undefined)"
