#!/bin/bash
# Build the C sources as a shared library (libcdriver_<OP>_<REPEAT>.so) and as
# the driver executable (driver_<OP>_<REPEAT>) for every valid configuration.
#
# c_src/ is never modified; all outputs land in cbuild/.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"
SRC="$ROOT/c_src/src"
OUT="$ROOT/cbuild"
mkdir -p "$OUT"

for op in add sub mul; do
  for rep in 0 1 2 3 4 5 6 7; do
    tag="${op}_${rep}"
    # shared library from mdcore.c only (mdmain.c has main())
    gcc -shared -fPIC -DOP="$op" -DREPEAT="$rep" \
        -o "$OUT/libcdriver_${tag}.so" "$SRC/mdcore.c"
    # driver executable
    gcc -DOP="$op" -DREPEAT="$rep" \
        -o "$OUT/driver_${tag}" "$SRC/mdcore.c" "$SRC/mdmain.c"
  done
done
echo "built $(ls "$OUT" | wc -l) artifacts in $OUT"
