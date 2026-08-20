#!/usr/bin/env bash
# Phase D — symbol parity between the C .so and the Rust .so.
#
# Usage: ./check_symbols.sh [debug|release]
# Exits non-zero if the C .so exports any dynamic symbol the Rust .so does not.
set -uo pipefail

cd "$(dirname "$0")"
PROFILE="${1:-release}"
export CARGO_NET_OFFLINE=true

CC_BIN="${CC:-cc}"
mkdir -p c_build
"$CC_BIN" -shared -fPIC -O2 -o c_build/libdriver_c.so c_src/src/main.c || exit 1

if [ "$PROFILE" = "release" ]; then
    cargo build --release --quiet || exit 1
else
    cargo build --quiet || exit 1
fi
RUST_SO="target/$PROFILE/libdriver.so"

if [ ! -f "$RUST_SO" ]; then
    echo "FAIL: $RUST_SO not built"
    exit 1
fi

tmp="$(mktemp -d "${TMPDIR:-/tmp}/symcheck.XXXXXX")"
trap 'rm -rf "$tmp"' EXIT

# defined dynamic symbols, names only
nm -D --defined-only c_build/libdriver_c.so | awk '{print $NF}' | sort -u > "$tmp/c.txt"
nm -D --defined-only "$RUST_SO"             | awk '{print $NF}' | sort -u > "$tmp/rust.txt"

echo "== C .so defined dynamic symbols ($(wc -l < "$tmp/c.txt")) =="
cat "$tmp/c.txt"
echo "== Rust .so defined dynamic symbols ($(wc -l < "$tmp/rust.txt")) =="
cat "$tmp/rust.txt"

comm -23 "$tmp/c.txt" "$tmp/rust.txt" > "$tmp/missing.txt"
comm -13 "$tmp/c.txt" "$tmp/rust.txt" > "$tmp/extra.txt"

echo "== missing from Rust .so =="
cat "$tmp/missing.txt"
echo "== extra in Rust .so (allowed) =="
cat "$tmp/extra.txt"

echo "== undefined symbols in Rust .so that do not resolve =="
UNRESOLVED="$(ldd -r "$RUST_SO" 2>&1 | grep -i "undefined symbol" || true)"
echo "${UNRESOLVED:-<none>}"

rc=0
if [ -s "$tmp/missing.txt" ]; then
    echo "FAIL: Rust .so is missing $(wc -l < "$tmp/missing.txt") C symbol(s)"
    rc=1
fi
if [ -n "$UNRESOLVED" ]; then
    echo "FAIL: unresolved symbols in the Rust .so"
    rc=1
fi
[ $rc -eq 0 ] && echo "PASS: symbol parity ($PROFILE)"
exit $rc
