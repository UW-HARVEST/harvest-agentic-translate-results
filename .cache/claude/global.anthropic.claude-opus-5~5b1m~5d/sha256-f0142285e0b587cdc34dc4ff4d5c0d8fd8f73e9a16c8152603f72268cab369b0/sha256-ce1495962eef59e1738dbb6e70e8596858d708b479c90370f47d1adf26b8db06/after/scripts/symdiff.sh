#!/usr/bin/env bash
# Mechanically diff dynamic symbols between the C .so and the Rust .so.
set -uo pipefail
TD="${TMPDIR:-.}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CSO="$(find "$ROOT/c_src/build" -maxdepth 1 -name 'lib*.so' | head -1)"
RSO="${1:-$ROOT/translation/target/release/libldexp_q2_lib.so}"
[ -f "$CSO" ] || { echo "C .so not found"; exit 1; }
[ -f "$RSO" ] || { echo "Rust .so not found: $RSO"; exit 1; }
echo "C  .so: $CSO"
echo "Rust.so: $RSO"
norm() { nm -D --defined-only "$1" | awk '{print $3}' | grep -v '^$' | sort -u; }
norm "$CSO" > ${TD}/c.syms.$$ ; norm "$RSO" > ${TD}/r.syms.$$
echo "--- C defined dynamic symbols ($(wc -l < ${TD}/c.syms.$$)) ---"; cat ${TD}/c.syms.$$
echo "--- Rust defined dynamic symbols ($(wc -l < ${TD}/r.syms.$$)) ---"; cat ${TD}/r.syms.$$
echo "--- MISSING from Rust (in C, not in Rust) ---"
comm -23 ${TD}/c.syms.$$ ${TD}/r.syms.$$ | tee ${TD}/missing.$$
MISSING=$(wc -l < ${TD}/missing.$$)
echo "--- UNDEFINED non-libc symbols in Rust .so ---"
nm -D -u "$RSO" | awk '{print $2}' | grep -v '^$' \
  | grep -v -E '@GLIBC|^_ITM_|^__gmon_start__|^__cxa_|^_Unwind_|^__tls_get_addr' | tee ${TD}/undef.$$
UNDEF=$(wc -l < ${TD}/undef.$$)
echo "==== MISSING=$MISSING UNDEFINED_NONLIBC=$UNDEF ===="
rm -f ${TD}/c.syms.$$ ${TD}/r.syms.$$ ${TD}/missing.$$ ${TD}/undef.$$
[ "$MISSING" -eq 0 ] && [ "$UNDEF" -eq 0 ] && echo "SYMBOL PARITY: PASS" || { echo "SYMBOL PARITY: FAIL"; exit 1; }
