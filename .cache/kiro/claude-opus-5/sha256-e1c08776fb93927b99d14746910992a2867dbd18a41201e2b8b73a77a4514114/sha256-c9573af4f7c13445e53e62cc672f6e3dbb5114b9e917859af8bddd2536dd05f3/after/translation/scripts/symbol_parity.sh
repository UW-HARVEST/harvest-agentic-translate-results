#!/usr/bin/env bash
# Phase D — symbol parity between the C .so and every Rust .so.
# Exits non-zero unless the "C exports that Rust does not" diff is empty.
set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
c_so="$here/../c_src/build/libdriver.so"
status=0

if [[ ! -f "$c_so" ]]; then
    echo "missing $c_so — build the C library first" >&2
    exit 1
fi

names() { nm -D --defined-only "$1" | awk '{print $NF}' | sort -u; }

c_list="$(mktemp)"; names "$c_so" > "$c_list"
echo "C .so exports $(wc -l < "$c_list") defined dynamic symbol(s):"
sed 's/^/  /' "$c_list"

for profile in release debug; do
    r_so="$here/target/$profile/libdriver.so"
    [[ -f "$r_so" ]] || { echo "-- skipping $profile (not built)"; continue; }
    r_list="$(mktemp)"; names "$r_so" > "$r_list"
    missing="$(comm -23 "$c_list" "$r_list")"
    if [[ -n "$missing" ]]; then
        echo "FAIL: rust-$profile is missing:" >&2
        echo "$missing" | sed 's/^/  /' >&2
        status=1
    else
        echo "OK: rust-$profile exports all $(wc -l < "$c_list") C symbol(s)"
    fi
    unresolved="$(ldd -r "$r_so" 2>&1 | grep -E 'undefined symbol|not found' || true)"
    if [[ -n "$unresolved" ]]; then
        echo "FAIL: rust-$profile has unresolved imports:" >&2
        echo "$unresolved" | sed 's/^/  /' >&2
        status=1
    else
        echo "OK: rust-$profile has 0 unresolved (non-libc) imports"
    fi
    rm -f "$r_list"
done

rm -f "$c_list"
exit $status
