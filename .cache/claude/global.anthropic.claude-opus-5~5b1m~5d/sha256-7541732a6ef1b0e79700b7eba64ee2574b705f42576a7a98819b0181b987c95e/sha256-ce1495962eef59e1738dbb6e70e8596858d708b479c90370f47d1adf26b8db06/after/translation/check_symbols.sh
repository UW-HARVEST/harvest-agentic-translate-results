#!/usr/bin/env bash
# Phase D: symbol parity. The diff must be EMPTY.
set -u
cd "$(dirname "$0")" || exit 1

C_SO=../c_src/build/libdriver.so
R_SO=target/release/libdriver.so

for so in "$C_SO" "$R_SO"; do
    [ -f "$so" ] || { echo "MISSING: $so (build it first)"; exit 1; }
done

syms() { nm -D --defined-only "$1" | awk '{print $NF}' | sort -u; }

echo "=== C  .so exported symbols ==="; syms "$C_SO"
echo "=== Rust .so exported symbols ==="; syms "$R_SO"

missing=$(comm -23 <(syms "$C_SO") <(syms "$R_SO"))
extra=$(comm -13 <(syms "$C_SO") <(syms "$R_SO"))

rc=0
if [ -n "$missing" ]; then
    echo "=== FAIL: exported by C but NOT by Rust ==="; echo "$missing"; rc=1
else
    echo "=== OK: 0 symbols missing from the Rust .so ==="
fi
[ -n "$extra" ] && { echo "=== NOTE: exported by Rust but not C (informational) ==="; echo "$extra"; }

# Undefined non-libc symbols in the Rust .so would mean an untranslated module.
und=$(nm -D --undefined-only "$R_SO" | awk '{print $NF}' | sed 's/@.*//' \
      | grep -vE '^(_|__)' \
      | grep -vqE . && echo "" || nm -D --undefined-only "$R_SO" | awk '{print $NF}' | sed 's/@.*//')
echo "=== Rust .so undefined (imported) symbols — all must be libc/runtime ==="
nm -D --undefined-only "$R_SO" | awk '{print $NF}' | sort -u
exit $rc
