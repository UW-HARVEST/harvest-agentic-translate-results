#!/usr/bin/env bash
# Phase A / Phase D: every dynamic symbol the C .so defines must also be defined
# by the Rust .so, and the Rust .so must have no unresolved non-libc symbols.
set -euo pipefail
cd "$(dirname "$0")"

PROFILE="${1:-debug}"
C_SO=c_build/libcdriver.so
R_SO="target/${PROFILE}/libdriver.so"

[ -f "$C_SO" ] || { echo "missing $C_SO (run ./build_c.sh)"; exit 1; }
[ -f "$R_SO" ] || { echo "missing $R_SO (run cargo build)"; exit 1; }

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

nm -D --defined-only "$C_SO" | awk '{print $3}' | sort > "$tmp/c.txt"
nm -D --defined-only "$R_SO" | awk '{print $3}' | sort > "$tmp/r.txt"

echo "== C .so defined symbols =="; cat "$tmp/c.txt"
echo "== Rust .so defined symbols =="; cat "$tmp/r.txt"

missing=$(comm -23 "$tmp/c.txt" "$tmp/r.txt")
extra=$(comm -13 "$tmp/c.txt" "$tmp/r.txt")

rc=0
if [ -n "$missing" ]; then
    echo "FAIL: symbols exported by the C .so but MISSING from the Rust .so:"
    echo "$missing"
    rc=1
else
    echo "OK: symbol diff (C minus Rust) is empty"
fi
if [ -n "$extra" ]; then
    echo "NOTE: extra symbols exported only by the Rust .so:"
    echo "$extra"
fi

# Unresolved symbols (everything must come from libc / libgcc_s).
if ldd -r "$R_SO" 2>&1 | grep -q "undefined symbol"; then
    echo "FAIL: unresolved symbols in $R_SO:"
    ldd -r "$R_SO" 2>&1 | grep "undefined symbol"
    rc=1
else
    echo "OK: no undefined symbols in $R_SO"
fi

exit $rc
