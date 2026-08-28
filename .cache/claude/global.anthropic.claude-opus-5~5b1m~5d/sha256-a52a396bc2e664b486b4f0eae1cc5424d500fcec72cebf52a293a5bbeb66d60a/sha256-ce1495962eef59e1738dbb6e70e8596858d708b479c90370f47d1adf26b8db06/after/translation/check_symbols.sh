#!/usr/bin/env bash
# Phase A / Phase D — mechanical symbol diff between the C .so and the Rust .so.
# Exits non-zero if the Rust .so is missing any symbol the C .so exports.
set -uo pipefail
cd "$(dirname "$0")"
HTMP=${TMPDIR:-/tmp}; mkdir -p "$HTMP"

C_SO=${HATCH_C_SO:-$(find ../c_src/build -maxdepth 1 -name 'lib*.so' 2>/dev/null | head -1)}
R_SO=${HATCH_RUST_SO:-target/release/libhatch_lib.so}

[ -n "${C_SO:-}" ] && [ -f "$C_SO" ] || { echo "C .so not found; build c_src first"; exit 2; }
[ -f "$R_SO" ] || { echo "Rust .so not found ($R_SO); run: cargo build --release"; exit 2; }

echo "C    : $C_SO"
echo "Rust : $R_SO"
echo

nm -D --defined-only "$C_SO" | awk '{print $3}' | sort -u > $HTMP/.hatch_c.syms
nm -D --defined-only "$R_SO" | awk '{print $3}' | sort -u > $HTMP/.hatch_r.syms

printf 'C exports  : %s\n' "$(wc -l < $HTMP/.hatch_c.syms)"
printf 'Rust exports: %s\n' "$(wc -l < $HTMP/.hatch_r.syms)"
echo
echo "--- in C but MISSING from Rust (must be empty) ---"
missing=$(comm -23 $HTMP/.hatch_c.syms $HTMP/.hatch_r.syms)
if [ -z "$missing" ]; then echo "(empty)"; else echo "$missing"; fi
echo
echo "--- extra symbols exported only by Rust (informational) ---"
extra=$(comm -13 $HTMP/.hatch_c.syms $HTMP/.hatch_r.syms)
if [ -z "$extra" ]; then echo "(none)"; else echo "$extra"; fi
echo
echo "--- unresolved (ldd -r) ---"
for so in "$C_SO" "$R_SO"; do
  bad=$(ldd -r "$so" 2>&1 | grep -E 'undefined symbol|not found')
  printf '%s: %s\n' "$(basename "$so")" "${bad:-none}"
done

[ -z "$missing" ]
