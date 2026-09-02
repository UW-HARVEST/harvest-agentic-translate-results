#!/bin/bash
# Regenerate the cached C reference outputs used by
# translation/tests/long_exec_diff.rs.
#
# Each `long_exec` call is 2000 * 100 * 262144 ~= 5.2e10 kernel applications,
# about 470 s of CPU with the -O0 build that c_src/CMakeLists.txt produces, so
# the reference is generated once, in parallel, out of process, and cached.
#
# Everything here reads the C shared object only; c_src is never modified.
set -eu
here="$(cd "$(dirname "$0")" && pwd)"
C="$here/../c_src/build/liblong.so"
ref="$here/../translation/tests/reference"
mkdir -p "$ref"

SEEDS="0 1 2 3 7 42 5 100 777 12345 31337 65535 123456789 999983 2000000000 4000000000 2147483648 4294967295"
SEEDS_HASHED="4 6 8 9 10 11 13 17 19 23 29 97 128 255 256 1000 4096 54321 88888888 1000003 16777216 2147483647 3000000000 4294967294"

for s in $SEEDS; do
    ( "$here/driver" "$C" "$s" "$ref/c.exec.$s.bin" > "$ref/c.exec.$s.out" ) &
done
# For these the fixture is the exact stdout plus an FNV-1a fingerprint of the
# 1 MiB image, to keep tens of MiB of binary fixtures out of the crate.
for s in $SEEDS_HASHED; do
    ( "$here/runner" "$C" "exec:$s" hash > "$ref/c.exec.$s.raw" ) &
done

# Composite / state-carry-over rows (CONFIGS.md rows 32-34).
( "$here/runner" "$C" exec:42 pxo:1 dump:"$ref/c.row32.bin" > "$ref/c.row32.out" ) &
( "$here/runner" "$C" exec:42 exec:42 exec:7 dump:"$ref/c.row33.bin" > "$ref/c.row33.out" ) &
( "$here/runner" "$C" fill:rand:99 pxo:1 exec:42 dump:"$ref/c.row34.bin" > "$ref/c.row34.out" ) &

wait

# split the hashed-seed captures into <seed>.out (the library's own printf) and
# <seed>.hash (the fingerprint the runner appended)
for s in $SEEDS_HASHED; do
    head -n -1 "$ref/c.exec.$s.raw" > "$ref/c.exec.$s.out"
    tail -n 1  "$ref/c.exec.$s.raw" > "$ref/c.exec.$s.hash"
    rm -f "$ref/c.exec.$s.raw"
done
# and a fingerprint alongside every full dump, so the two formats cross-check
for s in $SEEDS; do
    "$here/fnv" "$ref/c.exec.$s.bin" > "$ref/c.exec.$s.hash"
done

echo "reference regenerated in $ref"
