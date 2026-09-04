#!/bin/bash
# Build the difftest harness against both libraries, run both, diff the output.
set -u
W=$HARVEST_WORKDIR
mkdir -p "$W/_dt"
cp -f "$W/translation/target/release/libsodium.so" "$W/_dt/libsodium.so" || exit 1

gcc -O1 -std=c99 -w -I"$W/c_src/libsodium/include" -o "$W/_dt/dt_c" "$W/tools/difftest.c" \
    -L"$W/_cbuild" -lsodium -Wl,-rpath,"$W/_cbuild" || exit 1
gcc -O1 -std=c99 -w -I"$W/c_src/libsodium/include" -o "$W/_dt/dt_r" "$W/tools/difftest.c" \
    -L"$W/_dt" -lsodium -Wl,-rpath,"$W/_dt" || exit 1

( cd "$W/_dt" && timeout 600 ./dt_c > out_c.txt 2>&1; echo "C   rc=$?" )
( cd "$W/_dt" && timeout 600 ./dt_r > out_r.txt 2>&1; echo "RS  rc=$?" )

if diff -u "$W/_dt/out_c.txt" "$W/_dt/out_r.txt" > "$W/_dt/diff.txt"; then
    echo "IDENTICAL ($(wc -l < "$W/_dt/out_c.txt") lines)"
else
    echo "DIFFERENCES: $(grep -c '^[-+]' "$W/_dt/diff.txt") changed lines"
    head -120 "$W/_dt/diff.txt"
fi
