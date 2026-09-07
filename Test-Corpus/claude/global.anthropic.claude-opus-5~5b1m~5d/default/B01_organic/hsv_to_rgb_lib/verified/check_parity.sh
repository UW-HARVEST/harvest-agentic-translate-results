#!/usr/bin/env bash
# Phase D: symbol parity between the C .so and the Rust .so.
set -uo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
C_SO=$(ls "$ROOT"/c_src/build/*.so | head -1)
R_SO="$ROOT/translation/target/ffi-harness/default/release/libhsv_to_rgb_lib.so"

[ -f "$R_SO" ] || { echo "rust .so missing: $R_SO"; exit 1; }

echo "C   : $C_SO"
echo "Rust: $R_SO"
echo
echo "=== C defined symbols ==="
nm -D --defined-only "$C_SO" | awk '{print $2, $3}' | sort | tee /dev/stderr >/dev/null
nm -D --defined-only "$C_SO" | awk '{print $3}' | sort > "${TMPDIR:-/tmp}/c.syms"
echo "=== Rust defined symbols (filtered to C's namespace) ==="
nm -D --defined-only "$R_SO" | awk '{print $3}' | sort > "${TMPDIR:-/tmp}/r.syms"
cat "${TMPDIR:-/tmp}/r.syms"
echo
echo "=== MISSING from Rust (must be empty) ==="
comm -23 "${TMPDIR:-/tmp}/c.syms" "${TMPDIR:-/tmp}/r.syms" | tee "${TMPDIR:-/tmp}/missing.syms"
MISSING=$(wc -l < "${TMPDIR:-/tmp}/missing.syms")
echo
echo "=== Unresolved symbols in Rust .so (ldd -r; must be empty) ==="
# Authoritative check: instead of pattern-matching names, ask the dynamic
# linker to resolve EVERY import. Anything left over is a genuinely missing
# implementation; glibc/libgcc runtime imports (_Unwind_*, getenv, statx, ...)
# resolve fine and are pulled in by the Rust std runtime, not by untranslated C.
ldd -r "$R_SO" 2>&1 | grep -E 'undefined symbol|not found' | tee "${TMPDIR:-/tmp}/undef.syms"
UNDEF=$(wc -l < "${TMPDIR:-/tmp}/undef.syms")
echo
echo "=== C .so imports (for reference) ==="
nm -D --undefined-only "$C_SO" | awk '{print $NF}'
echo
echo "RESULT: missing=$MISSING undefined_non_libc=$UNDEF"
[ "$MISSING" -eq 0 ] && [ "$UNDEF" -eq 0 ] && echo "SYMBOL PARITY: PASS" || { echo "SYMBOL PARITY: FAIL"; exit 1; }
