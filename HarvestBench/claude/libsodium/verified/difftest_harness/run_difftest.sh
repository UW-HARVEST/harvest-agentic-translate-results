#!/bin/bash
# Build both libraries and run the differential test harness.
# Any symbol still undefined in the Rust .so (because a module is not translated
# yet) gets an abort()ing stub so that dlopen succeeds; the corresponding cases
# then show up as failures/aborts instead of blocking the whole run.
set -u
cd "$(dirname "$0")"
HERE=$PWD
REPO=$(cd .. && pwd)
cd "$REPO"
CBUILD=${TMPDIR:-/tmp}/cbuild

if [ ! -f "$CBUILD/libsodium.so" ]; then
    mkdir -p "$CBUILD" && (cd "$CBUILD" && cmake "$REPO/c_src" -DCMAKE_BUILD_TYPE=Release >/dev/null && make -j16 >/dev/null 2>&1)
fi

cargo build --release --offline 2>&1 | grep -E '^error' -A8 | head -40
[ -f target/release/libsodium.so ] || { echo "rust .so missing"; exit 1; }

STUBDIR=${TMPDIR:-/tmp}/dtstub
mkdir -p "$STUBDIR"
# undefined symbols in the rust .so that the C library defines => our gaps
nm -D -u target/release/libsodium.so | awk '{print $2}' | sed 's/@.*//' | sort -u > "$STUBDIR/undef.txt"
nm -D --defined-only "$CBUILD/libsodium.so" | awk '{print $3}' | sort -u > "$STUBDIR/cdef.txt"
comm -12 "$STUBDIR/undef.txt" "$STUBDIR/cdef.txt" > "$STUBDIR/gaps.txt"
NGAPS=$(wc -l < "$STUBDIR/gaps.txt")
echo "=== unresolved-in-rust symbols that C defines: $NGAPS ==="
if [ "$NGAPS" -gt 0 ]; then
    head -40 "$STUBDIR/gaps.txt" | sed 's/^/    /'
    {
      echo '#include <stdlib.h>'
      echo '#include <stdio.h>'
      while read -r s; do
        echo "long $s(void){ fprintf(stderr,\"STUB CALLED: $s\\n\"); abort(); }"
      done < "$STUBDIR/gaps.txt"
    } > "$STUBDIR/stub.c"
    gcc -shared -fPIC -o "$STUBDIR/libstub.so" "$STUBDIR/stub.c" 2>/dev/null
    PRELOAD="$STUBDIR/libstub.so"
else
    PRELOAD=""
fi

gcc -O1 -w -o "$HERE/difftest" "$HERE/difftest.c" -I"$HERE" -ldl || exit 1
if [ -n "$PRELOAD" ]; then
    LD_PRELOAD="$PRELOAD" "$HERE/difftest" "$CBUILD/libsodium.so" "$REPO/target/release/libsodium.so"
else
    "$HERE/difftest" "$CBUILD/libsodium.so" "$REPO/target/release/libsodium.so"
fi
