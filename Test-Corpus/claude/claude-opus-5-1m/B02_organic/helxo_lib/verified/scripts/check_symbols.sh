#!/bin/sh
# Phase D — symbol parity between the C .so and the Rust .so.
# Every dynamic symbol the C library defines must also be defined by the Rust
# library, with the exact same name.  The diff must be EMPTY.
set -e
cd "$(dirname "$0")/.."
T=$(mktemp -d "${TMPDIR:-/tmp}/symcheck.XXXXXX")
trap 'rm -rf "$T"' EXIT
C_SO=c_src/build/libtranslated_rust.so
R_SO=${1:-target/debug/libhelxo_lib.so}

[ -f "$C_SO" ] || { echo "missing $C_SO (build it with cmake)"; exit 1; }
[ -f "$R_SO" ] || { echo "missing $R_SO (cargo build)"; exit 1; }

nm -D --defined-only "$C_SO" | awk '{print $3}' | sort -u > "$T"/c_syms
nm -D --defined-only "$R_SO" | awk '{print $3}' | sort -u > "$T"/r_syms

echo "C   defines $(wc -l < "$T"/c_syms) dynamic symbols"
echo "RUST defines $(wc -l < "$T"/r_syms) dynamic symbols"

missing=$(comm -23 "$T"/c_syms "$T"/r_syms)
extra=$(comm -13 "$T"/c_syms "$T"/r_syms)

if [ -n "$missing" ]; then
    echo "MISSING from the Rust .so:"; echo "$missing"; exit 1
fi
if [ -n "$extra" ]; then
    echo "EXTRA in the Rust .so (not exported by C):"; echo "$extra"; exit 1
fi
echo "symbol diff EMPTY -- full parity"

# and no unresolved non-libc imports
und=$(nm -D -u "$R_SO" | awk '{print $2}' | grep -v '@GLIBC\|@GCC\|^_ITM_\|^__gmon_start__\|^__cxa_finalize$\|^_ITM' || true)
if [ -n "$und" ]; then
    echo "unresolved non-libc imports in the Rust .so:"; echo "$und"; exit 1
fi
echo "no unresolved non-libc imports"
