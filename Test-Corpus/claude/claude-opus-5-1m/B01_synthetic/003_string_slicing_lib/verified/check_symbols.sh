#!/usr/bin/env bash
# Phase D — exported-symbol parity between the C and the Rust shared object.
#
# Gate: every dynamic symbol DEFINED by the C .so must also be defined by the
# Rust .so under the exact same name.  Extra Rust-runtime symbols are allowed.
# Also verifies the Rust .so has no unresolvable (non-libc) imports.
set -u
cd "$(dirname "$0")"

C_SO=${C_SO:-c_src/build/libString_Slice.so}
PROFILE=${PROFILE:-debug}
R_SO=${R_SO:-target/$PROFILE/libString_Slice.so}

for f in "$C_SO" "$R_SO"; do
    [ -f "$f" ] || { echo "missing shared object: $f"; exit 2; }
done

defined() { nm -D --defined-only --format=posix "$1" | awk '{print $1}' | sort -u; }

tmp=$(mktemp -d "${TMPDIR:-/tmp}/symcheck.XXXXXX")
trap 'rm -rf "$tmp"' EXIT
defined "$C_SO" > "$tmp/c.txt"
defined "$R_SO" > "$tmp/r.txt"

echo "=== defined dynamic symbols ==="
echo "C   ($C_SO): $(wc -l < "$tmp/c.txt")"
sed 's/^/  C    /' "$tmp/c.txt"
echo "Rust ($R_SO): $(wc -l < "$tmp/r.txt")"

missing=$(comm -23 "$tmp/c.txt" "$tmp/r.txt")
extra=$(comm -13 "$tmp/c.txt" "$tmp/r.txt")

echo
echo "=== symbols in C but NOT in Rust (must be empty) ==="
if [ -n "$missing" ]; then printf '%s\n' "$missing" | sed 's/^/  MISSING /'; else echo "  (none)"; fi

echo
echo "=== Rust-only symbols (informational: Rust runtime artifacts) ==="
if [ -n "$extra" ]; then printf '%s\n' "$extra" | sed 's/^/  extra   /' | head -40; else echo "  (none)"; fi

echo
echo "=== unresolvable imports in the Rust .so (must be empty) ==="
unres=$(ldd -r "$R_SO" 2>&1 | grep -i 'undefined symbol' || true)
if [ -n "$unres" ]; then printf '%s\n' "$unres" | sed 's/^/  /'; else echo "  (none)"; fi

if [ -n "$missing" ] || [ -n "$unres" ]; then
    echo
    echo "SYMBOL PARITY: FAIL"
    exit 1
fi
echo
echo "SYMBOL PARITY: OK (0 missing, 0 unresolved)"
