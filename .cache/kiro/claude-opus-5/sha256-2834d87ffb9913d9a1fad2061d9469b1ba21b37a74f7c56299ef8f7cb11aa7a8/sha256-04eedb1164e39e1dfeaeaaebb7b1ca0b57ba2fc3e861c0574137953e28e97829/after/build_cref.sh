#!/usr/bin/env bash
# Build the C reference: a shared library (mdcore.c only, matching the C ABI
# surface of the Rust cdylib) and the full `driver` executable, for every
# OP x REPEAT configuration. Nothing under c_src/ is modified.
set -eu
here="$(cd "$(dirname "$0")" && pwd)"
CSRC="$here/c_src/src"
OUT=/tmp/cref
rm -rf "$OUT"
for OP in add sub mul; do
  for R in 0 1 2 3 4 5 6 7; do
    d="$OUT/${OP}_${R}"
    mkdir -p "$d"
    gcc -shared -fPIC -DOP="$OP" -DREPEAT="$R" \
        -o "$d/libmdcore.so" "$CSRC/mdcore.c"
    gcc -fPIC -DOP="$OP" -DREPEAT="$R" \
        -o "$d/driver" "$CSRC/mdcore.c" "$CSRC/mdmain.c"
  done
done
echo "built $(find "$OUT" -name 'libmdcore.so' | wc -l) shared libs and drivers"
